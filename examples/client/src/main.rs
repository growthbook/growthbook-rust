use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::{FeatureResult, GrowthBookAttribute, GrowthBookAttributeValue};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_url = "<your_growthbook_url>";
    let sdk_key = "some-test-api-key";

    // New Builder Pattern with Auto-Refresh, Caching, and Callbacks
    let gb_client = GrowthBookClientBuilder::new()
        .api_url(api_url.to_string())
        .client_key(sdk_key.to_string())
        .ttl(Duration::from_secs(10))
        .auto_refresh(true)
        .refresh_interval(Duration::from_secs(5))
        .on_feature_usage(Box::new(|key, result| {
            println!("Callback: Feature '{}' evaluated. Value: {:?}", key, result.value);
        }))
        .on_experiment_viewed(Box::new(|experiment_result| {
            println!("Callback: Experiment viewed! Experiment: {}, Variation: {}", 
                experiment_result.key, experiment_result.variation_id);
        }))
        .add_on_refresh(Box::new(|| {
            println!("Callback: Features refreshed from server!");
        }))
        .build()
        .await?;

    // Example loop to demonstrate auto-refresh and context
    loop {
        {
            let feature_name = "rust-sdk-test-feature";
            println!("total features {:?}", gb_client.total_features());

            // Check feature with global context (if set in builder, though we didn't set any here)
            // and override with local attributes
            let mut attributes = HashMap::new();
            attributes.insert("id".to_string(), GrowthBookAttributeValue::String("123".to_string()));
            // Convert HashMap to Vec<GrowthBookAttribute> for the API
            let attr_vec: Vec<GrowthBookAttribute> = attributes
                .into_iter()
                .map(|(k, v)| GrowthBookAttribute::new(k, v))
                .collect();

            // This should trigger on_feature_usage
            let on = gb_client.is_on(feature_name, Some(attr_vec.clone()));
            println!("feature: {} on {:?}", feature_name, on);

            // This should also trigger on_feature_usage
            let feature = gb_client.feature_result(feature_name, Some(attr_vec));
            println!("feature: {} string value {:?}", feature_name, feature.value);

            // Example of typed value retrieval
            let string_feature = gb_client.feature_result(feature_name, None);
            if let Ok(string) = string_feature.value_as::<String>() {
                println!("feature: {} string value {:?}", feature_name, string);
            }
        }

        // Example of creating FeatureResult for testing/mocking
        let test_result = FeatureResult::new(json!("test-value"), true, "test".to_string());
        println!("Test FeatureResult: on={}, value={}", test_result.on, test_result.value);

        sleep(Duration::from_secs(5)).await;
    }
}

#[derive(Deserialize)]
pub struct Custom {
    pub first: String,
    pub second: i64,
}
