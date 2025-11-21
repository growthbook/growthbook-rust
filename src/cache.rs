use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::dto::GrowthBookResponse;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait FeatureCache: Send + Sync {
    fn get(&self, key: &str) -> BoxFuture<'_, Option<GrowthBookResponse>>;
    fn set(&self, key: &str, value: GrowthBookResponse) -> BoxFuture<'_, ()>;
}

#[derive(Clone)]
pub struct InMemoryCache {
    cache: Arc<RwLock<HashMap<String, (GrowthBookResponse, Instant)>>>,
    ttl: Duration,
}

impl InMemoryCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }
}

impl FeatureCache for InMemoryCache {
    fn get(&self, key: &str) -> BoxFuture<'_, Option<GrowthBookResponse>> {
        let key = key.to_string();
        let cache = self.cache.clone();
        Box::pin(async move {
            let cache = cache.read().unwrap();
            if let Some((value, expires_at)) = cache.get(&key) {
                if Instant::now() < *expires_at {
                    return Some(value.clone());
                }
            }
            None
        })
    }

    fn set(&self, key: &str, value: GrowthBookResponse) -> BoxFuture<'_, ()> {
        let key = key.to_string();
        let cache = self.cache.clone();
        let ttl = self.ttl;
        Box::pin(async move {
            let mut cache = cache.write().unwrap();
            cache.insert(key, (value, Instant::now() + ttl));
        })
    }
}
