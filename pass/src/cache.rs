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

use async_lock::RwLock;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub struct Cache {
    inner: Arc<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
    enabled: Arc<AtomicBool>,
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

impl Cache {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            enabled: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub async fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
        if !enabled {
            self.clear().await;
        }
    }

    pub async fn clear(&self) {
        self.inner.write().await.clear();
    }

    pub async fn store<T: 'static + Send + Sync>(&self, _key: T, value: impl Any + Send + Sync) {
        let mut map = self.inner.write().await;
        if !self.is_enabled() {
            return;
        }
        map.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub async fn get<T: 'static + Send + Sync, V: 'static + Clone>(&self, _key: T) -> Option<V> {
        if !self.is_enabled() {
            return None;
        }
        let map = self.inner.read().await;
        map.get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref::<V>().cloned())
    }

    pub async fn delete<T: 'static>(&self, _key: T) {
        let mut map = self.inner.write().await;
        map.remove(&TypeId::of::<T>());
    }

    pub async fn ensure_has_value<T: 'static + Send + Sync, V: 'static + Clone + Send + Sync>(
        &self,
        _key: T,
        default: impl Fn() -> V,
    ) {
        let mut map = self.inner.write().await;
        if !self.is_enabled() {
            return;
        }
        if map.get(&TypeId::of::<T>()).is_none() {
            map.insert(TypeId::of::<T>(), Box::new(default()));
        };
    }

    pub async fn update<T: 'static + Send + Sync, V: 'static + Send + Sync>(
        &self,
        _key: T,
        updater: impl FnOnce(&mut V),
    ) {
        let mut map = self.inner.write().await;
        if !self.is_enabled() {
            return;
        }
        if let Some(boxed) = map.get_mut(&TypeId::of::<T>())
            && let Some(value) = boxed.downcast_mut::<V>()
        {
            updater(value);
        }
    }

    pub async fn with_entry<T, V, R>(
        &self,
        _key: T,
        default: impl Fn() -> V,
        updater: impl FnOnce(&mut V) -> R,
    ) -> Option<R>
    where
        T: 'static + Send + Sync,
        V: 'static + Send + Sync,
    {
        let mut map = self.inner.write().await;
        if !self.is_enabled() {
            let mut value = default();
            return Some(updater(&mut value));
        }
        let entry = map
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(default()));
        entry.downcast_mut::<V>().map(updater)
    }

    pub async fn update_if_no_value<T, V, E, F, Fut>(&self, _key: T, callback: F) -> Result<V, E>
    where
        T: 'static + Send + Sync,
        V: 'static + Clone + Send + Sync,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<V, E>>,
    {
        let mut guard = self.inner.write().await;

        if !self.is_enabled() {
            drop(guard);
            return callback().await;
        }

        if let Some(boxed) = guard.get(&TypeId::of::<T>())
            && let Some(casted) = boxed.downcast_ref::<V>().cloned()
        {
            return Ok(casted);
        }

        match callback().await {
            Ok(value) => {
                guard.insert(TypeId::of::<T>(), Box::new(value.clone()));
                Ok(value)
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct KeyA;
    struct KeyB;

    #[tokio::test]
    async fn enabled_by_default_and_stores() {
        let cache = Cache::new();
        assert!(cache.is_enabled());
        cache.store(KeyA, 5u32).await;
        assert_eq!(cache.get::<_, u32>(KeyA).await, Some(5));
    }

    #[tokio::test]
    async fn disabled_get_returns_none_and_store_is_noop() {
        let cache = Cache::new();
        cache.store(KeyA, 5u32).await;
        cache.set_enabled(false).await;
        assert_eq!(cache.get::<_, u32>(KeyA).await, None);
        cache.store(KeyB, 7u32).await;
        cache.set_enabled(true).await;
        assert_eq!(cache.get::<_, u32>(KeyA).await, None);
        assert_eq!(cache.get::<_, u32>(KeyB).await, None);
    }

    #[tokio::test]
    async fn disabled_ensure_update_and_delete_do_nothing() {
        let cache = Cache::new();
        cache.set_enabled(false).await;
        cache.ensure_has_value(KeyA, || 1u32).await;
        cache.update(KeyA, |v: &mut u32| *v += 1).await;
        cache.delete(KeyA).await;
        cache.set_enabled(true).await;
        assert_eq!(cache.get::<_, u32>(KeyA).await, None);
    }

    #[tokio::test]
    async fn disabled_with_entry_runs_on_default_without_keeping() {
        let cache = Cache::new();
        cache.set_enabled(false).await;
        let first = cache
            .with_entry(
                KeyA,
                || 10u32,
                |v| {
                    *v += 1;
                    *v
                },
            )
            .await;
        let second = cache
            .with_entry(
                KeyA,
                || 10u32,
                |v| {
                    *v += 1;
                    *v
                },
            )
            .await;
        assert_eq!(first, Some(11));
        assert_eq!(second, Some(11));
    }

    #[tokio::test]
    async fn enabled_with_entry_keeps_value() {
        let cache = Cache::new();
        let first = cache
            .with_entry(
                KeyA,
                || 10u32,
                |v| {
                    *v += 1;
                    *v
                },
            )
            .await;
        let second = cache
            .with_entry(
                KeyA,
                || 10u32,
                |v| {
                    *v += 1;
                    *v
                },
            )
            .await;
        assert_eq!(first, Some(11));
        assert_eq!(second, Some(12));
    }

    #[tokio::test]
    async fn update_if_no_value_always_calls_back_when_disabled() {
        let cache = Cache::new();
        cache.set_enabled(false).await;
        for expected in [1u32, 2] {
            let res: Result<u32, ()> = cache
                .update_if_no_value(KeyA, || async move { Ok(expected) })
                .await;
            assert_eq!(res, Ok(expected));
        }
        cache.set_enabled(true).await;
        assert_eq!(cache.get::<_, u32>(KeyA).await, None);
    }

    #[tokio::test]
    async fn update_if_no_value_caches_when_enabled() {
        let cache = Cache::new();
        let a: Result<u32, ()> = cache.update_if_no_value(KeyA, || async { Ok(1) }).await;
        let b: Result<u32, ()> = cache.update_if_no_value(KeyA, || async { Ok(2) }).await;
        assert_eq!(a, Ok(1));
        assert_eq!(b, Ok(1));
    }

    #[tokio::test]
    async fn clones_share_enabled_flag() {
        let cache = Cache::new();
        let clone = cache.clone();
        clone.set_enabled(false).await;
        assert!(!cache.is_enabled());
        cache.set_enabled(true).await;
        assert!(clone.is_enabled());
    }

    #[tokio::test]
    async fn clear_removes_everything() {
        let cache = Cache::new();
        cache.store(KeyA, 1u32).await;
        cache.store(KeyB, 2u32).await;
        cache.clear().await;
        assert_eq!(cache.get::<_, u32>(KeyA).await, None);
        assert_eq!(cache.get::<_, u32>(KeyB).await, None);
    }
}
