//! Facade de lancement des workers long-vivants exposes par les features.

mod handles;
mod spawn;

pub use handles::RuntimeWorkerHandles;
pub use spawn::spawn_workers;
