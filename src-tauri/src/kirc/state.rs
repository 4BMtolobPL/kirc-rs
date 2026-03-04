use crate::kirc::types::{ChannelId, ServerCommand, ServerId, ServerStatus};
use anyhow::anyhow;
use std::cmp::PartialEq;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tokio::time::timeout;

#[derive(Default)]
pub(super) enum ServerRuntime {
    #[default]
    Disconnected,
    Connecting {
        handle: JoinHandle<()>,
    },
    Registering {
        tx: UnboundedSender<ServerCommand>,
        handle: JoinHandle<()>,
    },
    Connected {
        tx: UnboundedSender<ServerCommand>,
        handle: JoinHandle<()>,
    },
    Disconnecting {
        handle: JoinHandle<()>,
    },
    Failed {
        error: String,
    },
}

impl ServerRuntime {
    fn status(&self) -> ServerStatus {
        match self {
            ServerRuntime::Disconnected => ServerStatus::Disconnected,
            ServerRuntime::Connecting { .. } => ServerStatus::Connecting,
            ServerRuntime::Registering { .. } => ServerStatus::Registering,
            ServerRuntime::Connected { .. } => ServerStatus::Connected,
            ServerRuntime::Disconnecting { .. } => ServerStatus::Disconnecting,
            ServerRuntime::Failed { .. } => ServerStatus::Failed,
        }
    }

    async fn graceful_shutdown(self) {
        const TIMEOUT: Duration = Duration::from_secs(5);

        match self {
            ServerRuntime::Connected { tx, handle } => {
                // 1. QUIT 전송
                let _ = tx.send(ServerCommand::Quit);

                // 2. 정상 종료 대기 (timeout optional)
                let _ = timeout(TIMEOUT, handle).await;
            }

            ServerRuntime::Registering { tx, handle } => {
                // 아직 welcome 전이라도 QUIT 시도 가능
                let _ = tx.send(ServerCommand::Quit);
                let _ = timeout(TIMEOUT, handle).await;
            }

            ServerRuntime::Connecting { handle } => {
                handle.abort();
                let _ = handle.await;
            }

            ServerRuntime::Disconnecting { handle } => {
                let _ = timeout(TIMEOUT, handle).await;
            }

            ServerRuntime::Disconnected | ServerRuntime::Failed { .. } => {
                // nothing
            }
        }
    }
}

#[derive(PartialEq)]
pub(crate) enum AppState {
    Running,
    ShuttingDown,
    Terminated,
}

impl AppState {
    fn as_u8(&self) -> u8 {
        match self {
            AppState::Running => 0,
            AppState::ShuttingDown => 1,
            AppState::Terminated => 2,
        }
    }

    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Running),
            1 => Some(Self::ShuttingDown),
            2 => Some(Self::Terminated),
            _ => None,
        }
    }
}

#[derive(Default)]
pub(super) struct ChannelState {
    pub(super) locked: bool,
}

pub(crate) struct IRCClientState {
    pub(super) servers: Mutex<HashMap<ServerId, ServerRuntime>>,
    pub(super) channel_states: Mutex<HashMap<ServerId, HashMap<ChannelId, ChannelState>>>,
    app_state: AtomicU8,
}

impl IRCClientState {
    pub(crate) fn new() -> Self {
        Self {
            servers: Mutex::new(HashMap::new()),
            channel_states: Mutex::new(HashMap::new()),
            app_state: AtomicU8::new(AppState::Running.as_u8()),
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
            guard.drain().map(|(_, runtime)| runtime).collect()
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

    pub(super) fn is_channel_locked(
        &self,
        server_id: ServerId,
        channel: &str,
    ) -> anyhow::Result<bool> {
        let channels = self
            .channel_states
            .lock()
            .map_err(|e| anyhow!("channel state mutex poisoned"))?;

        Ok(channels
            .get(&server_id)
            .and_then(|m| m.get(channel))
            .map(|s| s.locked)
            .unwrap_or(false))
    }
}
