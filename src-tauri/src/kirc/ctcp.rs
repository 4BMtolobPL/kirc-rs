#[derive(Debug)]
pub(super) enum CtcpCommand {
    Version,
    Ping(String),
    Time,
    Unknown(String),
}

pub(super) fn parse_ctcp(message: &str) -> Option<CtcpCommand> {
    if !message.starts_with('\x01') || !message.ends_with('\x01') {
        return None;
    }

    let inner = &message[1..message.len() - 1];
    let mut parts = inner.splitn(2, ' ');

    let cmd = parts.next()?.to_uppercase();
    let arg = parts.next().unwrap_or("").to_string();

    match cmd.as_str() {
        "VERSION" => Some(CtcpCommand::Version),
        "PING" => Some(CtcpCommand::Ping(arg)),
        "TIME" => Some(CtcpCommand::Time),
        _ => Some(CtcpCommand::Unknown(inner.to_string())),
    }
}
