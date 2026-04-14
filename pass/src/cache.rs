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

#[derive(Clone)]
pub struct Cache {
    inner: Arc<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn store<T: 'static + Send + Sync>(&self, _key: T, value: impl Any + Send + Sync) {
        let mut map = self.inner.write().await;
        map.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub async fn get<T: 'static + Send + Sync, V: 'static + Clone>(&self, _key: T) -> Option<V> {
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
        if let Some(boxed) = map.get_mut(&TypeId::of::<T>())
            && let Some(value) = boxed.downcast_mut::<V>()
        {
            updater(value);
        }
    }

    pub async fn update_if_no_value<T, V, E, F, Fut>(&self, _key: T, callback: F) -> Result<V, E>
    where
        T: 'static + Send + Sync,
        V: 'static + Clone + Send + Sync,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<V, E>>,
    {
        let mut guard = self.inner.write().await;

        if let Some(boxed) = guard.get(&TypeId::of::<T>())
            && let Some(casted) = boxed.downcast_ref::<V>().cloned()
        {
            return Ok(casted);
        }

        // If not, compute it using the callback
        match callback().await {
            Ok(value) => {
                // Store it in the cache only on success
                guard.insert(TypeId::of::<T>(), Box::new(value.clone()));
                Ok(value)
            }
            Err(e) => Err(e),
        }
    }
}
