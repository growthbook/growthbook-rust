use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::GrowthBookAttribute;
use serde_json::json;

// #1: each filter must hash on its own `attribute` (JS `getHashAttribute(filter.attribute)`,
// default "id"), not a single rule-level attribute. A force rule whose filter is
// keyed on `userId` must include a user who has `userId` (even without `id`).
#[tokio::test]
async fn filter_uses_its_own_attribute() {
    let features = json!({
        "f": {
            "defaultValue": false,
            "rules": [{
                "force": true,
                "filters": [{ "seed": "s", "hashVersion": 2, "attribute": "userId", "ranges": [[0.0, 1.0]] }]
            }]
        }
    });
    let client = GrowthBookClientBuilder::new().features_json(features).unwrap().build().await.unwrap();
    // Has userId, no id. Old code hashed the filter on "id" -> missing -> filtered out.
    let attrs = GrowthBookAttribute::from(json!({ "userId": "u1" })).unwrap();
    assert!(client.is_on("f", Some(attrs)), "filter must hash on its own `attribute` (userId), not \"id\"");
}

// #2: experiment rules must apply their filters. An experiment whose filter
// excludes everyone (empty ranges) must fall through to the default, not bucket
// the user into a variation.
#[tokio::test]
async fn experiment_rule_applies_filters() {
    let features = json!({
        "f": {
            "defaultValue": "default",
            "rules": [{
                "variations": ["a", "b"],
                // Empty range [0,0) excludes every user.
                "filters": [{ "seed": "s", "hashVersion": 2, "ranges": [[0.0, 0.0]] }]
            }]
        }
    });
    let client = GrowthBookClientBuilder::new().features_json(features).unwrap().build().await.unwrap();
    let attrs = GrowthBookAttribute::from(json!({ "id": "abc" })).unwrap();
    let value = client.feature_result("f", Some(attrs)).value;
    assert_eq!(value, json!("default"), "experiment rule must be skipped when its filter excludes the user");
}
