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

use anyhow::Result;
use pass_domain::models::organization_policy::OrganizationInfo;
use rusqlite::{Row, params};

#[derive(Debug, Clone)]
pub struct OrganizationPolicyModel {
    pub user_id: String,
    pub policy_json: String,
    pub updated_at: i64,
}

impl OrganizationPolicyModel {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(OrganizationPolicyModel {
            user_id: row.get("user_id")?,
            policy_json: row.get("policy_json")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub async fn get(db: &crate::DatabaseManager, user_id: &str) -> Result<Option<Self>> {
        let user_id = user_id.to_string();
        let conn = db.get_connection().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT user_id, policy_json, updated_at FROM organization_policies WHERE user_id = ?1",
            )?;
            match stmt.query_row([&user_id], OrganizationPolicyModel::from_row) {
                Ok(entry) => Ok(Some(entry)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(anyhow::Error::from(e)),
            }
        })
        .await?
    }

    pub async fn upsert(
        db: &crate::DatabaseManager,
        user_id: &str,
        policy: &OrganizationInfo,
    ) -> Result<()> {
        let user_id = user_id.to_string();
        let policy_json = serde_json::to_string(policy)
            .map_err(|e| anyhow::anyhow!("Error serializing organization policy: {}", e))?;
        let updated_at = jiff::Timestamp::now().as_second();
        let conn = db.get_connection().await?;
        conn.interact(move |conn| {
            conn.execute(
                "INSERT INTO organization_policies (user_id, policy_json, updated_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(user_id) DO UPDATE SET
                 policy_json = excluded.policy_json,
                 updated_at = excluded.updated_at",
                params![user_id, policy_json, updated_at],
            )?;
            Ok(())
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::create_test_db;

    fn test_policy() -> OrganizationInfo {
        OrganizationInfo {
            can_update: true,
            settings: pass_domain::models::organization_policy::OrganizationSettings {
                share_mode: pass_domain::models::organization_policy::OrganizationShareMode::Unrestricted,
                share_accept_mode: pass_domain::models::organization_policy::OrganizationShareMode::Unrestricted,
                item_share_mode: 0,
                public_link_mode: 0,
                force_lock_seconds: 0,
                export_mode: pass_domain::models::organization_policy::OrganizationExportMode::Unrestricted,
                password_policy: pass_domain::models::organization_policy::OrganizationPasswordPolicy {
                    random_password_allowed: true,
                    random_password_min_length: Some(4),
                    random_password_max_length: Some(64),
                    random_password_must_include_numbers: Some(true),
                    random_password_must_include_symbols: Some(true),
                    random_password_must_include_uppercase: Some(true),
                    memorable_password_allowed: true,
                    memorable_password_min_words: Some(4),
                    memorable_password_max_words: Some(8),
                    memorable_password_must_capitalize: Some(true),
                    memorable_password_must_include_numbers: Some(true),
                },
                vault_create_mode: pass_domain::models::organization_policy::OrganizationVaultCreateMode::OnlyOrgAdmins,
                alias_create_mode: pass_domain::models::organization_policy::OrganizationAliasCreateMode::Nobody,
            },
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_returns_none_when_absent() {
        let db = test_db!();
        let result = OrganizationPolicyModel::get(&db, "user1").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_upsert_and_get() {
        let db = test_db!();
        let before = jiff::Timestamp::now().as_second();
        let policy = test_policy();
        OrganizationPolicyModel::upsert(&db, "user1", &policy)
            .await
            .unwrap();
        let entry = OrganizationPolicyModel::get(&db, "user1")
            .await
            .unwrap()
            .unwrap();
        let policy_json = serde_json::to_string(&policy).unwrap();
        assert_eq!(entry.policy_json, policy_json);
        assert!(entry.updated_at >= before);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_upsert_overwrites() {
        let db = test_db!();
        let mut policy1 = test_policy();
        policy1.can_update = true;
        let mut policy2 = test_policy();
        policy2.can_update = false;
        OrganizationPolicyModel::upsert(&db, "user1", &policy1)
            .await
            .unwrap();
        OrganizationPolicyModel::upsert(&db, "user1", &policy2)
            .await
            .unwrap();
        let entry = OrganizationPolicyModel::get(&db, "user1")
            .await
            .unwrap()
            .unwrap();
        let policy_json = serde_json::to_string(&policy2).unwrap();
        assert_eq!(entry.policy_json, policy_json);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_policies_are_isolated_per_user() {
        let db = test_db!();
        let mut policy1 = test_policy();
        policy1.can_update = true;
        let mut policy2 = test_policy();
        policy2.can_update = false;
        OrganizationPolicyModel::upsert(&db, "user1", &policy1)
            .await
            .unwrap();
        OrganizationPolicyModel::upsert(&db, "user2", &policy2)
            .await
            .unwrap();
        let entry1 = OrganizationPolicyModel::get(&db, "user1")
            .await
            .unwrap()
            .unwrap();
        let entry2 = OrganizationPolicyModel::get(&db, "user2")
            .await
            .unwrap()
            .unwrap();
        let policy1_json = serde_json::to_string(&policy1).unwrap();
        let policy2_json = serde_json::to_string(&policy2).unwrap();
        assert_eq!(entry1.policy_json, policy1_json);
        assert_eq!(entry2.policy_json, policy2_json);
    }
}
