use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::GrowthBookAttribute;
use serde_json::json;

// #18: malformed/untrusted feature JSON must never panic the evaluator. Each of
// these used to index out of bounds; now the rule is skipped and the feature
// falls back to its default value.
async fn default_value_for(rule: serde_json::Value) -> serde_json::Value {
    let features = json!({ "f": { "defaultValue": "DEFAULT", "rules": [rule] } });
    let client = GrowthBookClientBuilder::new().features_json(features).unwrap().build().await.unwrap();
    let attrs = GrowthBookAttribute::from(json!({ "id": "abc" })).unwrap();
    client.feature_result("f", Some(attrs)).value
}

#[tokio::test]
async fn more_ranges_than_variations_does_not_panic() {
    // choose_variation can return an index past the (empty) variations vec.
    let v = default_value_for(json!({ "variations": [], "ranges": [[0.0, 1.0]] })).await;
    assert_eq!(v, json!("DEFAULT"));
}

#[tokio::test]
async fn short_ranges_tuple_does_not_panic() {
    let v = default_value_for(json!({ "variations": ["a", "b"], "ranges": [[0.5]] })).await;
    assert_eq!(v, json!("DEFAULT"));
}

#[tokio::test]
async fn short_namespace_tuple_does_not_panic() {
    // Malformed namespace excludes the user (JS parity), so the rule is skipped.
    let v = default_value_for(json!({ "variations": ["a", "b"], "namespace": ["x"] })).await;
    assert_eq!(v, json!("DEFAULT"));
}
