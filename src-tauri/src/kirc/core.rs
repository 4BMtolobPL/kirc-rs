use crate::kirc::emits::{emit_server_status, emit_system_message, emit_ui_event};
use crate::kirc::state::{IRCClientState, ServerRuntime};
use crate::kirc::types::{ServerCommand, ServerId, ServerStatus};
use futures::prelude::*;
use irc::client::prelude::*;
use tauri::{AppHandle, Manager};
use tauri_plugin_log::log::{error, trace};

pub(super) struct ServerConfig {
    server: String,
    port: u16,
    use_tls: bool,
    nickname: String,
}

impl ServerConfig {
    pub(super) fn new(server: String, port: u16, use_tls: bool, nickname: String) -> Self {
        Self {
            server,
            port,
            use_tls,
            nickname,
        }
    }
}

pub(super) async fn server_actor(
    server_id: ServerId,
    server_config: ServerConfig,
    app_handle: AppHandle,
) {
    // actor에선 error를 ?로 전파하지 않고, 소비/로깅만 하거나 이벤트로 전파

    let config = Config {
        server: Some(server_config.server),
        port: Some(server_config.port),
        use_tls: Some(server_config.use_tls),
        nickname: Some(server_config.nickname),
        ..Config::default()
    };

    let mut client = match Client::from_config(config).await {
        Ok(c) => c,
        Err(e) => {
            fail_state(server_id, app_handle, e.to_string());
            return;
        }
    };

    if let Err(e) = client.identify() {
        fail_state(server_id, app_handle, e.to_string());
        return;
    }

    let mut stream = match client.stream() {
        Ok(s) => s,
        Err(e) => {
            fail_state(server_id, app_handle, e.to_string());
            return;
        }
    };

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    {
        let state = app_handle.state::<IRCClientState>();
        let mut servers = state.servers.lock().unwrap();
        if let Some(ServerRuntime::Connecting { handle }) = servers.remove(&server_id) {
            trace!("server_actor status connecting to registering");
            servers.insert(
                server_id,
                ServerRuntime::Registering {
                    tx: tx.clone(),
                    handle,
                },
            );
        }
    }

    let _ = emit_server_status(&app_handle, server_id, ServerStatus::Registering);

    loop {
        tokio::select! {
            Some(result) = stream.next() => {
                match result {
                    Ok(message) => {
                        let _ = handle_message(server_id, message, &app_handle);
                    }
                    Err(_) => break,
                }
            }
            Some(cmd) = rx.recv() => {
                trace!("rx recieve: {cmd}");
                match cmd {
                    ServerCommand::Join(ch) => {
                        if let Err(e) = client.send_join(&ch) {
                            error!("Failed to send join message: {e}");
                        }
                    }
                    ServerCommand::Privmsg { target, message } => {
                        if let Err(e) = client.send_privmsg(&target, &message) {
                            error!("Failed to send privmsg: {e}");
                        }

                        match Message::with_tags(None, Some(client.current_nickname()), "PRIVMSG", vec![&target, &message]) {
                                Ok(msg) => {
                                    trace!("Create echo: {:?}", msg);
                                    handle_message(server_id, msg, &app_handle).expect("Failed to handle message");
                                }
                                Err(_) => {
                                    error!("Failed to create echo message");
                                }
                            }
                    }
                    ServerCommand::Quit => {
                        if let Err(e) = client.send_quit("bye") {
                            error!("Failed to send quit message: {e}");
                        }
                        break;
                    }
                }
            }
        }
    }

    {
        let state = app_handle.state::<IRCClientState>();
        let mut servers = state.servers.lock().unwrap();
        servers.insert(server_id, ServerRuntime::Disconnected);
    }
    let _ = emit_server_status(&app_handle, server_id, ServerStatus::Disconnected);
}

fn fail_state(server_id: ServerId, app_handle: AppHandle, message: String) {
    let state = app_handle.state::<IRCClientState>();
    let mut servers = state.servers.lock().unwrap();

    servers.insert(server_id, ServerRuntime::Failed { error: message });

    let _ = emit_server_status(&app_handle, server_id, ServerStatus::Failed);
}

fn handle_message(
    server_id: ServerId,
    message: Message,
    app_handle: &AppHandle,
) -> anyhow::Result<()> {
    let source_nickname = message.source_nickname().unwrap_or_else(|| "").to_string();

    match message.command {
        Command::PRIVMSG(target, content) => {
            trace!("PRIVMSG | from: {source_nickname}, target: {target}, content: {content}");
            emit_ui_event(app_handle)
                .user_message(server_id, target, source_nickname, content)
                .emit()?;
        }
        Command::JOIN(chanlist, _chankey, _real_name) => {
            emit_ui_event(app_handle)
                .join(server_id, chanlist, source_nickname)
                .emit()?;
        }
        Command::PART(chanlist, comment) => {
            emit_ui_event(app_handle)
                .part(server_id, chanlist, source_nickname, comment)
                .emit()?;
        }
        Command::QUIT(comment) => {
            emit_ui_event(app_handle)
                .quit(server_id, source_nickname, comment)
                .emit()?;
        }
        Command::NICK(nickname) => {
            emit_ui_event(app_handle)
                .nick(server_id, source_nickname, nickname)
                .emit()?;
        }
        Command::TOPIC(channel, topic) => {
            emit_ui_event(app_handle)
                .topic(server_id, channel, topic)
                .emit()?;
        }
        Command::ERROR(message) => {
            emit_ui_event(app_handle).error(server_id, message).emit()?;
        }
        Command::Response(Response::RPL_WELCOME, _) => {
            trace!("handle_message RPL_WELCOME");
            {
                let state = app_handle.state::<IRCClientState>();
                let mut servers = state.servers.lock().expect("Failed to lock servers");
                if let Some(ServerRuntime::Registering { tx, handle }) = servers.remove(&server_id)
                {
                    servers.insert(server_id, ServerRuntime::Connected { tx, handle });
                }
            }

            emit_server_status(app_handle, server_id, ServerStatus::Connected)?;

            // Optional: Alert system message
            emit_system_message(app_handle, server_id, "서버에 연결되었습니다.")?;
        }
        _ => {
            // TODO: Command 다른것도 추가하기
        }
    }

    Ok(())
}
