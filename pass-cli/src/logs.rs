use std::str::FromStr;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const MUON_LOG_LEVEL_ENV: &str = "MUON_LOG_LEVEL";

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

    let mut filter = tracing_subscriber::filter::Targets::new()
        .with_default(tracing::Level::ERROR)
        .with_target("pass", tracing::Level::DEBUG)
        .with_target("pass_domain", tracing::Level::DEBUG)
        .with_target("pass-cli", tracing::Level::DEBUG);

    if let Some(muon_log_level) = muon_log_level {
        filter = filter.with_target("muon", muon_log_level);
    }

    tracing_subscriber::registry()
        .with(subscriber.with_filter(filter))
        .init();
}
