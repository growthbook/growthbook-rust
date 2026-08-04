use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::GrowthBookAttribute;
use serde_json::json;

// #19: `$eq`/`$ne` on a *present* null attribute must match JS `===`/`!==`:
//   null === 5    -> false      null === null -> true
//   null !== 5    -> true       null !== null -> false
// The previous code special-cased the `Empty` arm and returned the wrong result
// for `$ne` (and `$eq: null`), also breaking the `ne == !eq` invariant.
#[tokio::test]
async fn eq_ne_present_null_matches_js() {
    let features_json = json!({
        "ne-5":    { "defaultValue": false, "rules": [{ "condition": { "x": { "$ne": 5 } },    "force": true }] },
        "eq-5":    { "defaultValue": false, "rules": [{ "condition": { "x": { "$eq": 5 } },    "force": true }] },
        "ne-null": { "defaultValue": false, "rules": [{ "condition": { "x": { "$ne": null } }, "force": true }] },
        "eq-null": { "defaultValue": false, "rules": [{ "condition": { "x": { "$eq": null } }, "force": true }] }
    });

    let client = GrowthBookClientBuilder::new().features_json(features_json).unwrap().build().await.expect("Failed to build client");

    // Attribute present but null.
    let null_attr = || GrowthBookAttribute::from(json!({ "x": null })).unwrap();

    assert!(client.is_on("ne-5", Some(null_attr())), "null $ne 5 must be true");
    assert!(!client.is_on("eq-5", Some(null_attr())), "null $eq 5 must be false");
    assert!(!client.is_on("ne-null", Some(null_attr())), "null $ne null must be false");
    assert!(client.is_on("eq-null", Some(null_attr())), "null $eq null must be true");

    // Missing attribute (absent key) was already correct; guard against regressions.
    let missing = || GrowthBookAttribute::from(json!({ "y": 1 })).unwrap();
    assert!(client.is_on("ne-5", Some(missing())), "missing $ne 5 must be true");
    assert!(!client.is_on("eq-5", Some(missing())), "missing $eq 5 must be false");
}
