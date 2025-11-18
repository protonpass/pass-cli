use crate::PassClient;
use crate::common::CodeResponse;
use anyhow::{Context, Result};
use muon::POST;
use pass_domain::{TelemetryEvent, TelemetryEventData};
use std::collections::HashMap;

const EVENT_CHUNK_SIZE: usize = 500;
const MEASUREMENT_GROUP: &str = "pass.any.user_actions";
const PLAN_NAME_KEY: &str = "user_tier";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SendTelemetryRequest {
    #[serde(rename = "EventInfo")]
    event_info: Vec<EventInfo>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct EventInfo {
    #[serde(rename = "MeasurementGroup")]
    measurement_group: String,
    #[serde(rename = "Event")]
    event: String,
    #[serde(rename = "Values")]
    values: HashMap<String, String>,
    #[serde(rename = "Dimensions")]
    dimensions: HashMap<String, String>,
}

impl PassClient {
    // Convenience method to emit telemetry events.
    // Failures are logged but not propagated to avoid breaking operations.
    pub async fn emit_telemetry(&self, event: &dyn TelemetryEvent) {
        match self
            .client_features
            .get_telemetry_handler()
            .await
            .emit_telemetry(event)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                warn!("Failed to emit telemetry event: {:?}", e);
            }
        }
    }

    pub async fn send_telemetry_events(&self, events: Vec<TelemetryEventData>) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }
        let plan = self
            .get_user_access()
            .await
            .context("Error getting user access")?
            .plan
            .internal_name;

        let mut extra_dimensions = Self::get_os_info();
        extra_dimensions.insert(PLAN_NAME_KEY.to_string(), plan);

        let chunks = events.chunks(EVENT_CHUNK_SIZE);
        for chunk in chunks {
            self.send_telemetry_chunk(&extra_dimensions, chunk).await?;
        }
        Ok(())
    }

    async fn send_telemetry_chunk(
        &self,
        extra_dimensions: &HashMap<String, String>,
        chunk: &[TelemetryEventData],
    ) -> Result<()> {
        let body = Self::build_request(extra_dimensions, chunk);
        let req = POST!("/data/v1/stats/multiple")
            .body_json(body)
            .context("Error creating telemetry request")?;

        let res = self.send(req).await?;
        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        Ok(())
    }

    fn build_request(
        extra_dimensions: &HashMap<String, String>,
        chunk: &[TelemetryEventData],
    ) -> SendTelemetryRequest {
        let mut events = Vec::new();

        for event in chunk {
            let event_info = Self::build_event(extra_dimensions, event);
            events.push(event_info);
        }

        SendTelemetryRequest { event_info: events }
    }

    fn build_event(
        extra_dimensions: &HashMap<String, String>,
        event: &TelemetryEventData,
    ) -> EventInfo {
        let mut dimensions = event.dimensions.clone();
        for (name, value) in extra_dimensions {
            dimensions.insert(name.to_string(), value.to_string());
        }

        EventInfo {
            measurement_group: MEASUREMENT_GROUP.to_string(),
            event: event.event_type.clone(),
            values: HashMap::new(), // unused
            dimensions,
        }
    }

    fn get_os_info() -> HashMap<String, String> {
        let mut os_info: HashMap<String, String> = HashMap::new();
        os_info.insert("os".to_string(), std::env::consts::OS.to_string());
        os_info.insert("arch".to_string(), std::env::consts::ARCH.to_string());
        os_info
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;

    #[muon::test(scheme(HTTP))]
    async fn test_send_telemetry_with_zero_events(server: Arc<Server>) {
        let client = server.pass_client().await;

        // Setup handler that should not be called for telemetry
        let handled = server.handler("/data/v1/stats/multiple", |_| {
            success(CodeResponse { code: 1000 })
        });

        // Send empty event list
        let events: Vec<TelemetryEventData> = vec![];
        let result = client.send_telemetry_events(events).await;

        // Should succeed without sending telemetry
        assert!(result.is_ok(), "Should succeed with zero events");

        // The telemetry endpoint should not be hit
        assert_not_hit!(handled);
    }

    #[muon::test(scheme(HTTP))]
    async fn test_send_telemetry_with_single_event(server: Arc<Server>) {
        let client = server.pass_client().await;

        let handled = server.handler("/data/v1/stats/multiple", |_| {
            success(CodeResponse { code: 1000 })
        });

        let recorder = server.new_recorder();

        // Create a single telemetry event
        let mut dimensions = HashMap::new();
        dimensions.insert("action".to_string(), "test_action".to_string());
        dimensions.insert("source".to_string(), "cli".to_string());

        let events = vec![TelemetryEventData {
            event_type: "test.event".to_string(),
            dimensions: dimensions.clone(),
            user_id: None,
            timestamp: 1234567890,
        }];

        let result = client.send_telemetry_events(events).await;
        assert!(result.is_ok(), "Should successfully send single event");

        assert_hit!(handled);

        // Verify the request payload
        let req: SendTelemetryRequest = last_request!(recorder);
        assert_eq!(1, req.event_info.len(), "Should have 1 event in payload");

        let event_info = &req.event_info[0];
        assert_eq!(MEASUREMENT_GROUP, event_info.measurement_group);
        assert_eq!("test.event", event_info.event);
        assert!(event_info.values.is_empty(), "Values should be empty");

        // Verify dimensions include both original and plan
        assert_eq!("test_action", event_info.dimensions.get("action").unwrap());
        assert_eq!("cli", event_info.dimensions.get("source").unwrap());
        assert!(
            event_info.dimensions.contains_key(PLAN_NAME_KEY),
            "Should include user_tier dimension"
        );
        assert_eq!(
            TEST_PLAN_NAME,
            event_info.dimensions.get(PLAN_NAME_KEY).unwrap()
        );
    }

    #[muon::test(scheme(HTTP))]
    async fn test_send_telemetry_with_two_events(server: Arc<Server>) {
        let client = server.pass_client().await;

        let handled = server.handler("/data/v1/stats/multiple", |_| {
            success(CodeResponse { code: 1000 })
        });

        let recorder = server.new_recorder();

        // Create two telemetry events with different data
        let mut dimensions1 = HashMap::new();
        dimensions1.insert("item_type".to_string(), "login".to_string());
        dimensions1.insert("has_totp".to_string(), "true".to_string());

        let mut dimensions2 = HashMap::new();
        dimensions2.insert("item_type".to_string(), "note".to_string());
        dimensions2.insert("has_attachments".to_string(), "false".to_string());

        let events = vec![
            TelemetryEventData {
                event_type: "item.create.login".to_string(),
                dimensions: dimensions1.clone(),
                user_id: None,
                timestamp: 1234567890,
            },
            TelemetryEventData {
                event_type: "item.create.note".to_string(),
                dimensions: dimensions2.clone(),
                user_id: None,
                timestamp: 1234567891,
            },
        ];

        let result = client.send_telemetry_events(events).await;
        assert!(result.is_ok(), "Should successfully send two events");

        assert_hit!(handled);

        // Verify the request payload
        let req: SendTelemetryRequest = last_request!(recorder);
        assert_eq!(2, req.event_info.len(), "Should have 2 events in payload");

        // Verify first event
        let event1 = &req.event_info[0];
        assert_eq!(MEASUREMENT_GROUP, event1.measurement_group);
        assert_eq!("item.create.login", event1.event);
        assert_eq!("login", event1.dimensions.get("item_type").unwrap());
        assert_eq!("true", event1.dimensions.get("has_totp").unwrap());
        assert!(
            event1.dimensions.contains_key(PLAN_NAME_KEY),
            "First event should include user_tier"
        );

        // Verify second event
        let event2 = &req.event_info[1];
        assert_eq!(MEASUREMENT_GROUP, event2.measurement_group);
        assert_eq!("item.create.note", event2.event);
        assert_eq!("note", event2.dimensions.get("item_type").unwrap());
        assert_eq!("false", event2.dimensions.get("has_attachments").unwrap());
        assert!(
            event2.dimensions.contains_key(PLAN_NAME_KEY),
            "Second event should include user_tier"
        );
    }

    #[muon::test(scheme(HTTP))]
    async fn test_send_telemetry_with_plus_plan(server: Arc<Server>) {
        let client = server.pass_client_with_plan(PlanType::Plus).await;

        let handled = server.handler("/data/v1/stats/multiple", |_| {
            success(CodeResponse { code: 1000 })
        });

        let recorder = server.new_recorder();

        // Create a telemetry event
        let mut dimensions = HashMap::new();
        dimensions.insert("feature".to_string(), "advanced".to_string());

        let events = vec![TelemetryEventData {
            event_type: "feature.used".to_string(),
            dimensions,
            user_id: None,
            timestamp: 1234567890,
        }];

        let result = client.send_telemetry_events(events).await;
        assert!(result.is_ok(), "Should successfully send event");

        assert_hit!(handled);

        // Verify the plan is correctly set to the test plan
        let req: SendTelemetryRequest = last_request!(recorder);
        assert_eq!(1, req.event_info.len());

        let event_info = &req.event_info[0];
        assert_eq!(
            TEST_PLAN_NAME,
            event_info.dimensions.get(PLAN_NAME_KEY).unwrap(),
            "Should have test plan in dimensions"
        );
    }

    #[muon::test(scheme(HTTP))]
    async fn test_send_telemetry_with_empty_dimensions(server: Arc<Server>) {
        let client = server.pass_client().await;

        let handled = server.handler("/data/v1/stats/multiple", |_| {
            success(CodeResponse { code: 1000 })
        });

        let recorder = server.new_recorder();

        // Create event with no dimensions
        let events = vec![TelemetryEventData {
            event_type: "simple.event".to_string(),
            dimensions: HashMap::new(),
            user_id: None,
            timestamp: 1234567890,
        }];

        let result = client.send_telemetry_events(events).await;
        assert!(
            result.is_ok(),
            "Should successfully send event with no extra dimensions"
        );

        assert_hit!(handled);

        // Verify the request payload
        let req: SendTelemetryRequest = last_request!(recorder);
        assert_eq!(1, req.event_info.len());

        let event_info = &req.event_info[0];
        assert_eq!("simple.event", event_info.event);

        // Should have the plan dimension + 2 for the OS
        assert_eq!(
            3,
            event_info.dimensions.len(),
            "Should only have user_tier dimension + OS dimensions"
        );
        assert!(event_info.dimensions.contains_key(PLAN_NAME_KEY));
    }

    #[test]
    fn test_build_request_structure() {
        let mut dimensions1 = HashMap::new();
        dimensions1.insert("key1".to_string(), "value1".to_string());

        let mut dimensions2 = HashMap::new();
        dimensions2.insert("key2".to_string(), "value2".to_string());

        let events = vec![
            TelemetryEventData {
                event_type: "event.one".to_string(),
                dimensions: dimensions1,
                user_id: None,
                timestamp: 1000,
            },
            TelemetryEventData {
                event_type: "event.two".to_string(),
                dimensions: dimensions2,
                user_id: None,
                timestamp: 2000,
            },
        ];

        let request = PassClient::build_request(&HashMap::new(), &events);

        assert_eq!(2, request.event_info.len());
        assert_eq!(MEASUREMENT_GROUP, request.event_info[0].measurement_group);
        assert_eq!("event.one", request.event_info[0].event);
        assert_eq!("event.two", request.event_info[1].event);
    }

    #[test]
    fn test_build_event_adds_plan_dimension() {
        let mut event_dimensions = HashMap::new();
        event_dimensions.insert("custom_key".to_string(), "custom_value".to_string());

        let event_data = TelemetryEventData {
            event_type: "test.event".to_string(),
            dimensions: event_dimensions.clone(),
            user_id: None,
            timestamp: 1234567890,
        };

        let plan = "business";
        let mut extra_dimensions = HashMap::new();
        extra_dimensions.insert(PLAN_NAME_KEY.to_string(), plan.to_string());
        let event_info = PassClient::build_event(&extra_dimensions, &event_data);

        // Verify structure
        assert_eq!(MEASUREMENT_GROUP, event_info.measurement_group);
        assert_eq!("test.event", event_info.event);
        assert!(event_info.values.is_empty());

        // Verify dimensions include both original and plan
        assert_eq!(2, event_info.dimensions.len());
        assert_eq!(
            "custom_value",
            event_info.dimensions.get("custom_key").unwrap()
        );
        assert_eq!(
            "business",
            event_info.dimensions.get(PLAN_NAME_KEY).unwrap()
        );
    }
}
