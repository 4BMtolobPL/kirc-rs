use crate::kirc::state::channel::ChannelState;
use crate::kirc::state::kirc::KircState;
use crate::kirc::state::server::ServerState;
use crate::kirc::types::server::ServerConfig;
use crate::kirc::types::ChannelId;
use crate::memento::Memento;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub(super) struct ServerStateSnapshot {
    config: ServerConfig,
    channels: HashMap<ChannelId, ChannelState>,
}

impl ServerStateSnapshot {
    pub(super) fn new(config: ServerConfig, channels: HashMap<ChannelId, ChannelState>) -> Self {
        Self { config, channels }
    }
}

impl Memento<ServerState> for ServerStateSnapshot {
    fn restore(self) -> ServerState {
        ServerState::with_channel(self.config, self.channels)
    }
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct KircStateSnapshot {
    servers: Vec<ServerStateSnapshot>,
}

impl From<Vec<ServerStateSnapshot>> for KircStateSnapshot {
    fn from(value: Vec<ServerStateSnapshot>) -> Self {
        Self { servers: value }
    }
}

impl FromIterator<ServerStateSnapshot> for KircStateSnapshot {
    fn from_iter<T: IntoIterator<Item = ServerStateSnapshot>>(iter: T) -> Self {
        Self {
            servers: iter.into_iter().collect(),
        }
    }
}

impl Memento<KircState> for KircStateSnapshot {
    fn restore(self) -> KircState {
        KircState::from_iter(self.servers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kirc::types::server::ServerConfig;
    use crate::kirc::types::ServerStatus;

    fn mock_server_config(name: &str) -> ServerConfig {
        ServerConfig::new(name.to_string(), 6667, false, "nick".to_string())
    }

    #[test]
    fn test_server_state_snapshot_serialization() {
        let config = mock_server_config("irc.test.net");
        let mut channels = HashMap::new();
        channels.insert(
            "#test".to_string(),
            ChannelState {
                name: "#test".to_string(),
                locked: false,
            },
        );

        let snapshot = ServerStateSnapshot::new(config, channels);
        let serialized = serde_json::to_string(&snapshot).unwrap();
        let deserialized: ServerStateSnapshot = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.config.server(), "irc.test.net");
        assert!(deserialized.channels.contains_key("#test"));
    }

    #[test]
    fn test_server_state_snapshot_restore() {
        let config = mock_server_config("irc.test.net");
        let mut channels = HashMap::new();
        channels.insert(
            "#test".to_string(),
            ChannelState {
                name: "#test".to_string(),
                locked: true,
            },
        );

        let snapshot = ServerStateSnapshot::new(config, channels);
        let server_state = snapshot.restore();

        assert_eq!(server_state.status(), ServerStatus::Disconnected);
        assert!(server_state.channels().contains_key("#test"));
        assert!(server_state.is_channel_locked("#test"));
    }

    #[test]
    fn test_kirc_state_snapshot_restoration() {
        let config1 = mock_server_config("irc.1.net");
        let config2 = mock_server_config("irc.2.net");

        let s1 = ServerStateSnapshot::new(config1, HashMap::new());
        let s2 = ServerStateSnapshot::new(config2, HashMap::new());

        let kirc_snapshot = KircStateSnapshot::from_iter(vec![s1, s2]);
        let kirc_state = kirc_snapshot.restore();

        assert_eq!(kirc_state.get_all_servers().len(), 2);
    }
}
