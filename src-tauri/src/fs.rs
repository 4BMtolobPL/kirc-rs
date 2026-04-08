use crate::memento::Memento;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::path::Path;
use tracing::debug;

pub(crate) fn load<T, A>(path: &Path) -> anyhow::Result<T>
where
    T: Memento<A> + DeserializeOwned + Default,
{
    debug!(path = %path.display(), "Load");
    if !path.exists() {
        return Ok(T::default());
    }

    let contents = fs::read_to_string(path)?;
    let snapshot = serde_json::from_str(&contents)?;
    Ok(snapshot)
}

pub(crate) fn save<T, A>(path: &Path, snapshot: T) -> anyhow::Result<()>
where
    T: Memento<A> + Serialize,
{
    debug!(path = %path.display(), "Save");
    let tmp_path = path.with_extension("tmp");
    let data = serde_json::to_string(&snapshot)?;

    fs::write(&tmp_path, data)?;
    fs::rename(&tmp_path, path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::tempdir;

    #[derive(Serialize, Deserialize, Default, PartialEq, Debug)]
    struct MockSnapshot {
        data: String,
    }

    impl Memento<String> for MockSnapshot {
        fn restore(self) -> String {
            self.data
        }
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        let snapshot = MockSnapshot {
            data: "hello".to_string(),
        };
        save(&path, snapshot).unwrap();
        assert!(path.exists());

        let loaded: MockSnapshot = load(&path).unwrap();
        assert_eq!(loaded.data, "hello");
    }

    #[test]
    fn test_load_default_if_not_exists() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("non_existent.json");

        let loaded: MockSnapshot = load(&path).unwrap();
        assert_eq!(loaded.data, "");
    }
}
