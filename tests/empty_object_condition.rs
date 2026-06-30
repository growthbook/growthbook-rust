use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::GrowthBookAttribute;
use serde_json::json;

// R4: a non-operator condition `{x: {}}` is a deep-equality check. It must match
// only when the user's `x` is itself an empty object `{}`, not when `x` is
// missing. After empty objects started deserializing to `Object([])` (R4), the
// previous branch had the logic inverted (matched on a missing attribute).
#[tokio::test]
async fn empty_object_condition_matches_only_empty_object() {
    let features_json = json!({
        "eq-empty-object": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": {} }, "force": true }]
        }
    });

    let client = GrowthBookClientBuilder::new().features_json(features_json).unwrap().build().await.expect("Failed to build client");

    let attrs = |value: serde_json::Value| GrowthBookAttribute::from(json!({ "x": value })).unwrap();

    // {x: {}} → matches an empty object
    assert!(client.is_on("eq-empty-object", Some(attrs(json!({})))), "condition {{x: {{}}}} must match user x = {{}}");

    // A non-empty object must NOT match (deep equality fails)
    assert!(
        !client.is_on("eq-empty-object", Some(attrs(json!({ "a": 1 })))),
        "condition {{x: {{}}}} must not match a non-empty object"
    );

    // null must NOT match
    assert!(!client.is_on("eq-empty-object", Some(attrs(json!(null)))), "condition {{x: {{}}}} must not match null");

    // A missing attribute must NOT match (this is the inverted-logic regression)
    let missing = GrowthBookAttribute::from(json!({ "y": 1 })).unwrap();
    assert!(!client.is_on("eq-empty-object", Some(missing)), "condition {{x: {{}}}} must not match a missing attribute");
}
