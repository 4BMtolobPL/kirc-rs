use crate::error::MyCustomError;
use crate::kirc::commands::payload::{ChannelLockPayload, ConnectServerPayload};
use crate::kirc::emits::{emit_channel_lock_changed, emit_server_added, emit_server_status};
use crate::kirc::state::IRCClientState;
use crate::kirc::types::{ServerCommand, ServerId, ServerStatus};
use anyhow::{anyhow, Context};
use tauri::{AppHandle, State};
use tauri_plugin_log::log::info;

#[tauri::command]
pub(crate) async fn connect_server(
    payload: ConnectServerPayload,
    state: State<'_, IRCClientState>,
    app_handle: AppHandle,
) -> Result<(), MyCustomError> {
    if state.is_shutting_down() {
        return Err(MyCustomError::Anyhow(anyhow!(
            "Application is shutting down"
        )));
    }

    let server_id = if let Some(server_id) = payload.server_id() {
        if let Some(server) = state.get_server(server_id) {
            if server.is_active() {
                return Err(MyCustomError::Anyhow(anyhow!(
                    "Already connecting or connected"
                )));
            }

            server_id
        } else {
            // payload에 server_id는 있지만 실제 저장된 server가 없는 경우
            return Err(MyCustomError::Anyhow(anyhow!("Server not found")));
        }
    } else {
        // payload에 server_id가 없는 경우(신규)
        let server_id = state.add_server(payload.to_config());

        emit_server_added(
            &app_handle,
            server_id,
            payload.host(),
            payload.port(),
            payload.tls(),
            payload.nickname(),
            ServerStatus::Disconnected,
        )?;

        server_id
    };

    state.run_server(server_id, &app_handle)?;
    emit_server_status(&app_handle, server_id, ServerStatus::Connecting)?;

    Ok(())
}

#[tauri::command]
pub(crate) fn join_channel(
    server_id: ServerId,
    channel: String,
    state: State<IRCClientState>,
) -> Result<(), MyCustomError> {
    info!("Tauri command: join channel invoked, server_id: {server_id}, channel: {channel}");

    let server = state.get_server(server_id).context("Can't find server")?;
    server.send_command(ServerCommand::Join(channel))?;

    Ok(())
}

#[tauri::command]
pub(crate) fn send_message(
    server_id: ServerId,
    target: String,
    message: String,
    state: State<IRCClientState>,
) -> Result<(), MyCustomError> {
    info!("Tauri command: send message invoked, server_id: {server_id}, target: {target}, message: {message}");

    // 1. 정책 체크
    if state.is_channel_locked(server_id, &target) {
        return Err(MyCustomError::Anyhow(anyhow!("Channel is locked")));
    }

    // 2. 서버 runtime 접근
    let server = state.get_server(server_id).context("Can't find server")?;
    server.send_command(ServerCommand::Privmsg { target, message })?;

    Ok(())
}

#[tauri::command]
pub(crate) fn cancel_connect(
    server_id: ServerId,
    state: State<IRCClientState>,
    app_handle: AppHandle,
) -> Result<(), MyCustomError> {
    info!("Tauri command: cancel connect invoked, server_id: {server_id}");

    if let Some(server) = state.get_server(server_id) {
        if server.abort_connecting() {
            emit_server_status(&app_handle, server_id, ServerStatus::Failed)?;
        }
    }

    Ok(())
}

#[tauri::command]
pub(crate) fn disconnect_server(
    server_id: ServerId,
    state: State<IRCClientState>,
    app_handle: AppHandle,
) -> Result<(), MyCustomError> {
    if let Some(server) = state.get_server(server_id) {
        server.disconnect();
        emit_server_status(&app_handle, server_id, server.status())?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn lock_channel(
    payload: ChannelLockPayload,
    state: State<IRCClientState>,
    app_handle: AppHandle,
) -> Result<(), MyCustomError> {
    let server = state
        .get_server(payload.server_id())
        .context("Can't find server")?;
    server.set_channel_locked(payload.channel(), true);

    emit_channel_lock_changed(&app_handle, payload.server_id(), payload.channel(), true)?;

    Ok(())
}

#[tauri::command]
pub(crate) fn unlock_channel(
    payload: ChannelLockPayload,
    state: State<IRCClientState>,
    app_handle: AppHandle,
) -> Result<(), MyCustomError> {
    if let Some(server) = state.get_server(payload.server_id()) {
        server.set_channel_locked(payload.channel(), false);
    }

    emit_channel_lock_changed(&app_handle, payload.server_id(), payload.channel(), false)?;

    Ok(())
}

#[tauri::command]
pub(crate) fn is_channel_locked(
    payload: ChannelLockPayload,
    state: State<IRCClientState>,
) -> Result<bool, MyCustomError> {
    Ok(state.is_channel_locked(payload.server_id(), payload.channel()))
}

mod payload {
    use crate::kirc::types::server::ServerConfig;
    use crate::kirc::types::{ChannelId, ServerId};
    use serde::Deserialize;

    #[derive(Deserialize, Debug, Clone)]
    pub(crate) struct ConnectServerPayload {
        server_id: Option<ServerId>,
        host: String,
        port: u16,
        tls: bool,
        nickname: String,
    }

    impl ConnectServerPayload {
        pub(super) fn server_id(&self) -> Option<ServerId> {
            self.server_id
        }

        pub(super) fn host(&self) -> &str {
            &self.host
        }

        pub(super) fn port(&self) -> u16 {
            self.port
        }

        pub(super) fn tls(&self) -> bool {
            self.tls
        }

        pub(super) fn nickname(&self) -> &str {
            &self.nickname
        }

        pub(super) fn to_config(&self) -> ServerConfig {
            ServerConfig::new(
                self.host.to_string(),
                self.port,
                self.tls,
                self.nickname.to_string(),
            )
        }
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub(crate) struct ChannelLockPayload {
        server_id: ServerId,
        channel: ChannelId,
    }

    impl ChannelLockPayload {
        pub(super) fn server_id(&self) -> ServerId {
            self.server_id
        }

        pub(super) fn channel(&self) -> &str {
            &self.channel
        }
    }
}
