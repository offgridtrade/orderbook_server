pub mod L1;
pub mod L2;
pub mod L3;

use std::path::Path;

use bincode;
use rust_rocksdb::{DB, Options};
use thiserror::Error;

pub use self::L1::L1 as L1Struct;
pub use self::L2::{L2 as L2Struct, Level};
pub use self::L3::{Order, OrderStorage};

#[derive(Debug, Error)]
pub enum SnapshotError {
    #[error("rocksdb error: {0}")]
    RocksDb(#[from] rust_rocksdb::Error),
    #[error("serialization error: {0}")]
    Encode(#[from] bincode::Error),
}

/// Persist the provided L1, L2 and L3 snapshots into RocksDB using fixed keys.
///
/// This is meant as a minimal example of how snapshot data can be written.
pub fn save_levels_snapshot(
    path: impl AsRef<Path>,
    l1: &L1Struct,
    l2: &L2Struct,
    l3: &OrderStorage,
) -> Result<(), SnapshotError> {
    let mut options = Options::default();
    options.create_if_missing(true);
    let db = DB::open(&options, path)?;

    db.put(b"snapshot:l1", bincode::serialize(l1)?)?;
    db.put(b"snapshot:l2", bincode::serialize(l2)?)?;
    db.put(b"snapshot:l3", bincode::serialize(l3)?)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn saves_levels_snapshot() {
        let dir = tempdir().expect("create temp dir");

        let l1 = L1Struct::new(100, 90, 110, 5, 5, 10, 10);
        let l2 = L2Struct {
            bids: vec![Level {
                price: 100,
                quantity: 1_000,
            }],
            asks: vec![Level {
                price: 110,
                quantity: 2_000,
            }],
            ..Default::default()
        };

        let mut storage = OrderStorage::new();
        let (id, _) = storage
            .create_order(vec![1], vec![2], 100, 10, 10, 0)
            .expect("create order");
        storage.insert_id(100, id, 10).expect("insert id");

        save_levels_snapshot(dir.path(), &l1, &l2, &storage).expect("save snapshot");

        let db = DB::open_default(dir.path()).expect("open db");
        let loaded_l1: L1Struct = bincode::deserialize(&db.get(b"snapshot:l1").unwrap().unwrap())
            .expect("deserialize l1");
        assert_eq!(loaded_l1, l1);

        let loaded_l2: L2Struct = bincode::deserialize(&db.get(b"snapshot:l2").unwrap().unwrap())
            .expect("deserialize l2");
        assert_eq!(loaded_l2, l2);

        let loaded_l3: OrderStorage =
            bincode::deserialize(&db.get(b"snapshot:l3").unwrap().unwrap())
                .expect("deserialize l3");
        assert_eq!(
            loaded_l3.get_order(id).unwrap(),
            storage.get_order(id).unwrap()
        );
    }
}
