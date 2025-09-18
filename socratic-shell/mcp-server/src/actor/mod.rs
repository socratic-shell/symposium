// Actor module for IPC refactoring
//
// This module contains focused actors following the Tokio actor pattern:
// - Each actor owns specific state and responsibilities
// - Actors communicate via message passing channels
// - Clean separation of concerns

pub mod client;
pub mod dispatch;
pub mod discovery;
pub mod reference;
pub mod stdout;

// Re-export handles for easy access
pub use client::ClientHandle;
pub use dispatch::DispatchHandle;
pub use discovery::DiscoveryHandle;
pub use reference::ReferenceHandle;
pub use stdout::StdoutHandle;
