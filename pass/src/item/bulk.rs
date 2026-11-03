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

use crate::domain::{ItemId, ShareId};
use crate::permission::PermissionAction;
use crate::{PassClient, PassClientContext};
use anyhow::{Error, Result, anyhow};
use pass_derive::sdk_export;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

pub(crate) const MAX_ITEMS_PER_TRASH_REQUEST: usize = 100;
pub(crate) const MAX_ITEMS_PER_DELETE_REQUEST: usize = 100;
pub(crate) const MAX_ITEMS_PER_MOVE_REQUEST: usize = 100;
pub(crate) const MAX_ITEMS_PER_CREATE_REQUEST: usize = 100;

#[sdk_export]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemRef {
    pub share_id: ShareId,
    pub item_id: ItemId,
}

#[sdk_export]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BulkItemResult {
    pub share_id: ShareId,
    pub item_id: ItemId,
    pub new_item_id: Option<ItemId>,
    pub error: Option<String>,
}

#[sdk_export]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BulkCreateResult {
    pub share_id: ShareId,
    pub item_id: Option<ItemId>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RevisionedItem {
    pub index: usize,
    pub item_id: ItemId,
    pub revision: u64,
}

pub(crate) type ItemOutcome = Result<Option<ItemId>>;

impl BulkItemResult {
    pub(crate) fn from_outcomes(items: &[ItemRef], outcomes: Vec<ItemOutcome>) -> Vec<Self> {
        items
            .iter()
            .zip(outcomes)
            .map(|(item, outcome)| {
                let (new_item_id, error) = match outcome {
                    Ok(id) => (id, None),
                    Err(e) => (None, Some(format_error(&e))),
                };
                Self {
                    share_id: item.share_id.clone(),
                    item_id: item.item_id.clone(),
                    new_item_id,
                    error,
                }
            })
            .collect()
    }

    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }
}

impl BulkCreateResult {
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }
}

pub(crate) fn format_error(error: &Error) -> String {
    format!("{error:#}")
}

pub(crate) fn fan_out_error(error: Error, count: usize) -> Vec<Error> {
    if count == 0 {
        return Vec::new();
    }
    let message = format_error(&error);
    let mut errors: Vec<Error> = (1..count).map(|_| anyhow!(message.clone())).collect();
    errors.push(error);
    errors
}

pub(crate) fn matched_ids(
    expected: usize,
    response_ids: Vec<String>,
    context: &'static str,
) -> Vec<Result<ItemId>> {
    let mut ids = response_ids.into_iter();
    (0..expected)
        .map(|_| ids.next().map(ItemId::new).ok_or_else(|| anyhow!(context)))
        .collect()
}

pub(crate) fn duplicate_positions(items: &[ItemRef]) -> Vec<Option<usize>> {
    let mut first_seen: HashMap<(&ShareId, &ItemId), usize> = HashMap::new();
    items
        .iter()
        .enumerate()
        .map(
            |(index, item)| match first_seen.entry((&item.share_id, &item.item_id)) {
                Entry::Occupied(entry) => Some(*entry.get()),
                Entry::Vacant(entry) => {
                    entry.insert(index);
                    None
                }
            },
        )
        .collect()
}

pub(crate) fn resolve_duplicates(results: &mut [ItemOutcome], duplicate_of: &[Option<usize>]) {
    for (index, original) in duplicate_of.iter().enumerate() {
        let Some(original) = *original else {
            continue;
        };
        results[index] = match &results[original] {
            Ok(id) => Ok(id.clone()),
            Err(e) => Err(anyhow!(format_error(e))),
        };
    }
}

impl<C: PassClientContext> PassClient<C> {
    pub(crate) async fn guard_item_deletions(
        &self,
        items: &[ItemRef],
        duplicate_of: &[Option<usize>],
        results: &mut [ItemOutcome],
    ) -> Vec<usize> {
        let mut permitted = Vec::with_capacity(items.len());
        for (index, item) in items.iter().enumerate() {
            if duplicate_of[index].is_some() {
                continue;
            }
            match self
                .action_guard(PermissionAction::DeleteItem {
                    share_id: item.share_id.clone(),
                    item_id: item.item_id.clone(),
                })
                .await
            {
                Ok(()) => permitted.push(index),
                Err(e) => results[index] = Err(e),
            }
        }
        permitted
    }
}

pub(crate) fn group_by_share<T>(items: Vec<(ShareId, T)>) -> Vec<(ShareId, Vec<T>)> {
    let mut positions: HashMap<ShareId, usize> = HashMap::new();
    let mut groups: Vec<(ShareId, Vec<T>)> = Vec::new();
    for (share_id, item) in items {
        match positions.get(&share_id) {
            Some(position) => groups[*position].1.push(item),
            None => {
                positions.insert(share_id.clone(), groups.len());
                groups.push((share_id, vec![item]));
            }
        }
    }
    groups
}

