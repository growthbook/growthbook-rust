use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::GrowthBookAttribute;
use serde_json::json;

// A force rule with a `range` (percentage-gated force, no `coverage`) must hash
// on the rule's `hashAttribute`, not a hardcoded "id" — matching JS
// `isIncludedInRollout`, which resolves the hash value via the rule's
// hashAttribute. Previously the Force path ignored `hashAttribute` and always
// hashed on "id", so a user keyed on a different attribute was never included.
#[tokio::test]
async fn force_rule_with_range_honors_hash_attribute() {
    let features_json = json!({
        "f": {
            "defaultValue": false,
            "rules": [{
                "force": true,
                // Covers the whole [0,1) bucket space, so any user with the hash
                // attribute is included.
                "range": [0.0, 1.0],
                "hashAttribute": "userId"
            }]
        }
    });

    let client = GrowthBookClientBuilder::new().features_json(features_json).unwrap().build().await.expect("Failed to build client");

    // User has `userId` but no `id`. Hashing on "id" (the old bug) would find no
    // value and skip the rule -> default (false). Hashing on `userId` includes
    // the user -> forced true.
    let attrs = GrowthBookAttribute::from(json!({ "userId": "u1" })).unwrap();
    assert!(client.is_on("f", Some(attrs)), "force rule with a range must hash on its hashAttribute, not \"id\"");
}
