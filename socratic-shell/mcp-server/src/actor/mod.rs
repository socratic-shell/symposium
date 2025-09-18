// Actor module for IPC refactoring
//
// This module contains focused actors following the Tokio actor pattern:
// - Each actor owns specific state and responsibilities
// - Actors communicate via message passing channels
// - Clean separation of concerns

pub mod daemon;
pub mod discovery;
pub mod reference;

// Re-export handles for easy access
pub use daemon::DaemonHandle;
pub use discovery::DiscoveryHandle;
pub use reference::ReferenceHandle;
