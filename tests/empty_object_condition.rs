use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::GrowthBookAttribute;
use serde_json::json;

// Spec 0.11.0: a condition value of `{}` has no operators, so it passes for
// every value of the attribute, including a missing one.
#[tokio::test]
async fn empty_object_condition_always_passes() {
    let features_json = json!({
        "eq-empty-object": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": {} }, "force": true }]
        }
    });

    let client = GrowthBookClientBuilder::new().features_json(features_json).unwrap().build().await.expect("Failed to build client");

    let attrs = |value: serde_json::Value| GrowthBookAttribute::from(json!({ "x": value })).unwrap();

    for value in [json!({}), json!({ "a": 1 }), json!(null), json!(0), json!("x"), json!([])] {
        assert!(client.is_on("eq-empty-object", Some(attrs(value.clone()))), "condition {{x: {{}}}} must match x = {value}");
    }

    let missing = GrowthBookAttribute::from(json!({ "y": 1 })).unwrap();
    assert!(client.is_on("eq-empty-object", Some(missing)), "condition {{x: {{}}}} must match a missing attribute");
}
