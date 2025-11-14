use anyhow::{Context, Result};
use pass::PassClient;
use pass_db::{ActivityTimeModel, DatabaseManager, TelemetryEventModel};
use pass_domain::{TelemetryEvent, TelemetryHandler};
use std::convert::TryFrom;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

const TELEMETRY_SEND_INTERVAL: Duration = Duration::from_secs(3 * 24 * 60 * 60);
const TELEMETRY_SENT_ACTIVITY: &str = "telemetry_sent";

#[derive(Clone)]
pub struct SqliteTelemetryHandler {
    db: DatabaseManager,
    user_id: Arc<RwLock<Option<String>>>,
}

impl SqliteTelemetryHandler {
    pub fn new(db: DatabaseManager) -> Self {
        Self {
            db,
            user_id: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_user_id(&self, user_id: Option<String>) {
        *self.user_id.write().await = user_id;
    }

    async fn get_user_id(&self) -> Option<String> {
        let cached = self.user_id.read().await;
        cached.clone()
    }

    /// Check if telemetry needs to be sent
    async fn needs_to_send_telemetry(&self, user_id: &str) -> Result<bool> {
        let conn = self.db.get_connection().await?;

        let record = ActivityTimeModel::get(&conn, Some(user_id), TELEMETRY_SENT_ACTIVITY)
            .await
            .context("Failed to check telemetry send time")?;

        match record {
            Some(activity_time) => {
                let elapsed = chrono::Utc::now().timestamp() - activity_time.timestamp;
                Ok(elapsed >= TELEMETRY_SEND_INTERVAL.as_secs() as i64)
            }
            None => {
                // No record found, insert current time and return false
                let now = chrono::Utc::now().timestamp();
                ActivityTimeModel::upsert(
                    &conn,
                    Some(user_id.to_string()),
                    TELEMETRY_SENT_ACTIVITY,
                    now,
                )
                .await
                .context("Failed to insert activity_time")?;
                Ok(false)
            }
        }
    }

    /// Update the last telemetry send time
    async fn update_last_send_time(&self, user_id: Option<&str>) -> Result<()> {
        let conn = self.db.get_connection().await?;
        let now = chrono::Utc::now().timestamp();

        ActivityTimeModel::upsert(
            &conn,
            user_id.map(|s| s.to_string()),
            TELEMETRY_SENT_ACTIVITY,
            now,
        )
        .await
        .context("Failed to update last send time")
    }

    pub async fn get_telemetry_events(&self, user_id: &str) -> Result<Vec<TelemetryEvent>> {
        let conn = self.db.get_connection().await?;
        let events = TelemetryEventModel::get_by_user_id(&conn, user_id)
            .await
            .context("Error retrieving telemetry events")?;

        let mut res = Vec::with_capacity(events.len());
        for event in events {
            let mapped = TelemetryEvent::try_from(event)?;
            res.push(mapped);
        }

        Ok(res)
    }

    /// Send telemetry events to the server
    pub async fn send_telemetry(&self, client: &PassClient) -> Result<()> {
        unimplemented!()
        /*
        let user_id = self.get_user_id().await;
        let user_id_ref = user_id.as_deref();

        // Check if we need to send telemetry
        if !self.needs_to_send_telemetry(user_id_ref).await? {
            debug!("Telemetry send not needed yet");
            return Ok(());
        }

        debug!("Sending telemetry events");

        // Call the PassClient method to send events
        match client.send_telemetry_events(vec![]).await {
            Ok(_) => {
                // Clear the telemetry events table
                self.clear_events().await?;

                // Update the last send time
                self.update_last_send_time(user_id_ref).await?;

                debug!("Telemetry sent successfully");
                Ok(())
            }
            Err(e) => {
                warn!("Failed to send telemetry: {:?}", e);
                Err(e)
            }
        }

        */
    }

    /// Clear all telemetry events (used after sending or on logout)
    async fn clear_events(&self) -> Result<()> {
        let conn = self.db.get_connection().await?;
        TelemetryEventModel::delete_all(&conn)
            .await
            .context("Failed to clear telemetry events")?;
        Ok(())
    }

    /// Clear telemetry data on logout
    pub async fn clear_telemetry(&self) -> Result<()> {
        debug!("Clearing telemetry data");
        self.clear_events().await?;
        // Clear cached user_id
        *self.user_id.write().await = None;
        Ok(())
    }
}

#[async_trait::async_trait(?Send)]
impl TelemetryHandler for SqliteTelemetryHandler {
    async fn emit_telemetry(&self, event: TelemetryEvent) -> Result<()> {
        let conn = self.db.get_connection().await?;
        let user_id = self.get_user_id().await;

        TelemetryEventModel::insert(&conn, &event, user_id)
            .await
            .context("Failed to insert event")?;
        Ok(())
    }
}
