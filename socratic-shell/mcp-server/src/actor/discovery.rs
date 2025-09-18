// Discovery actor - handles marco/polo protocol for server discovery

use tokio::sync::mpsc;

/// Messages that can be sent to the discovery actor
enum DiscoveryMessage {
    // Handle incoming marco message and respond with polo
    HandleMarco {
        // TODO: Add necessary fields for marco handling
    },
}

/// Actor that handles discovery protocol (marco/polo)
struct DiscoveryActor {
    receiver: mpsc::Receiver<DiscoveryMessage>,
}

impl DiscoveryActor {
    fn new(receiver: mpsc::Receiver<DiscoveryMessage>) -> Self {
        Self { receiver }
    }

    async fn run(mut self) {
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg).await;
        }
    }

    async fn handle_message(&mut self, msg: DiscoveryMessage) {
        match msg {
            DiscoveryMessage::HandleMarco { .. } => {
                // TODO: Implement marco handling
            }
        }
    }
}

/// Handle for communicating with the discovery actor
#[derive(Clone)]
pub struct DiscoveryHandle {
    sender: mpsc::Sender<DiscoveryMessage>,
}

impl DiscoveryHandle {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let actor = DiscoveryActor::new(receiver);
        tokio::spawn(async move { actor.run().await });

        Self { sender }
    }

    // TODO: Add public methods for discovery operations
}
