use serde::{Serialize, Serializer};
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum MyCustomError {
    /*#[error("IRC server error: {0}")]
    IRCServer(String),*/
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl Serialize for MyCustomError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_serialization() {
        let err = MyCustomError::Anyhow(anyhow::anyhow!("test error"));
        let serialized = serde_json::to_string(&err).unwrap();
        assert_eq!(serialized, "\"test error\"");
    }
}
