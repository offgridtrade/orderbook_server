use offgrid_primitives::matching_engine::MatchingEngine;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Deserialization error: {0}")]
    Deserialization(String),
}

impl From<postcard::Error> for SnapshotError {
    fn from(err: postcard::Error) -> Self {
        SnapshotError::Serialization(format!("{}", err))
    }
}

/// Save a snapshot of the matching engine state to disk
/// 
/// This saves the entire state (all pairs and their orderbooks) to a binary file.
/// The snapshot can be loaded later to recover the state after a server restart.
/// 
/// # Arguments
/// * `engine` - Reference to the MatchingEngine to snapshot
/// * `path` - Path where the snapshot will be saved
pub fn save_snapshot<P: AsRef<Path>>(engine: &MatchingEngine, path: P) -> Result<(), SnapshotError> {
    // Serialize to binary format using postcard
    let data = postcard::to_allocvec(engine)
        .map_err(|e| SnapshotError::Serialization(format!("Failed to serialize: {}", e)))?;

    // Atomic write: write to temp file first, then rename
    let path_ref = path.as_ref();
    let temp_path = path_ref.with_extension("tmp");
    
    // Create parent directory if it doesn't exist
    if let Some(parent) = path_ref.parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Write to temporary file
    let mut file = fs::File::create(&temp_path)?;
    file.write_all(&data)?;
    file.sync_all()?; // Ensure data is flushed to disk
    
    // Atomically rename (this is atomic on most filesystems)
    fs::rename(&temp_path, path_ref)?;
    
    Ok(())
}

/// Load a snapshot of the matching engine state from disk
/// 
/// This restores the entire state (all pairs and their orderbooks) from a previously saved snapshot.
/// 
/// # Arguments
/// * `path` - Path to the snapshot file to load
pub fn load_snapshot<P: AsRef<Path>>(path: P) -> Result<MatchingEngine, SnapshotError> {
    let path_ref = path.as_ref();
    
    // Check if file exists
    if !path_ref.exists() {
        return Err(SnapshotError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Snapshot file not found: {}", path_ref.display()),
        )));
    }

    // Read file
    let mut file = fs::File::open(path_ref)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    // Deserialize using postcard
    let engine = postcard::from_bytes(&data)
        .map_err(|e| SnapshotError::Deserialization(format!("Failed to deserialize: {}", e)))?;

    Ok(engine)
}

/// Load a snapshot or create a new matching engine if snapshot doesn't exist
/// 
/// This is a convenience function that attempts to load a snapshot, but falls back
/// to creating a new matching engine if the snapshot file doesn't exist.
/// 
/// # Arguments
/// * `path` - Path to the snapshot file to load (if it exists)
pub fn load_snapshot_or_new<P: AsRef<Path>>(path: P) -> Result<MatchingEngine, SnapshotError> {
    match load_snapshot(&path) {
        Ok(engine) => Ok(engine),
        Err(SnapshotError::Io(ref e)) if e.kind() == std::io::ErrorKind::NotFound => {
            // Snapshot doesn't exist, create new engine
            Ok(MatchingEngine::new())
        }
        Err(e) => Err(e),
    }
}

/// Spawn a snapshot thread that periodically saves the matching engine state
/// 
/// # Arguments
/// * `engine` - Shared reference to the MatchingEngine
/// * `snapshot_path` - Path where snapshots will be saved
/// * `interval_seconds` - How often to take snapshots (in seconds)
/// * `shutdown_flag` - Flag to signal shutdown
pub fn spawn_snapshot_thread(
    engine: Arc<Mutex<MatchingEngine>>,
    snapshot_path: String,
    interval_seconds: u64,
    shutdown_flag: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        println!("Snapshot thread started (interval: {}s, path: {})", interval_seconds, snapshot_path);
        let interval = Duration::from_secs(interval_seconds);
        
        loop {
            // Wait for interval or shutdown signal
            for _ in 0..(interval_seconds * 10) {
                if shutdown_flag.load(Ordering::Relaxed) {
                    // Before shutdown, save one final snapshot
                    println!("Taking final snapshot before shutdown...");
                    if let Ok(engine_guard) = engine.lock() {
                        if let Err(e) = save_snapshot(&*engine_guard, &snapshot_path) {
                            eprintln!("Error saving final snapshot: {}", e);
                        } else {
                            println!("Final snapshot saved successfully");
                        }
                    }
                    println!("Snapshot thread stopped");
                    return;
                }
                thread::sleep(Duration::from_millis(100));
            }
            
            // Take snapshot
            if let Ok(engine_guard) = engine.lock() {
                match save_snapshot(&*engine_guard, &snapshot_path) {
                    Ok(()) => {
                        println!("Snapshot saved successfully to {}", snapshot_path);
                    }
                    Err(e) => {
                        eprintln!("Error saving snapshot: {}", e);
                    }
                }
            } else {
                eprintln!("Failed to acquire lock for snapshot");
            }
        }
    })
}
