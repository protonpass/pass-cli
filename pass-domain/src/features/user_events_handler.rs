use crate::{EventId, UserEvents};
use anyhow::Result;

#[async_trait::async_trait]
pub trait UserEventsHandler: Send + Sync {
    async fn get_last_user_event_id(&self) -> Result<Option<EventId>>;
    async fn set_last_user_event_id(&self, event_id: EventId) -> Result<()>;
    async fn tick(&self);
    async fn on_event(&self, event: UserEvents) -> Result<()>;
}
