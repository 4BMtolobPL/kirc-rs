use crate::kirc::ctcp::{parse_ctcp, CtcpCommand};
use crate::kirc::emits::{
    emit_change_nick_failed, emit_server_status, emit_system_message, emit_ui_event,
};
use crate::kirc::state::kirc::KircState;
use crate::kirc::types::server::ServerConfig;
use crate::kirc::types::{ServerCommand, ServerId, ServerStatus};
use futures::prelude::*;
use irc::client::prelude::*;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tracing::{debug, error, info, instrument, trace, warn};

#[instrument(name = "server_actor", skip_all, fields(server_id = %server_id))]
pub(super) async fn server_actor(
    server_id: ServerId,
    server_config: ServerConfig,
    app_handle: AppHandle,
) {
    // actor에선 error를 ?로 전파하지 않고, 소비/로깅만 하거나 이벤트로 전파
    debug!(server_id = %server_id, "Starting server actor");

    let config = Config {
        server: Some(server_config.server().to_string()),
        port: Some(server_config.port()),
        use_tls: Some(server_config.use_tls()),
        nickname: Some(server_config.nickname().to_string()),
        // alt 닉네임 직접 제어
        /*alt_nicks: vec![
            format!("{}_", server_config.nickname()),
            format!("{}__", server_config.nickname()),
        ],*/
        alt_nicks: vec![],
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
        let state = app_handle.state::<Arc<KircState>>();
        if let Some(server) = state.get_server(server_id) {
            server.transition_to_registering(tx.clone());
        }
    }

    let _ = emit_server_status(&app_handle, server_id, ServerStatus::Registering);

    trace!(server_id = %server_id, "Start server actor loop");
    loop {
        tokio::select! {
            Some(result) = stream.next() => {
                match result {
                    Ok(message) => {
                        let _ = handle_message(&client, server_id, message, &app_handle);
                    }
                    Err(irc::error::Error::NoUsableNick) => {
                        // 사용 가능한 닉네임이 없을때 (단순 닉네임 중복 등)
                        // 처음부터 사용가능한 닉네임이 없었다면 identify에서 걸렸을테니 break 하지 않고 continue
                        debug!(event = "irc_stream_error", error = "NoUsableNick", "IRC stream error, No usable nick");

                        // 닉네임 변경은 alt_nick 없이 실패시 emit 후 바로 continue
                        let _ = emit_change_nick_failed(&app_handle, server_id, "Can't change nickname");

                        continue
                    }
                    Err(e) => {
                        error!(
                            event = "irc_stream_error",
                            error = ?e,
                            "IRC stream error, connection likely closed"
                        );
                        break
                    },
                }
            }
            Some(cmd) = rx.recv() => {
                match cmd {
                    ServerCommand::Join(ch) => {
                        info!(event = "join", channel = %ch);
                        if let Err(e) = client.send_join(&ch) {
                            error!("Failed to send join message: {e}");
                        }
                    }
                    ServerCommand::Privmsg { target, message } => {
                        if let Err(e) = client.send_privmsg(&target, &message) {
                            error!("Failed to send privmsg: {e}");
                        }

                        let current_nick = {
                            let state = app_handle.state::<Arc<KircState>>();
                            if let Some(server) = state.get_server(server_id) {
                                &server.current_nickname()
                            } else {
                                ""
                            }
                        };

                        match Message::with_tags(None, Some(current_nick), "PRIVMSG", vec![&target, &message]) {
                                Ok(msg) => {
                                    handle_message(&client, server_id, msg, &app_handle).expect("Failed to handle message");
                                }
                                Err(_) => {
                                    error!("Failed to create echo message");
                                }
                            }
                    }
                    ServerCommand::Part { channel_name } => {
                        if let Err(e) = client.send_part(&channel_name) {
                            error!("Failed to send part: {e}");
                        }
                    }
                    ServerCommand::Nick( new_nick ) => {
                        info!(event = "nick", new_nick = %new_nick);
                        if let Err(e) = client.send(Command::NICK(new_nick.to_owned())) {
                            error!(event = "nick_send_failed", command = "NICK", new_nick = %new_nick, error = %e, "failed to send IRC NICK command");
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
        let state = app_handle.state::<Arc<KircState>>();
        if let Some(server) = state.get_server(server_id) {
            server.transition_to_disconnected();
        }
    }

    let _ = emit_server_status(&app_handle, server_id, ServerStatus::Disconnected);
}

fn fail_state(server_id: ServerId, app_handle: AppHandle, message: String) {
    let state = app_handle.state::<Arc<KircState>>();

    if let Some(server) = state.get_server(server_id) {
        server.transition_to_failed(message);
    }

    let _ = emit_server_status(&app_handle, server_id, ServerStatus::Failed);
}

/// 서버에서 클라이언트로 보낸 메세지 핸들링
#[instrument(skip(client, app_handle, message), level = "trace")]
fn handle_message(
    client: &Client,
    server_id: ServerId,
    message: Message,
    app_handle: &AppHandle,
) -> anyhow::Result<()> {
    let source_nickname = message.source_nickname().unwrap_or("").to_string();

    match message.command {
        Command::PRIVMSG(target, content) => {
            if let Some(ctcp) = parse_ctcp(&content) {
                info!(target = %target, content = %content, "Received CTCP message");
                handle_ctcp(client, &source_nickname, ctcp);
            } else {
                emit_ui_event(app_handle)
                    .user_message(server_id, target, source_nickname, content)
                    .emit()?;
            }
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
            // 1. 자기 자신인지 확인
            let state = app_handle.state::<Arc<KircState>>();
            if let Some(server) = state.get_server(server_id) {
                if server.current_nickname() == source_nickname {
                    server.set_current_nickname(&nickname);
                }
            }

            // 지금은 백엔드에서 유저목록 관리 x, 나중에 변경될 수 있음
            // 2. 모든 채널에서 유저 닉 변경
            /*for channel in state.channels.values_mut() {
                if channel.users.remove(&old_nick) {
                    channel.users.insert(new_nick.clone());
                }
            }*/

            // 3. 프론트로 이벤트 emit
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
            warn!(message = %message, "Server command error");
            emit_ui_event(app_handle).error(server_id, message).emit()?;
        }
        Command::Response(Response::RPL_WELCOME, _) => {
            trace!("Response RPL_WELCOME");
            {
                let state = app_handle.state::<Arc<KircState>>();
                if let Some(server) = state.get_server(server_id) {
                    server.transition_to_connected();

                    // 기존 채널이 존재하면 연결
                    for (channel_name, _channel_state) in server.channels() {
                        server.send_command(ServerCommand::Join(channel_name))?;
                    }
                }
            }

            emit_server_status(app_handle, server_id, ServerStatus::Connected)?;

            // Optional: Alert system message
            emit_system_message(app_handle, server_id, "서버에 연결되었습니다.")?;
        }
        Command::Response(Response::ERR_NICKNAMEINUSE, _) => {
            // 닉네임이 중복된 경우
            // alt nick으로 재시도
            // 그래도 안될경우 오류 던지기
            trace!("닉네임 중복");
        }
        _ => {
            // TODO: Command 다른것도 추가하기
            debug!(event = "unprocessed_irc_message", command = ?message.command);
        }
    }

    Ok(())
}

fn handle_ctcp(client: &Client, source_nickname: &str, ctcp: CtcpCommand) {
    debug!(event = "handle_ctcp_message", command = ?ctcp);
    match ctcp {
        CtcpCommand::Version => {
            let reply = "\x01VERSION kirc v0.1\x01";
            let _ = client.send_notice(source_nickname, reply);
        }

        CtcpCommand::Ping(payload) => {
            let reply = format!("\x01PING {}\x01", payload);
            let _ = client.send_notice(source_nickname, &reply);
        }

        CtcpCommand::Time => {
            let now = chrono::Local::now().to_rfc2822();
            let reply = format!("\x01TIME {}\x01", now);
            let _ = client.send_notice(source_nickname, &reply);
        }

        CtcpCommand::Unknown(msg) => {
            // 무시 (보통 응답 안 함)
            warn!(event = "unknown_ctcp_command", message = %msg)
        }
    }
}
