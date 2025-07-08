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
        if let Some(boxed) = map.get_mut(&TypeId::of::<T>()) {
            if let Some(value) = boxed.downcast_mut::<V>() {
                updater(value);
            }
        }
    }
}
