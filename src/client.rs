use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use tokio::time::sleep;
use tracing::error;

use crate::cache::{FeatureCache, InMemoryCache};
use crate::dto::GrowthBookResponse;
use crate::env::Environment;
use crate::error::GrowthbookError;
use crate::gateway::GrowthbookGateway;
use crate::growthbook::GrowthBook;
use crate::model_public::{ExperimentResult, FeatureResult, GrowthBookAttribute};

pub type OnFeatureUsageCallback = Arc<dyn Fn(String, FeatureResult) + Send + Sync>;
pub type OnExperimentViewedCallback = Arc<dyn Fn(ExperimentResult) + Send + Sync>;
pub type OnRefreshCallback = Arc<dyn Fn() + Send + Sync>; // Keeping it simple for now, maybe pass features later if needed

#[derive(Clone)]
pub struct GrowthBookClient {
    pub gb: Arc<RwLock<GrowthBook>>,
    pub cache: Option<Arc<dyn FeatureCache>>,
    gateway: Option<Arc<GrowthbookGateway>>,
    auto_refresh: bool,
    refresh_interval: Duration,
    pub on_feature_usage: Option<OnFeatureUsageCallback>,
    pub on_experiment_viewed: Option<OnExperimentViewedCallback>,
    pub on_refresh: Vec<OnRefreshCallback>,
}

impl Debug for GrowthBookClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrowthBookClient")
            .field("gb", &self.gb)
            .field("auto_refresh", &self.auto_refresh)
            .field("refresh_interval", &self.refresh_interval)
            .field("on_feature_usage", &self.on_feature_usage.is_some())
            .field("on_experiment_viewed", &self.on_experiment_viewed.is_some())
            .field("on_refresh", &self.on_refresh.len())
            .finish()
    }
}

pub struct GrowthBookClientBuilder {
    api_url: Option<String>,
    client_key: Option<String>,
    cache: Option<Arc<dyn FeatureCache>>,
    ttl: Option<Duration>,
    auto_refresh: bool,
    refresh_interval: Option<Duration>,
    attributes: Option<HashMap<String, GrowthBookAttribute>>,
    on_feature_usage: Option<OnFeatureUsageCallback>,
    on_experiment_viewed: Option<OnExperimentViewedCallback>,
    on_refresh: Vec<OnRefreshCallback>,
}

impl GrowthBookClientBuilder {
    pub fn new() -> Self {
        Self {
            api_url: None,
            client_key: None,
            cache: None,
            ttl: None,
            auto_refresh: false,
            refresh_interval: None,
            attributes: None,
            on_feature_usage: None,
            on_experiment_viewed: None,
            on_refresh: Vec::new(),
        }
    }

    pub fn api_url(mut self, api_url: String) -> Self {
        self.api_url = Some(api_url);
        self
    }

    pub fn client_key(mut self, client_key: String) -> Self {
        self.client_key = Some(client_key);
        self
    }

    pub fn cache(mut self, cache: Arc<dyn FeatureCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    pub fn auto_refresh(mut self, auto_refresh: bool) -> Self {
        self.auto_refresh = auto_refresh;
        self
    }

    pub fn refresh_interval(mut self, interval: Duration) -> Self {
        self.refresh_interval = Some(interval);
        self
    }

    pub fn attributes(mut self, attributes: HashMap<String, GrowthBookAttribute>) -> Self {
        self.attributes = Some(attributes);
        self
    }

    pub fn on_feature_usage(mut self, callback: Box<dyn Fn(String, FeatureResult) + Send + Sync>) -> Self {
        self.on_feature_usage = Some(Arc::from(callback));
        self
    }

    pub fn on_experiment_viewed(mut self, callback: Box<dyn Fn(ExperimentResult) + Send + Sync>) -> Self {
        self.on_experiment_viewed = Some(Arc::from(callback));
        self
    }

    pub fn add_on_refresh(mut self, callback: Box<dyn Fn() + Send + Sync>) -> Self {
        self.on_refresh.push(Arc::from(callback));
        self
    }

    pub async fn build(self) -> Result<GrowthBookClient, GrowthbookError> {
        let api_url = self.api_url.ok_or(GrowthbookError::new(crate::error::GrowthbookErrorCode::ConfigError, "API URL is required"))?;
        let client_key = self.client_key.ok_or(GrowthbookError::new(crate::error::GrowthbookErrorCode::ConfigError, "Client Key is required"))?;
        
        let refresh_interval = self.refresh_interval.unwrap_or_else(|| {
            let seconds = Environment::u64_or_default("GB_UPDATE_INTERVAL", 60);
            Duration::from_secs(seconds)
        });

        let gateway = GrowthbookGateway::new(&api_url, &client_key, Duration::from_secs(10))?;
        let gateway_arc = Arc::new(gateway);

        let cache = self.cache.unwrap_or_else(|| {
            let ttl = self.ttl.unwrap_or(Duration::from_secs(60));
            Arc::new(InMemoryCache::new(ttl))
        });

        let client = GrowthBookClient {
            gb: Arc::new(RwLock::new(GrowthBook {
                forced_variations: None,
                features: HashMap::new(),
                attributes: self.attributes,
            })),
            cache: Some(cache),
            gateway: Some(gateway_arc),
            auto_refresh: self.auto_refresh,
            refresh_interval,
            on_feature_usage: self.on_feature_usage,
            on_experiment_viewed: self.on_experiment_viewed,
            on_refresh: self.on_refresh,
        };

        // Initial load
        client.refresh().await;

        if client.auto_refresh {
            client.start_auto_refresh();
        }

        Ok(client)
    }
}

impl GrowthBookClient {
    pub async fn refresh(&self) {
        if let Some(gateway) = &self.gateway {
            let cache_key = "features";
            
            // Try cache first
            if let Some(cache) = &self.cache {
                if let Some(response) = cache.get(cache_key).await {
                    self.update_gb(response);
                    return;
                }
            }

            // Fetch from network
            match gateway.get_features(None).await {
                Ok(response) => {
                    // Update cache
                    if let Some(cache) = &self.cache {
                        cache.set(cache_key, response.clone()).await;
                    }
                    self.update_gb(response);
                },
                Err(e) => {
                    error!("[growthbook-sdk] Failed to fetch features: {:?}", e);
                }
            }
        }
    }

