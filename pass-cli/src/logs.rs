/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

use std::str::FromStr;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const MUON_LOG_LEVEL_ENV: &str = "MUON_LOG_LEVEL";
const PASS_LOG_LEVEL_ENV: &str = "PASS_LOG_LEVEL";

pub fn setup_logs() {
    let subscriber = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
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
    } else {
        filter = filter.with_target("muon", tracing_subscriber::filter::LevelFilter::OFF);
    }

    tracing_subscriber::registry()
        .with(subscriber.with_filter(filter))
        .init();
}
