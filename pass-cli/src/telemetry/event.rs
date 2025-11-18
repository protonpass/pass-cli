use pass_domain::TelemetryEvent;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct CommandEvent {
    pub command: String,
}

impl CommandEvent {
    pub fn new(command: &str) -> Self {
        Self {
            command: command.to_string(),
        }
    }
}

impl TelemetryEvent for CommandEvent {
    fn event_type(&self) -> String {
        "command.execute".to_string()
    }

    fn dimensions(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("executedCommand".to_string(), self.command.clone());
        map
    }
}
