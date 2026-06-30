use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::GrowthBookAttribute;
use serde_json::json;

// R5: `$inGroup`/`$notInGroup` resolve a saved-group id from the context and
// test membership with type-strict equality. Driven through the public client
// builder, including the `saved_groups(...)` manual-load path.
#[tokio::test]
async fn in_group_and_not_in_group_membership() {
    let features = json!({
        "in-test": {
            "defaultValue": false,
            "rules": [{ "condition": { "id": { "$inGroup": "group_id" } }, "force": true }]
        },
        "notin-test": {
            "defaultValue": false,
            "rules": [{ "condition": { "id": { "$notInGroup": "group_id" } }, "force": true }]
        },
        "unknown-group": {
            "defaultValue": false,
            "rules": [{ "condition": { "id": { "$notInGroup": "missing" } }, "force": true }]
        }
    });

    // group_id = [1, "2", 3] — a mix of integers and a string, to prove the
    // membership test is type-aware.
    let saved_groups = json!({ "group_id": [1, "2", 3] });

    let client = GrowthBookClientBuilder::new()
        .features_json(features)
        .unwrap()
        .saved_groups(saved_groups)
        .build()
        .await
        .expect("Failed to build client");

    let attrs = |id: serde_json::Value| Some(GrowthBookAttribute::from(json!({ "id": id })).unwrap());

    // Member (integer 1) → inGroup true, notInGroup false.
    assert!(client.is_on("in-test", attrs(json!(1))), "1 is a member");
    assert!(!client.is_on("notin-test", attrs(json!(1))), "1 is a member");

    // Non-member (integer 5) → inGroup false, notInGroup true.
    assert!(!client.is_on("in-test", attrs(json!(5))), "5 is not a member");
    assert!(client.is_on("notin-test", attrs(json!(5))), "5 is not a member");

    // Type-aware: string "2" matches the string member "2".
    assert!(client.is_on("in-test", attrs(json!("2"))), "string \"2\" matches member \"2\"");

    // Type-aware: string "3" does NOT match the integer member 3.
    assert!(!client.is_on("in-test", attrs(json!("3"))), "string \"3\" must not match integer member 3");

    // Unknown group id → notInGroup is true.
    assert!(client.is_on("unknown-group", attrs(json!(1))), "unknown group → notInGroup is true");
}
