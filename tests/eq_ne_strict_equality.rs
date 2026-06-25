use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::GrowthBookAttribute;
use serde_json::json;

// R7: `$eq`/`$ne` must be strict (JS `===`), with no number↔string coercion.
// `$lt`/`$gt` keep their coercion, matching JS `<`/`>`.
//
// Each feature forces `true` when its condition matches, so `is_on` reflects
// the operator result directly.
#[tokio::test]
async fn eq_ne_are_strict_lt_still_coerces() {
    let features_json = json!({
        "eq-num": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": { "$eq": 5 } }, "force": true }]
        },
        "ne-num": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": { "$ne": 5 } }, "force": true }]
        },
        "eq-bool": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": { "$eq": true } }, "force": true }]
        },
        "lt-num": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": { "$lt": 5 } }, "force": true }]
        }
    });

    let client = GrowthBookClientBuilder::new().features_json(features_json).unwrap().build().await.expect("Failed to build client");

    let attrs = |value: serde_json::Value| GrowthBookAttribute::from(json!({ "x": value })).unwrap();

    // $eq 5 vs "5" → false (string is a different JS type; no coercion)
    assert!(!client.is_on("eq-num", Some(attrs(json!("5")))), "$eq 5 must not match string \"5\"");
    // $eq 5 vs 5 → true (same type sanity check)
    assert!(client.is_on("eq-num", Some(attrs(json!(5)))), "$eq 5 must match integer 5");
    // $ne 5 vs "5" → true (already strict; guards against regressions)
    assert!(client.is_on("ne-num", Some(attrs(json!("5")))), "$ne 5 must treat string \"5\" as not-equal");
    // $eq true vs 1 → false (boolean vs number are different JS types)
    assert!(!client.is_on("eq-bool", Some(attrs(json!(1)))), "$eq true must not match integer 1");
    // $lt 5 vs "3" → true (numeric coercion still applies for ordering ops)
    assert!(client.is_on("lt-num", Some(attrs(json!("3")))), "$lt 5 must still coerce string \"3\"");
}
