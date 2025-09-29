use std::fmt;
use std::str::FromStr;
use tracing::Event;
use tracing::field::Visit;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const MUON_LOG_LEVEL_ENV: &str = "MUON_LOG_LEVEL";
const PASS_LOG_LEVEL_ENV: &str = "PASS_LOG_LEVEL";

fn transform_webauthn_log_message(message: &str) -> String {
    if message.contains("STATUS: Please touch your device.") {
        "Please touch your FIDO2 device now\n".to_string()
    } else {
        message.to_string()
    }
}

struct WebauthnLogFormatter;

impl<S, N> tracing_subscriber::fmt::FormatEvent<S, N> for WebauthnLogFormatter
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        // Extract the message from the event
        let mut message = String::new();
        event.record(&mut MessageVisitor(&mut message));

        // Transform the message if needed
        let transformed_message = transform_webauthn_log_message(&message);

        // Write the transformed message
        write!(writer, "{}", transformed_message)?;

        Ok(())
    }
}

/// Visitor to extract the message from a tracing event
struct MessageVisitor<'a>(&'a mut String);

impl<'a> Visit for MessageVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            *self.0 = format!("{:?}", value);
        }
    }
}

pub fn setup_logs() {
    let subscriber = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_target(false);

    let muon_log_level = match std::env::var(MUON_LOG_LEVEL_ENV) {
        Ok(val) => {
            if val == "off" {
                None
            } else {
                Some(tracing::Level::from_str(&val).expect("invalid MUON_LOG_LEVEL"))
            }
        }
        Err(_) => None,
    };

    let pass_log_level = match std::env::var(PASS_LOG_LEVEL_ENV) {
        Ok(val) => {
            if val == "off" {
                None
            } else {
                Some(tracing::Level::from_str(&val).expect("invalid PASS_LOG_LEVEL"))
            }
        }
        Err(_) => {
            if cfg!(debug_assertions) {
                Some(tracing::Level::DEBUG)
            } else {
                None
            }
        }
    };

    let mut filter = tracing_subscriber::filter::Targets::new().with_default(tracing::Level::ERROR);

    if let Some(log_level) = pass_log_level {
        filter = filter
            .with_target("pass", log_level)
            .with_target("pass_cli", log_level)
            .with_target("pass_domain", log_level)
            .with_target("pass_fs", log_level)
            .with_target("pass_pgp", log_level);
    }

    if let Some(muon_log_level) = muon_log_level {
        filter = filter.with_target("muon", muon_log_level);
    }

    let webauthn_logs = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .event_format(WebauthnLogFormatter)
        .with_filter(
            tracing_subscriber::filter::Targets::new()
                .with_target("webauthn_authenticator_rs", tracing::Level::INFO),
        );

    tracing_subscriber::registry()
        .with(subscriber.with_filter(filter))
        .with(webauthn_logs)
        .init();
}
