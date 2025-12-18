use anyhow::Result;
use pass_domain::{EventId, UserEvents, UserEventsHandler};
use tokio::sync::RwLock;
use tokio::sync::mpsc::Sender;

pub struct SshAgentEventHandler {
    last_event_id: RwLock<Option<EventId>>,
    refresh_interval: u64,
    tx: Sender<UserEvents>,
}

impl SshAgentEventHandler {
    pub fn new(tx: Sender<UserEvents>, refresh_interval: u64) -> Self {
        Self {
            last_event_id: RwLock::new(None),
            refresh_interval,
            tx,
        }
    }
}

#[async_trait::async_trait]
impl UserEventsHandler for SshAgentEventHandler {
    async fn get_last_user_event_id(&self) -> Result<Option<EventId>> {
        Ok(self.last_event_id.read().await.clone())
    }

    async fn set_last_user_event_id(&self, event_id: EventId) -> Result<()> {
        self.last_event_id.write().await.replace(event_id);
        Ok(())
    }

    async fn tick(&self) {
        tokio::time::sleep(std::time::Duration::from_secs(self.refresh_interval)).await;
    }

    async fn on_event(&self, event: UserEvents) -> Result<()> {
        if let Err(e) = self.tx.send(event).await {
            warn!("Error sending event: {:?}", e);
        }

        Ok(())
    }
}
