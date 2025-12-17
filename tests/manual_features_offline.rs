use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::dto::GrowthBookFeature;
use serde_json::Value;
use std::collections::HashMap;

#[tokio::test]
async fn test_offline_mode_manual_features() {
    // Scenario 1: Manual features provided, NO API credentials
    // Should succeed and use manual features. Refresh should be skipped (implied by no network error).

    let mut features = HashMap::new();
    features.insert(
        "offline-feature".to_string(),
        GrowthBookFeature {
            default_value: Some(Value::String("enabled".to_string())),
            rules: None,
        },
    );

    let client = GrowthBookClientBuilder::new()
        .features(features)
        .build()
        .await
        .expect("Build should succeed in offline mode with manual features");

    assert!(client.is_on("offline-feature", None));
    assert_eq!(
        client.feature_result("offline-feature", None).value,
        Value::String("enabled".to_string())
    );
}

#[tokio::test]
async fn test_hybrid_mode_manual_features_overwrite() {
    // Scenario 2: Manual features provided AND API credentials provided.
    // Should succeed, use manual features immediately (skipping initial refresh),
    // but have gateway configured for background updates.

    // We use a fake URL. If refresh() was called, it would fail or hang (depending on timeout).
    // Since we skip refresh on build, this should pass immediately.
    
    let mut features = HashMap::new();
    features.insert(
        "hybrid-feature".to_string(),
        GrowthBookFeature {
            default_value: Some(Value::String("present".to_string())),
            rules: None,
        },
    );

    let client = GrowthBookClientBuilder::new()
        .api_url("http://localhost:9999".to_string()) // unreachable
        .client_key("dummy".to_string())
        .features(features)
        .build()
        .await
        .expect("Build should succeed in hybrid mode");

    // Verify manual features are present immediately
    assert_eq!(
        client.feature_result("hybrid-feature", None).value,
        Value::String("present".to_string())
    );
}

#[tokio::test]
async fn test_missing_config_error() {
    // Scenario 3: No features AND No credentials
    // Should fail with specific error

    let result = GrowthBookClientBuilder::new()
        .build()
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    // Just checking it errors is enough
    println!("Got expected error: {:?}", err);
}
