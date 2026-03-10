use crate::error::MyCustomError;
use crate::kirc::commands::payload::{
    ChannelInfo, ChannelPayload, ConnectServerPayload, ServerInfo,
};
use crate::kirc::manager::KircManager;
use crate::kirc::state::kirc::KircState;
use crate::kirc::types::ServerId;
use anyhow::Context;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tauri_plugin_log::log::info;

#[tauri::command]
pub(crate) async fn init_servers(manager: State<'_, KircManager>) -> Result<(), MyCustomError> {
    manager.process_auto_connect();
    Ok(())
}

#[tauri::command]
pub(crate) fn get_servers(
    state: State<'_, Arc<KircState>>,
) -> Result<Vec<ServerInfo>, MyCustomError> {
    let servers = state.get_all_servers();
    let mut infos = Vec::new();

    for (id, server_state) in servers {
        let config = server_state.config();
        let channel_infos = server_state
            .channels()
            .into_iter()
            .map(|(name, s)| ChannelInfo::new(&name, s.locked))
            .collect();

        infos.push(ServerInfo::new(
            id,
            config.server(),
            config.server(),
            config.port(),
            config.use_tls(),
            config.nickname(),
            server_state.status(),
            channel_infos,
        ));
    }

    Ok(infos)
}

#[tauri::command]
pub(crate) async fn connect_server(
    payload: ConnectServerPayload,
    manager: State<'_, KircManager>,
) -> Result<(), MyCustomError> {
    manager
        .connect_server(payload.server_id(), payload.to_config())
        .map_err(MyCustomError::Anyhow)?;

    Ok(())
}

#[tauri::command]
pub(crate) fn join_channel(
    server_id: ServerId,
    channel: String,
    manager: State<KircManager>,
) -> Result<(), MyCustomError> {
    info!("Tauri command: join channel invoked, server_id: {server_id}, channel: {channel}");
    manager
        .join_channel(server_id, &channel)
        .map_err(MyCustomError::Anyhow)?;

    Ok(())
}

#[tauri::command]
pub(crate) fn leave_channel(
    payload: ChannelPayload,
    manager: State<KircManager>,
) -> Result<(), MyCustomError> {
    info!("Tauri command: part channel invoked");
    manager
        .part_channel(payload.server_id(), payload.channel())
        .map_err(MyCustomError::Anyhow)?;

    Ok(())
}

#[tauri::command]
pub(crate) fn send_message(
    server_id: ServerId,
    target: String,
    message: String,
    state: State<Arc<KircState>>,
) -> Result<(), MyCustomError> {
    info!("Tauri command: send message invoked, server_id: {server_id}, target: {target}, message: {message}");

    // 1. 정책 체크
    if state.is_channel_locked(server_id, &target) {
        return Err(MyCustomError::Anyhow(anyhow::anyhow!("Channel is locked")));
    }

    // 2. 서버 runtime 접근
    let server = state.get_server(server_id).context("Can't find server")?;
    server.send_command(crate::kirc::types::ServerCommand::Privmsg { target, message })?;

    Ok(())
}

#[tauri::command]
pub(crate) fn cancel_connect(
    server_id: ServerId,
    manager: State<'_, KircManager>,
) -> Result<(), MyCustomError> {
    info!("Tauri command: cancel connect invoked, server_id: {server_id}");

    manager
        .cancel_connect(server_id)
        .map_err(MyCustomError::Anyhow)?;

    Ok(())
}

#[tauri::command]
pub(crate) fn disconnect_server(
    server_id: ServerId,
    manager: State<'_, KircManager>,
) -> Result<(), MyCustomError> {
    manager
        .disconnect_server(server_id)
        .map_err(MyCustomError::Anyhow)?;
    Ok(())
}

#[tauri::command]
pub(crate) fn lock_channel(
    payload: ChannelPayload,
    state: State<'_, Arc<KircState>>,
    app_handle: AppHandle,
) -> Result<(), MyCustomError> {
    let server = state
        .get_server(payload.server_id())
        .context("Can't find server")?;
    server.set_channel_locked(payload.channel(), true);

    crate::kirc::emits::emit_channel_lock_changed(
        &app_handle,
        payload.server_id(),
        payload.channel(),
        true,
    )?;

    Ok(())
}

#[tauri::command]
pub(crate) fn unlock_channel(
    payload: ChannelPayload,
    state: State<'_, Arc<KircState>>,
    app_handle: AppHandle,
) -> Result<(), MyCustomError> {
    if let Some(server) = state.get_server(payload.server_id()) {
        server.set_channel_locked(payload.channel(), false);
    }

    crate::kirc::emits::emit_channel_lock_changed(
        &app_handle,
        payload.server_id(),
        payload.channel(),
        false,
    )?;

    Ok(())
}

#[tauri::command]
pub(crate) fn is_channel_locked(
    payload: ChannelPayload,
    state: State<'_, Arc<KircState>>,
) -> Result<bool, MyCustomError> {
    Ok(state.is_channel_locked(payload.server_id(), payload.channel()))
}

mod payload {
    use crate::kirc::types::server::ServerConfig;
    use crate::kirc::types::{ChannelId, ServerId, ServerStatus};
    use serde::{Deserialize, Serialize};

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
    pub(crate) struct ChannelPayload {
        server_id: ServerId,
        channel: ChannelId,
    }

    impl ChannelPayload {
        pub(super) fn server_id(&self) -> ServerId {
            self.server_id
        }

        pub(super) fn channel(&self) -> &str {
            &self.channel
        }
    }

    #[derive(Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub(crate) struct ChannelInfo {
        name: String,
        locked: bool,
    }

    impl ChannelInfo {
        pub(super) fn new(name: &str, locked: bool) -> Self {
            Self {
                name: name.to_string(),
                locked,
            }
        }
    }

    #[derive(Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub(crate) struct ServerInfo {
        id: ServerId,
        name: String,
        host: String,
        port: u16,
        tls: bool,
        nickname: String,
        status: ServerStatus,
        channels: Vec<ChannelInfo>,
    }

    impl ServerInfo {
        pub(super) fn new(
            id: ServerId,
            name: &str,
            host: &str,
            port: u16,
            tls: bool,
            nickname: &str,
            status: ServerStatus,
            channels: Vec<ChannelInfo>,
        ) -> Self {
            Self {
                id,
                name: name.to_string(),
                host: host.to_string(),
                port,
                tls,
                nickname: nickname.to_string(),
                status,
                channels,
            }
        }
    }
}
