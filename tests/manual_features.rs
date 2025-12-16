use growthbook_rust::client::GrowthBookClientBuilder;
use growthbook_rust::client::GrowthBookClientTrait;
use serde_json::json;

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
        .features_json(features_json).unwrap()
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

#[tokio::test]
async fn test_offline_mode_no_api_url_required() {
    // When features are provided directly, api_url and client_key are NOT required.
    // This matches the Python SDK behavior where features can be set without network config.
    let features_json = json!({
        "offline-feature": {
            "defaultValue": true
        },
        "offline-disabled": {
            "defaultValue": false
        }
    });

    let client = GrowthBookClientBuilder::new()
        .features_json(features_json).unwrap()
        .build()
        .await
        .expect("Failed to build client in offline mode");

    assert!(client.is_on("offline-feature", None));
    assert!(!client.is_on("offline-disabled", None));
    assert!(!client.is_on("unknown-feature", None));
}

#[tokio::test]
async fn test_offline_mode_with_rules() {
    let features_json = json!({
        "targeted-feature": {
            "defaultValue": false,
            "rules": [
                {
                    "condition": {"country": "US"},
                    "force": true
                }
            ]
        }
    });

    let client = GrowthBookClientBuilder::new()
        .features_json(features_json).unwrap()
        .build()
        .await
        .expect("Failed to build client");

    // Without attributes, should get default value
    assert!(!client.is_on("targeted-feature", None));
}
