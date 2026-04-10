use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub(in crate::kirc) struct ChannelState {
    pub(in crate::kirc) name: String,
    pub(in crate::kirc) locked: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_state_new() {
        let name = "#test".to_string();
        let state = ChannelState {
            name: name.clone(),
            locked: false,
        };

        assert_eq!(state.name, name);
        assert!(!state.locked);
    }

    #[test]
    fn test_channel_state_serialization() {
        let state = ChannelState {
            name: "#test".to_string(),
            locked: true,
        };

        let serialized = serde_json::to_string(&state).unwrap();
        let deserialized: ChannelState = serde_json::from_str(&serialized).unwrap();

        assert_eq!(state, deserialized);
    }
}
