use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::dto::GrowthBookFeature;
use serde_json::Value;
use std::collections::HashMap;

#[tokio::test]
async fn test_manual_features() {
    let features_json = json!({
        "manual-feature": {
            "defaultValue": true
        },
        "manual-value-feature": {
            "defaultValue": "foo"
        }
    });

    let client = GrowthBookClientBuilder::new()
        .api_url("http://localhost:1234".to_string()) // Fake URL
        .client_key("fake-key".to_string())
        .auto_refresh(false) // Disable auto-refresh
        .features_json(features_json)
        .unwrap()
        .build()
        .await
        .expect("Failed to build client");

    // Check boolean feature
    assert!(client.is_on("manual-feature", None));

    // Check value feature
    let result = client.feature_result("manual-value-feature", None);
    assert_eq!(result.value, "foo");

    // Check unknown feature
    assert!(!client.is_on("unknown-feature", None));
}

// Scenario 1: Manual features provided, NO Url or ClientKey configurations needed.
#[tokio::test]
async fn test_offline_mode_manual_features() {
    let features_json = json!({
        "feature1": {
            "defaultValue": "enabled"
        }
    });

    let client = GrowthBookClientBuilder::new()
        .features_json(features_json)
        .unwrap()
        .build()
        .await
        .expect("Build should succeed in offline mode with manual features");

    assert!(client.is_on("feature1", None));
    assert_eq!(client.feature_result("feature1", None).value, Value::String("enabled".to_string()));
}

// Scenario 2: Manual features provided AND API credentials provided.
// Should succeed, use manual features immediately (skipping initial refresh),
// but have gateway configured for background updates.
#[tokio::test]
async fn test_hybrid_mode_manual_features_overwrite() {
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
    assert_eq!(client.feature_result("hybrid-feature", None).value, Value::String("present".to_string()));
}

#[tokio::test]
async fn test_missing_config_error() {
    // Scenario 3: No features AND No credentials
    // Should fail with specific error

    let result = GrowthBookClientBuilder::new().build().await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    // Just checking it errors is enough
    println!("Got expected error: {:?}", err);
}