pub(crate) fn chunk_owned<T>(items: Vec<T>, size: usize) -> Vec<Vec<T>> {
    let size = size.max(1);
    let mut chunks = Vec::with_capacity(items.len().div_ceil(size));
    let mut iter = items.into_iter();
    loop {
        let chunk: Vec<T> = iter.by_ref().take(size).collect();
        if chunk.is_empty() {
            break;
        }
        chunks.push(chunk);
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SessionInvalidatedError;

    fn share(id: &str) -> ShareId {
        ShareId::new(id.to_string())
    }

    #[test]
    fn group_by_share_keeps_first_seen_order() {
        let groups = group_by_share(vec![
            (share("b"), 1),
            (share("a"), 2),
            (share("b"), 3),
            (share("c"), 4),
            (share("a"), 5),
        ]);
        assert_eq!(
            groups,
            vec![
                (share("b"), vec![1, 3]),
                (share("a"), vec![2, 5]),
                (share("c"), vec![4]),
            ]
        );
    }

    #[test]
    fn group_by_share_empty() {
        assert!(group_by_share::<u32>(vec![]).is_empty());
    }

    #[test]
    fn chunk_owned_splits_at_limit() {
        let chunks = chunk_owned((0..5).collect::<Vec<_>>(), 2);
        assert_eq!(chunks, vec![vec![0, 1], vec![2, 3], vec![4]]);
    }

    #[test]
    fn chunk_owned_exact_multiple() {
        let chunks = chunk_owned((0..4).collect::<Vec<_>>(), 2);
        assert_eq!(chunks, vec![vec![0, 1], vec![2, 3]]);
    }

    #[test]
    fn chunk_owned_empty_and_zero_size() {
        assert!(chunk_owned::<u32>(vec![], 3).is_empty());
        assert_eq!(chunk_owned(vec![1, 2], 0), vec![vec![1], vec![2]]);
    }

    #[test]
    fn from_outcomes_formats_errors_in_order() {
        let items = vec![
            ItemRef {
                share_id: share("s"),
                item_id: ItemId::new("a".to_string()),
            },
            ItemRef {
                share_id: share("s"),
                item_id: ItemId::new("b".to_string()),
            },
        ];
        let results = BulkItemResult::from_outcomes(
            &items,
            vec![
                Ok(Some(ItemId::new("n".to_string()))),
                Err(anyhow!("boom").context("outer")),
            ],
        );
        assert!(results[0].is_success());
        assert_eq!(
            results[0].new_item_id.as_ref().map(|i| i.to_string()),
            Some("n".to_string())
        );
        assert_eq!(results[1].error.as_deref(), Some("outer: boom"));
    }

    #[test]
    fn fan_out_error_keeps_original_for_single() {
        let errors = fan_out_error(Error::new(SessionInvalidatedError), 1);
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0]
                .downcast_ref::<SessionInvalidatedError>()
                .is_some()
        );
        assert!(fan_out_error(anyhow!("x"), 0).is_empty());
        assert_eq!(fan_out_error(anyhow!("x"), 3).len(), 3);
    }

    fn item_ref(share_id: &str, item_id: &str) -> ItemRef {
        ItemRef {
            share_id: share(share_id),
            item_id: ItemId::new(item_id.to_string()),
        }
    }

    #[test]
    fn duplicate_positions_points_to_first_occurrence() {
        let positions = duplicate_positions(&[
            item_ref("s", "a"),
            item_ref("s", "b"),
            item_ref("s", "a"),
            item_ref("t", "a"),
            item_ref("s", "a"),
        ]);
        assert_eq!(positions, vec![None, None, Some(0), None, Some(0)]);
    }

    #[test]
    fn resolve_duplicates_copies_original_outcome() {
        let mut results: Vec<ItemOutcome> = vec![
            Ok(Some(ItemId::new("n".to_string()))),
            Err(anyhow!("boom").context("outer")),
            Ok(None),
            Ok(None),
        ];
        resolve_duplicates(&mut results, &[None, None, Some(0), Some(1)]);
        assert!(
            results[2]
                .as_ref()
                .is_ok_and(|id| id.as_ref().is_some_and(|id| id.to_string() == "n"))
        );
        assert_eq!(
            results[3].as_ref().map_err(format_error).err().as_deref(),
            Some("outer: boom")
        );
    }

    #[test]
    fn matched_ids_flags_missing_responses() {
        let ids = matched_ids(3, vec!["x".to_string(), "y".to_string()], "missing");
        assert_eq!(ids.len(), 3);
        assert!(ids[0].as_ref().is_ok_and(|id| id.to_string() == "x"));
        assert!(ids[1].is_ok());
        assert!(ids[2].is_err());
    }
}
