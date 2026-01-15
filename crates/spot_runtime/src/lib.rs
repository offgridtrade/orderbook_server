pub mod network;
pub mod jobs;
pub mod metrics;
pub mod snapshot;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
