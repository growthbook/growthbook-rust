use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::GrowthBookAttribute;
use serde_json::json;

// R4: an empty object `{}` has JS type "object", not "null". Previously the
// `From<Value>` conversion collapsed both `{}` and `null` into the same value,
// so `{x: {$type: "object"}}` against `{x: {}}` returned false.
#[tokio::test]
async fn empty_object_is_typed_object_not_null() {
    let features_json = json!({
        "is-object": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": { "$type": "object" } }, "force": true }]
        },
        "is-null": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": { "$type": "null" } }, "force": true }]
        }
    });

    let client = GrowthBookClientBuilder::new().features_json(features_json).unwrap().build().await.expect("Failed to build client");

    // {x: {}} → type "object"
    let empty_obj = GrowthBookAttribute::from(json!({ "x": {} })).unwrap();
    assert!(client.is_on("is-object", Some(empty_obj.clone())), "empty object must be $type object");
    assert!(!client.is_on("is-null", Some(empty_obj)), "empty object must NOT be $type null");

    // {x: null} → type "null" (still correct after the change)
    let null_val = GrowthBookAttribute::from(json!({ "x": null })).unwrap();
    assert!(client.is_on("is-null", Some(null_val.clone())), "null must be $type null");
    assert!(!client.is_on("is-object", Some(null_val)), "null must NOT be $type object");
}
