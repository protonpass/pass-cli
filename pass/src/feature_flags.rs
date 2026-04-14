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

use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use muon::GET;
use pass_domain::FeatureFlag;
use std::str::FromStr;

struct FeatureFlagsCacheType;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct FeatureFlagsResponse {
    toggles: Vec<FeatureFlagToggle>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct FeatureFlagToggle {
    name: String,
    enabled: bool,
    variant: FeatureFlagVariant,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct FeatureFlagVariant {
    name: String,
    enabled: bool,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn has_feature_flag(&self, feature_flag: FeatureFlag) -> Result<bool> {
        let feature_flags = self
            .get_feature_flags()
            .await
            .context("Error retrieving feature flags")?;
        Ok(feature_flags.contains(&feature_flag))
    }

    async fn get_feature_flags(&self) -> Result<Vec<FeatureFlag>> {
        {
            let cached = self.cache.get(FeatureFlagsCacheType).await;
            if let Some(cached) = cached {
                return Ok(cached);
            }
        }

        let res = self
            .send(GET!("/feature/v2/frontend"))
            .await
            .context("Error requesting feature flags")?;
        let response: FeatureFlagsResponse = assert_response!(res);

        let mut feature_flags = vec![];
        for toggle in response.toggles {
            if toggle.enabled
                && let Ok(ff) = FeatureFlag::from_str(&toggle.name)
            {
                feature_flags.push(ff);
            }
        }

        self.cache
            .store(FeatureFlagsCacheType, feature_flags.clone())
            .await;

        Ok(feature_flags)
    }
}
