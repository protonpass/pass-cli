pub mod event;

use anyhow::{Context, Result};
use pass::PassClient;
use pass_db::{ActivityTimeModel, DatabaseManager, TelemetryEventModel};
use pass_domain::{TelemetryEvent, TelemetryEventData, TelemetryHandler};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

// 1 day
const TELEMETRY_SEND_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const TELEMETRY_SENT_ACTIVITY: &str = "telemetry_sent";
const TELEMETRY_DISABLED_ENV_VAR: &str = "PROTON_PASS_DISABLE_TELEMETRY";

#[derive(Clone)]
pub struct SqliteTelemetryHandler {
    db: DatabaseManager,
    user_id: Arc<RwLock<Option<String>>>,
    telemetry_enabled: bool,
}

fn is_telemetry_enabled() -> bool {
    std::env::var(TELEMETRY_DISABLED_ENV_VAR).is_err()
}

impl SqliteTelemetryHandler {
    pub fn new(db: DatabaseManager) -> Self {
        Self {
            db,
            user_id: Arc::new(RwLock::new(None)),
            telemetry_enabled: is_telemetry_enabled(),
        }
    }

    pub async fn set_user_id(&self, user_id: Option<String>) {
        *self.user_id.write().await = user_id;
    }

    pub async fn send_telemetry_if_needed(&self, user_id: Option<String>, client: &PassClient) {
        if let Some(user_id) = user_id
            && let Err(e) = self
                .internal_send_telemetry_if_needed(&user_id, client)
                .await
        {
            warn!("Error sending telemetry data: {e:#}");
        }
    }

    async fn internal_send_telemetry_if_needed(
        &self,
        user_id: &str,
        client: &PassClient,
    ) -> Result<()> {
        let needs_to_send = self
            .needs_to_send_telemetry(user_id)
            .await
            .context("Error checking whether we need to send telemetry")?;
        if !needs_to_send {
            debug!("No need to send telemetry data");
            return Ok(());
        }

        let should_send_telemetry = self.should_send_telemetry(client).await;

        if should_send_telemetry {
            debug!("Fetching telemetry events for user_id {user_id}");
            let events = self
                .get_telemetry_events(user_id)
                .await
                .context("Error getting telemetry events")?;

            debug!("Sending telemetry events for user_id {user_id}");
            client
                .send_telemetry_events(events)
                .await
                .context("Error sending telemetry events")?;
        } else {
            debug!("Sending of telemetry data is disabled");
        }

        debug!("Removing local telemetry events for user_id {user_id}");
        let conn = self
            .db
            .get_connection()
            .await
            .context("Error getting connection")?;
        TelemetryEventModel::delete_by_user_id(&conn, user_id)
            .await
            .context("Error removing telemetry events")?;

        debug!("Updating last telemetry sent time for user_id {user_id}");
        self.update_last_send_time(Some(user_id))
            .await
            .context("Error updating last send time")?;

        Ok(())
    }

    async fn should_send_telemetry(&self, client: &PassClient) -> bool {
        if !self.telemetry_enabled {
            return false;
        }

        match client.get_account_user_settings().await {
            Ok(settings) => settings.telemetry_enabled,
            Err(_) => false,
        }
    }

    async fn get_user_id(&self) -> Option<String> {
        let cached = self.user_id.read().await;
        cached.clone()
    }

    async fn needs_to_send_telemetry(&self, user_id: &str) -> Result<bool> {
        let conn = self.db.get_connection().await?;

        let record = ActivityTimeModel::get(&conn, Some(user_id), TELEMETRY_SENT_ACTIVITY)
            .await
            .context("Failed to check telemetry send time")?;

        match record {
            Some(activity_time) => {
                let elapsed = jiff::Timestamp::now().as_second() - activity_time.timestamp;
                Ok(elapsed >= TELEMETRY_SEND_INTERVAL.as_secs() as i64)
            }
            None => {
                // No record found, insert current time and return false
                let now = jiff::Timestamp::now().as_second();
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

    async fn update_last_send_time(&self, user_id: Option<&str>) -> Result<()> {
        let conn = self.db.get_connection().await?;
        let now = jiff::Timestamp::now().as_second();

        ActivityTimeModel::upsert(
            &conn,
            user_id.map(|s| s.to_string()),
            TELEMETRY_SENT_ACTIVITY,
            now,
        )
        .await
        .context("Failed to update last send time")
    }

    pub async fn get_telemetry_events(&self, user_id: &str) -> Result<Vec<TelemetryEventData>> {
        let conn = self.db.get_connection().await?;
        let events = TelemetryEventModel::get_by_user_id(&conn, user_id)
            .await
            .context("Error retrieving telemetry events")?;

        Ok(events)
    }
}

#[async_trait::async_trait(?Send)]
impl TelemetryHandler for SqliteTelemetryHandler {
    async fn emit_telemetry(&self, event: &dyn TelemetryEvent) -> Result<()> {
        if !self.telemetry_enabled {
            debug!("TelemetryEvent not stored as telemetry is disabled");
            return Ok(());
        }
        let conn = self.db.get_connection().await?;
        let user_id = self.get_user_id().await;

        TelemetryEventModel::insert(&conn, event, user_id)
            .await
            .context("Failed to insert event")?;
        Ok(())
    }
}
