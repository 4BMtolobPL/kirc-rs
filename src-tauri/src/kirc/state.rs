mod app_state;
pub(super) mod server_state;

use crate::kirc::core::server_actor;
use crate::kirc::types::server::ServerConfig;
use crate::kirc::types::ServerId;
use anyhow::anyhow;
use app_state::AppState;
use server_state::{ServerRuntime, ServerState};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use uuid::Uuid;

pub(crate) struct IRCClientState {
    servers: Mutex<HashMap<ServerId, Arc<ServerState>>>,
    app_state: AtomicU8,
}

impl IRCClientState {
    pub(crate) fn new() -> Self {
        Self {
            servers: Mutex::new(HashMap::new()),
            app_state: AtomicU8::new(AppState::Running.as_u8()),
        }
    }

    pub(super) fn get_server(&self, server_id: ServerId) -> Option<Arc<ServerState>> {
        self.servers.lock().unwrap().get(&server_id).cloned()
    }

    pub(super) fn add_server(&self, config: ServerConfig) -> ServerId {
        let server_id = Uuid::now_v7();

        self.servers.lock().unwrap().insert(
            server_id,
            Arc::new(ServerState::new(ServerRuntime::Disconnected, config)),
        );

        server_id
    }

    pub(super) fn run_server(
        &self,
        server_id: ServerId,
        app_handle: &AppHandle,
    ) -> anyhow::Result<()> {
        if let Some(server) = self.get_server(server_id) {
            let config = server.config();

            let handle = tokio::spawn(server_actor(server_id, config, app_handle.clone()));

            server.transition_to_connecting(handle);
            Ok(())
        } else {
            Err(anyhow!("Server not found"))
        }
    }

    fn app_state(&self) -> Option<AppState> {
        AppState::from_u8(self.app_state.load(Ordering::Acquire))
    }

    fn set_app_state(&self, state: AppState) {
        self.app_state.store(state.as_u8(), Ordering::Release);
    }

    pub(super) fn is_shutting_down(&self) -> bool {
        if let Some(state) = self.app_state() {
            state == AppState::ShuttingDown
        } else {
            false
        }
    }

    pub(crate) async fn shutdown(&self) {
        // 1. 상태 전이 AppState -> ShuttingDown
        self.set_app_state(AppState::ShuttingDown);

        // 2. 서버 drain
        let runtimes: Vec<ServerRuntime> = {
            let mut guard = self.servers.lock().unwrap();
            guard
                .drain()
                .map(|(_, state)| state.take_runtime())
                .collect()
        };

        // 3. 병렬 graceful 종료
        futures::future::join_all(
            runtimes
                .into_iter()
                .map(|runtime| runtime.graceful_shutdown()),
        )
        .await;

        // 4. 최종 상태 AppState -> Terminated
        self.set_app_state(AppState::Terminated);
    }

    pub(super) fn is_channel_locked(&self, server_id: ServerId, channel: &str) -> bool {
        self.get_server(server_id)
            .map(|s| s.is_channel_locked(channel))
            .unwrap_or(false)
    }
}