    fn update_gb(&self, response: GrowthBookResponse) {
        let mut writable_config = self.gb.write().expect("problem to create mutex for gb data");
        let attributes = writable_config.attributes.clone();
        *writable_config = GrowthBook {
            forced_variations: response.forced_variations,
            features: response.features,
            attributes,
        };
        
        for callback in &self.on_refresh {
            callback();
        }
    }

    pub fn start_auto_refresh(&self) {
        let client = self.clone();
        tokio::spawn(async move {
            loop {
                sleep(client.refresh_interval).await;
                client.refresh().await;
            }
        });
    }

    // Keep existing new method for backward compatibility, 
    // Old new: spawned a task immediately.
    pub async fn new(
        api_url: &str,
        sdk_key: &str,
        update_interval: Option<Duration>,
        _http_timeout: Option<Duration>,
    ) -> Result<Self, GrowthbookError> {
        let mut builder = GrowthBookClientBuilder::new()
            .api_url(api_url.to_string())
            .client_key(sdk_key.to_string())
            .auto_refresh(true)
            .ttl(Duration::from_secs(0)); // Disable caching for legacy new() to match old behavior
        
        // Legacy new doesn't support setting callbacks, so they default to None
        if let Some(interval) = update_interval {
            builder = builder.refresh_interval(interval);
        }
        
        builder.build().await
    }

    fn read_gb(&self) -> GrowthBook {
        match self.gb.read() {
            Ok(rw_read_guard) => (*rw_read_guard).clone(),
            Err(e) => {
                error!("{}", format!("[growthbook-sdk] problem to reading gb mutex data returning empty {:?}", e));
                GrowthBook {
                    forced_variations: None,
                    features: HashMap::new(),
                    attributes: None,
                }
            },
        }
    }
    fn resolve_feature(
        &self,
        feature_name: &str,
        user_attributes: Option<Vec<GrowthBookAttribute>>,
    ) -> FeatureResult {
        let result = self.read_gb().check(feature_name, &user_attributes);

        // 1. Trigger on_feature_usage only for successful evaluations
        // Exclude: unknownFeature, prerequisite, cyclicPrerequisite
        let invalid_sources = ["unknownFeature", "prerequisite", "cyclicPrerequisite"];
        if !invalid_sources.contains(&result.source.as_str()) {
             if let Some(cb) = &self.on_feature_usage {
                cb(feature_name.to_string(), result.clone());
            }
        }

        // 2. Trigger on_experiment_viewed only if in_experiment is true
        if let Some(cb) = &self.on_experiment_viewed {
            if let Some(experiment_result) = &result.experiment_result {
                if experiment_result.in_experiment {
                    cb(experiment_result.clone());
                }
            }
        }

        result
    }
}

pub trait GrowthBookClientTrait: Debug + Send + Sync {
    fn is_on(
        &self,
        feature_name: &str,
        user_attributes: Option<Vec<GrowthBookAttribute>>,
    ) -> bool;

    fn is_off(
        &self,
        feature_name: &str,
        user_attributes: Option<Vec<GrowthBookAttribute>>,
    ) -> bool;

    fn feature_result(
        &self,
        feature_name: &str,
        user_attributes: Option<Vec<GrowthBookAttribute>>,
    ) -> FeatureResult;

    fn total_features(&self) -> usize;
}

impl GrowthBookClientTrait for GrowthBookClient {
    fn is_on(
        &self,
        feature_name: &str,
        user_attributes: Option<Vec<GrowthBookAttribute>>,
    ) -> bool {
        self.resolve_feature(feature_name, user_attributes).on
    }

    fn is_off(
        &self,
        feature_name: &str,
        user_attributes: Option<Vec<GrowthBookAttribute>>,
    ) -> bool {
        self.resolve_feature(feature_name, user_attributes).off
    }

    fn feature_result(
        &self,
        feature_name: &str,
        user_attributes: Option<Vec<GrowthBookAttribute>>,
    ) -> FeatureResult {
        self.resolve_feature(feature_name, user_attributes)
    }

    fn total_features(&self) -> usize {
        let gb_data = self.read_gb();
        gb_data.features.len()
    }
}
