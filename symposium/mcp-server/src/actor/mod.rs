// Actor module for IPC refactoring
//
// This module contains focused actors following the Tokio actor pattern:
// - Each actor owns specific state and responsibilities
// - Actors communicate via message passing channels
// - Clean separation of concerns

use tokio::task::JoinHandle;

/// Trait for actors that can be spawned as Tokio tasks
pub trait Actor: Sized + Send + 'static {
    /// Spawn the actor as a background task
    fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move { self.run().await })
    }

    /// Run the actor's main loop
    fn run(self) -> impl std::future::Future<Output = ()> + Send;
}

pub mod client;
pub mod dispatch;
pub mod reference;
pub mod repeater;
pub mod stdio;

// Re-export handles for easy access
pub use client::spawn_client;
pub use dispatch::DispatchHandle;
pub use reference::ReferenceHandle;
pub use stdio::StdioHandle;
