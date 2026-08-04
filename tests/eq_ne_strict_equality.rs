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

// `$eq`/`$ne` must treat all JSON numbers as one type, matching JS (`5 === 5.0`).
// Strictness applies *across JS types* (number vs string), NOT across Rust's
// internal `Int`/`Float` split. Regression guard: making the scalar arm use raw
// `PartialEq` made `Int(5) != Float(5.0)`, so `$eq: 5` stopped matching `5.0`.
#[tokio::test]
async fn eq_ne_treat_int_and_float_as_the_same_number() {
    let features_json = json!({
        "eq-int": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": { "$eq": 5 } }, "force": true }]
        },
        "eq-float": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": { "$eq": 5.0 } }, "force": true }]
        },
        "ne-int": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": { "$ne": 5 } }, "force": true }]
        }
    });

    let client = GrowthBookClientBuilder::new().features_json(features_json).unwrap().build().await.expect("Failed to build client");

    let attrs = |value: serde_json::Value| GrowthBookAttribute::from(json!({ "x": value })).unwrap();

    // $eq 5 (int) vs 5.0 (float) → true (JS `5 === 5.0`).
    assert!(client.is_on("eq-int", Some(attrs(json!(5.0)))), "$eq 5 must match float 5.0");
    // $eq 5.0 (float) vs 5 (int) → true (symmetric).
    assert!(client.is_on("eq-float", Some(attrs(json!(5)))), "$eq 5.0 must match integer 5");
    // $eq 5 vs 5.5 → false (different numeric value).
    assert!(!client.is_on("eq-int", Some(attrs(json!(5.5)))), "$eq 5 must not match 5.5");
    // $ne 5 (int) vs 5.0 (float) → false (they are equal, so not-equal is false).
    assert!(!client.is_on("ne-int", Some(attrs(json!(5.0)))), "$ne 5 must treat float 5.0 as equal");
    // $eq 5 vs "5" stays strict across JS types → false (guards against over-correcting).
    assert!(!client.is_on("eq-int", Some(attrs(json!("5")))), "$eq 5 must still reject string \"5\"");
}

// `$ne` must stay the exact inverse of `$eq`, including for nested-object
// conditions that resolve the parent key to a whole object and compare via the
// flattened-string path. (Guards against `$eq` and `$ne` using different
// object comparisons, which would break the `ne == !eq` invariant.)
#[tokio::test]
async fn ne_is_the_inverse_of_eq_for_nested_objects() {
    let features_json = json!({
        "eq-obj": {
            "defaultValue": false,
            "rules": [{ "condition": { "tags": { "$eq": "world" } }, "force": true }]
        },
        "ne-obj": {
            "defaultValue": false,
            "rules": [{ "condition": { "tags": { "$ne": "world" } }, "force": true }]
        }
    });

    let client = GrowthBookClientBuilder::new().features_json(features_json).unwrap().build().await.expect("Failed to build client");

    // user `tags` is a nested object that flattens to "world".
    let attrs = GrowthBookAttribute::from(json!({ "tags": { "hello": "world" } })).unwrap();

    let eq = client.is_on("eq-obj", Some(attrs.clone()));
    let ne = client.is_on("ne-obj", Some(attrs));
    assert!(eq, "$eq should match the flattened object");
    assert!(!ne, "$ne should be the inverse of $eq");
    assert_ne!(eq, ne, "ne must be the exact inverse of eq");
}
