use crate::error::MyCustomError;
use crate::kirc::commands::payload::{ChannelLockPayload, ConnectServerPayload};
use crate::kirc::core::server_actor;
use crate::kirc::emits::{emit_channel_lock_changed, emit_server_added, emit_server_status};
use crate::kirc::state::{IRCClientState, ServerRuntime};
use crate::kirc::types::{ServerCommand, ServerId, ServerStatus};
use anyhow::{anyhow, Context};
use tauri::{AppHandle, State};
use tauri_plugin_log::log::info;
use uuid::Uuid;

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

    let server_id = payload.server_id().unwrap_or(Uuid::now_v7());
    let mut servers = state.servers.lock().unwrap();

    match servers.get(&server_id) {
        Some(
            ServerRuntime::Connecting { .. }
            | ServerRuntime::Registering { .. }
            | ServerRuntime::Connected { .. },
        ) => {
            return Err(MyCustomError::IRCServer(
                "Already connecting or connected".into(),
            ));
        }
        _ => {}
    }

    let handle = tokio::spawn(server_actor(
        server_id,
        payload.to_config(),
        app_handle.clone(),
    ));
    emit_server_added(
        &app_handle,
        server_id,
        payload.host(),
        payload.port(),
        payload.tls(),
        payload.nickname(),
        ServerStatus::Disconnected,
    )?;

    servers.insert(server_id, ServerRuntime::Connecting { handle });
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

    let servers = state.servers.lock().expect("Servers lock poisoned");
    let server = servers.get(&server_id).context("Can't find server")?;

    if let ServerRuntime::Connected { tx, .. } = server {
        tx.send(ServerCommand::Join(channel))
            .context("Failed to send join command")?;
    }

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
    if state.is_channel_locked(server_id, &target)? {
        return Err(MyCustomError::Anyhow(anyhow!("Channel is locked")));
    }

    // 2. 서버 runtime 접근
    let servers = state.servers.lock().expect("Servers lock poisoned");
    let runtime = servers.get(&server_id).context("Can't find server")?;

    if let ServerRuntime::Connected { tx, .. } = runtime {
        tx.send(ServerCommand::Privmsg { target, message })
            .context("Failed to send privmsg")?;
    } else {
        return Err(MyCustomError::Anyhow(anyhow!("Server not connected")));
    }

    Ok(())
}

#[tauri::command]
pub(crate) fn cancel_connect(
    server_id: ServerId,
    state: State<IRCClientState>,
    app_handle: AppHandle,
) -> Result<(), MyCustomError> {
    info!("Tauri command: cancel connect invoked, server_id: {server_id}");
    let mut servers = state.servers.lock().expect("Servers lock poisoned");

    if let Some(ServerRuntime::Connecting { handle }) = servers.remove(&server_id) {
        handle.abort();

        servers.insert(server_id, ServerRuntime::Disconnected);

        emit_server_status(&app_handle, server_id, ServerStatus::Failed)?;
    }

    Ok(())
}

#[tauri::command]
pub(crate) fn disconnect_server(
    server_id: ServerId,
    state: State<IRCClientState>,
) -> Result<(), MyCustomError> {
    let mut servers = state.servers.lock().expect("Servers lock poisoned");

    if let Some(runtime) = servers.remove(&server_id) {
        match runtime {
            ServerRuntime::Registering { tx, handle } | ServerRuntime::Connected { tx, handle } => {
                let _ = tx.send(ServerCommand::Quit);
                servers.insert(server_id, ServerRuntime::Disconnecting { handle });
            }
            other => {
                servers.insert(server_id, other);
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub(crate) fn lock_channel(
    payload: ChannelLockPayload,
    state: State<IRCClientState>,
    app_handle: AppHandle,
) -> Result<(), MyCustomError> {
    {
        let mut channels = state
            .channel_states
            .lock()
            .map_err(|_| MyCustomError::Anyhow(anyhow!("Channels lock poisoned")))?;
        let server_entry = channels.entry(payload.server_id()).or_default();
        let channel_state = server_entry
            .entry(payload.channel().to_string())
            .or_default();

        channel_state.locked = true;
    }

    emit_channel_lock_changed(&app_handle, payload.server_id(), payload.channel(), true)?;

    Ok(())
}

#[tauri::command]
pub(crate) fn unlock_channel(
    payload: ChannelLockPayload,
    state: State<IRCClientState>,
    app_handle: AppHandle,
) -> Result<(), MyCustomError> {
    {
        let mut channels = state
            .channel_states
            .lock()
            .map_err(|_| MyCustomError::Anyhow(anyhow!("Channels lock poisoned")))?;

        if let Some(server_entry) = channels.get_mut(&payload.server_id()) {
            if let Some(channel_state) = server_entry.get_mut(payload.channel()) {
                channel_state.locked = false;
            }
        }
    }

    emit_channel_lock_changed(&app_handle, payload.server_id(), payload.channel(), false)?;

    Ok(())
}

#[tauri::command]
pub fn is_channel_locked(
    payload: ChannelLockPayload,
    state: State<IRCClientState>,
) -> Result<bool, MyCustomError> {
    let channels = state
        .channel_states
        .lock()
        .map_err(|_| MyCustomError::Anyhow(anyhow!("Channels lock poisoned")))?;

    Ok(channels
        .get(&payload.server_id())
        .and_then(|m| m.get(payload.channel()))
        .map(|s| s.locked)
        .unwrap_or(false))
}

mod payload {
    use crate::kirc::core::ServerConfig;
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
