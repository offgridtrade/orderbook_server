use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use rust_rocksdb::{DB, Options};
use thiserror::Error;
#[cfg(test)]
use tempfile::tempdir;

use crate::{primitives, L1, L2, L3OrderStorage, Level};

#[derive(Debug, Error)]
pub enum SnapshotError {
    #[error("rocksdb error: {0}")]
    RocksDb(#[from] rust_rocksdb::Error),
    #[error("invalid snapshot length: expected {expected}, got {actual}")]
    InvalidSnapshotLength { expected: usize, actual: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookLevel {
    pub pair_id: u64,
    pub bid_head: u64,
    pub ask_head: u64,
    pub last_match_price: u64,
}

impl BookLevel {
    const BYTES: usize = 32;

    pub fn to_bytes(&self) -> [u8; Self::BYTES] {
        let mut buf = [0u8; Self::BYTES];
        buf[0..8].copy_from_slice(&self.pair_id.to_le_bytes());
        buf[8..16].copy_from_slice(&self.bid_head.to_le_bytes());
        buf[16..24].copy_from_slice(&self.ask_head.to_le_bytes());
        buf[24..32].copy_from_slice(&self.last_match_price.to_le_bytes());
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SnapshotError> {
        if bytes.len() != Self::BYTES {
            return Err(SnapshotError::InvalidSnapshotLength {
                expected: Self::BYTES,
                actual: bytes.len(),
            });
        }

        Ok(Self {
            pair_id: u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
            bid_head: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
            ask_head: u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
            last_match_price: u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
        })
    }
}

pub struct SnapshotStore {
    db: DB,
}

impl SnapshotStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SnapshotError> {
        let mut options = Options::default();
        options.create_if_missing(true);
        let db = DB::open(&options, path)?;
        Ok(Self { db })
    }

    pub fn save_book_level(&self, level: &BookLevel) -> Result<(), SnapshotError> {
        let key = Self::book_level_key(level.pair_id);
        self.db.put(key, level.to_bytes())?;
        Ok(())
    }

    pub fn load_book_level(&self, pair_id: u64) -> Result<Option<BookLevel>, SnapshotError> {
        let key = Self::book_level_key(pair_id);
        let value = self.db.get(key)?;
        match value {
            Some(bytes) => Ok(Some(BookLevel::from_bytes(&bytes)?)),
            None => Ok(None),
        }
    }

    fn book_level_key(pair_id: u64) -> String {
        format!("pair:{pair_id}:l1")
    }
}

pub trait LevelsSnapshotSource: Send + Sync + 'static {
    fn snapshot_levels(&self) -> (L1, L2, L3OrderStorage);
}

pub struct SnapshotCron {
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl SnapshotCron {
    pub fn start<P>(path: impl Into<PathBuf>, interval: Duration, provider: Arc<P>) -> Self
    where
        P: LevelsSnapshotSource,
    {
        assert!(!interval.is_zero(), "snapshot interval must be non-zero");

        let path = path.into();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();

        let handle = thread::spawn(move || {
            while !stop_flag.load(Ordering::Relaxed) {
                let started_at = Instant::now();
                let (l1, l2, l3) = provider.snapshot_levels();
                if let Err(err) = primitives::save_levels_snapshot(&path, &l1, &l2, &l3) {
                    eprintln!("snapshot cron failed to persist levels: {err:?}");
                }

                let elapsed = started_at.elapsed();
                if elapsed < interval {
                    let sleep_for = interval - elapsed;
                    let mut remaining = sleep_for;
                    while remaining > Duration::from_millis(10) {
                        if stop_flag.load(Ordering::Relaxed) {
                            return;
                        }
                        let step = remaining.min(Duration::from_millis(50));
                        thread::sleep(step);
                        remaining -= step;
                    }
                    if !stop_flag.load(Ordering::Relaxed) {
                        thread::sleep(remaining);
                    }
                }
            }
        });

        Self {
            stop,
            handle: Some(handle),
        }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for SnapshotCron {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(test)]
    use tempfile::tempdir;

    #[test]
    fn save_and_load_book_level() {
        let dir = tempdir().expect("create temp dir");
        let store = SnapshotStore::open(dir.path()).expect("open snapshot store");

        let level = BookLevel {
            pair_id: 42,
            bid_head: 110,
            ask_head: 120,
            last_match_price: 115,
        };

        store.save_book_level(&level).expect("save book level");
        let loaded = store
            .load_book_level(level.pair_id)
            .expect("load book level")
            .expect("book level exists");

        assert_eq!(level, loaded);
    }
}

#[cfg(test)]
mod cron_tests {
    use super::*;
    use std::sync::Mutex;
    #[cfg(test)]
    use tempfile::tempdir;

    struct TestSource {
        l1: Mutex<L1>,
        l2: Mutex<L2>,
        l3: Mutex<L3OrderStorage>,
    }

    impl TestSource {
        fn new() -> Self {
            let mut l3 = L3OrderStorage::new();
            let (id, _) = l3
                .create_order(vec![1], vec![2], 100, 10, 10, 0)
                .expect("create order");
            l3.insert_id(100, id, 10).expect("insert");

            Self {
                l1: Mutex::new(L1::new(100, 90, 110, 5, 5, 10, 10)),
                l2: Mutex::new(L2 {
                    bids: vec![Level {
                        price: 100,
                        quantity: 1_000,
                    }],
                    asks: vec![Level {
                        price: 110,
                        quantity: 2_000,
                    }],
                    ..Default::default()
                }),
                l3: Mutex::new(l3),
            }
        }
    }

    impl LevelsSnapshotSource for TestSource {
        fn snapshot_levels(&self) -> (L1, L2, L3OrderStorage) {
            (
                self.l1.lock().unwrap().clone(),
                self.l2.lock().unwrap().clone(),
                self.l3.lock().unwrap().clone(),
            )
        }
    }

    #[test]
    fn snapshot_cron_persists_periodically() {
        let dir = tempdir().expect("create temp dir");
        let provider = Arc::new(TestSource::new());

        {
            let mut cron = SnapshotCron::start(dir.path(), Duration::from_millis(100), provider);
            thread::sleep(Duration::from_millis(250));
            cron.stop();
        }

        let db = DB::open_default(dir.path()).expect("open db");
        assert!(db.get(b"snapshot:l1").unwrap().is_some());
        assert!(db.get(b"snapshot:l2").unwrap().is_some());
        assert!(db.get(b"snapshot:l3").unwrap().is_some());
    }
}
