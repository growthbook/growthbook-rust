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
