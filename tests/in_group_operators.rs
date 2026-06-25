use std::collections::HashMap;

use growthbook_rust::dto::GrowthBookFeature;
use growthbook_rust::growthbook::GrowthBook;
use growthbook_rust::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};
use serde_json::json;

// R5: `$inGroup`/`$notInGroup` resolve a saved-group id from the context and
// test membership with type-strict equality. Mirrors the saved-group cases in
// the conformance corpus, exercised through the full feature-evaluation path.
#[tokio::test]
async fn in_group_and_not_in_group_membership() {
    let features: HashMap<String, GrowthBookFeature> = serde_json::from_value(json!({
        "in-test": {
            "defaultValue": false,
            "rules": [{ "condition": { "id": { "$inGroup": "group_id" } }, "force": true }]
        },
        "notin-test": {
            "defaultValue": false,
            "rules": [{ "condition": { "id": { "$notInGroup": "group_id" } }, "force": true }]
        }
    }))
    .unwrap();

    // group_id = [1, "2", 3] — a mix of integers and a string, to prove the
    // membership test is type-aware.
    let mut saved_groups = HashMap::new();
    saved_groups.insert(
        "group_id".to_string(),
        vec![GrowthBookAttributeValue::Int(1), GrowthBookAttributeValue::String("2".to_string()), GrowthBookAttributeValue::Int(3)],
    );

    let gb = GrowthBook {
        forced_variations: None,
        features,
        attributes: None,
        sticky_bucket_service: None,
        saved_groups,
    };

    let in_group = |id: serde_json::Value| gb.check("in-test", &Some(GrowthBookAttribute::from(json!({ "id": id })).unwrap())).on;
    let not_in_group = |id: serde_json::Value| gb.check("notin-test", &Some(GrowthBookAttribute::from(json!({ "id": id })).unwrap())).on;

    // Member (integer 1) → inGroup true, notInGroup false.
    assert!(in_group(json!(1)), "1 is a member");
    assert!(!not_in_group(json!(1)), "1 is a member");

    // Non-member (integer 5) → inGroup false, notInGroup true.
    assert!(!in_group(json!(5)), "5 is not a member");
    assert!(not_in_group(json!(5)), "5 is not a member");

    // Type-aware: string "2" matches the string member "2".
    assert!(in_group(json!("2")), "string \"2\" matches member \"2\"");

    // Type-aware: string "3" does NOT match the integer member 3.
    assert!(!in_group(json!("3")), "string \"3\" must not match integer member 3");

    // Unknown group id → inGroup false, notInGroup true.
    let unknown = GrowthBookAttribute::from(json!({ "id": 1 })).unwrap();
    let features2: HashMap<String, GrowthBookFeature> =
        serde_json::from_value(json!({ "u": { "defaultValue": false, "rules": [{ "condition": { "id": { "$notInGroup": "missing" } }, "force": true }] } })).unwrap();
    let gb2 = GrowthBook {
        forced_variations: None,
        features: features2,
        attributes: None,
        sticky_bucket_service: None,
        saved_groups: HashMap::new(),
    };
    assert!(gb2.check("u", &Some(unknown)).on, "unknown group → notInGroup is true");
}
