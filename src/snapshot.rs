use std::fmt;
use std::path::Path;

use anyhow::Context;
use rust_rocksdb::{DB, Options};

/// Represents a snapshot depth of the order book.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum BookLevel {
    L1,
    L2,
    L3,
}

impl fmt::Display for BookLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BookLevel::L1 => write!(f, "L1"),
            BookLevel::L2 => write!(f, "L2"),
            BookLevel::L3 => write!(f, "L3"),
        }
    }
}

impl BookLevel {
    fn key(&self) -> String {
        format!("snapshot:{}", self)
    }
}

/// Stores order book snapshots in RocksDB.
///
/// Each level (L1, L2, L3) is mapped to its own key inside a RocksDB instance.
pub struct SnapshotStore {
    db: DB,
}

impl SnapshotStore {
    /// Opens (or creates) a RocksDB instance at `path`.
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let mut options = Options::default();
        options.create_if_missing(true);

        let db = DB::open(&options, path).context("failed to open RocksDB instance")?;
        Ok(Self { db })
    }

    /// Persists snapshot bytes for the given book level.
    pub fn save_snapshot(
        &self,
        level: BookLevel,
        snapshot: impl AsRef<[u8]>,
    ) -> anyhow::Result<()> {
        let key = level.key();
        self.db
            .put(key.as_bytes(), snapshot.as_ref())
            .context("failed to persist snapshot")?;
        Ok(())
    }

    /// Retrieves previously stored snapshot bytes for the given level, if any.
    pub fn load_snapshot(&self, level: BookLevel) -> anyhow::Result<Option<Vec<u8>>> {
        let key = level.key();
        let result = self
            .db
            .get(key.as_bytes())
            .context("failed to load snapshot")?;
        Ok(result)
    }

    /// Removes the snapshot for the provided level.
    pub fn delete_snapshot(&self, level: BookLevel) -> anyhow::Result<()> {
        let key = level.key();
        self.db
            .delete(key.as_bytes())
            .context("failed to delete snapshot")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn save_and_load_snapshots_for_each_level() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let store = SnapshotStore::open(temp_dir.path())?;

        let samples = [
            (BookLevel::L1, b"L1 snapshot data".as_ref()),
            (BookLevel::L2, b"L2 snapshot data".as_ref()),
            (BookLevel::L3, b"L3 snapshot data".as_ref()),
        ];

        for (level, data) in samples {
            store.save_snapshot(level, data)?;
            let stored = store.load_snapshot(level)?.expect("snapshot missing");
            assert_eq!(stored, data);
        }

        store.delete_snapshot(BookLevel::L2)?;
        assert!(store.load_snapshot(BookLevel::L2)?.is_none());

        Ok(())
    }
}
