#[derive(PartialEq, Debug)]
pub(in crate::kirc) enum AppState {
    Running,
    ShuttingDown,
    Terminated,
}

impl AppState {
    pub(in crate::kirc) fn as_u8(&self) -> u8 {
        match self {
            AppState::Running => 0,
            AppState::ShuttingDown => 1,
            AppState::Terminated => 2,
        }
    }

    pub(in crate::kirc) fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Running),
            1 => Some(Self::ShuttingDown),
            2 => Some(Self::Terminated),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_conversions() {
        assert_eq!(AppState::Running.as_u8(), 0);
        assert_eq!(AppState::ShuttingDown.as_u8(), 1);
        assert_eq!(AppState::Terminated.as_u8(), 2);

        assert_eq!(AppState::from_u8(0), Some(AppState::Running));
        assert_eq!(AppState::from_u8(1), Some(AppState::ShuttingDown));
        assert_eq!(AppState::from_u8(2), Some(AppState::Terminated));
        assert_eq!(AppState::from_u8(3), None);
    }
}
