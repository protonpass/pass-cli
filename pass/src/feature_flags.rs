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
