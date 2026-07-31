use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::GrowthBookAttribute;
use serde_json::json;

// R3: a non-string `$regex` pattern (e.g. `{x: {$regex: 5}}`) must match nothing.
//
// JS `getRegex(expected)` runs `expected.replace(/.../)` before compiling the
// pattern; on a non-string that throws a TypeError, which the `$regex` handler
// catches and returns `false` for. It does NOT coerce `5` into `/5/`. Rust used
// to return `true` unconditionally on this branch (ignoring the user value),
// which is the divergence this test locks down.
//
// Each feature forces `true` when its condition matches, so `is_on` reflects the
// operator result directly.
#[tokio::test]
async fn non_string_regex_pattern_matches_nothing() {
    let features_json = json!({
        // Non-string pattern → never matches.
        "regex-num": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": { "$regex": 5 } }, "force": true }]
        },
        // Case-insensitive variant shares the same non-string branch.
        "regexi-num": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": { "$regexi": 5 } }, "force": true }]
        },
        // `$notRegex` is the inverse: a non-string pattern matches nothing, so the
        // negation is always true.
        "not-regex-num": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": { "$notRegex": 5 } }, "force": true }]
        },
        // Sanity: a real string pattern still matches via coercion of the user value.
        "regex-str": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": { "$regex": "^5$" } }, "force": true }]
        }
    });

    let client = GrowthBookClientBuilder::new().features_json(features_json).unwrap().build().await.expect("Failed to build client");

    let attrs = |value: serde_json::Value| GrowthBookAttribute::from(json!({ "x": value })).unwrap();

    // $regex 5 vs {x: 5} → false (pattern is a number; JS throws → false).
    assert!(!client.is_on("regex-num", Some(attrs(json!(5)))), "$regex with a numeric pattern must not match");
    // Same for the string form of the value — still no match.
    assert!(!client.is_on("regex-num", Some(attrs(json!("5")))), "$regex with a numeric pattern must not match a string either");
    // $regexi 5 → false on the same branch.
    assert!(!client.is_on("regexi-num", Some(attrs(json!(5)))), "$regexi with a numeric pattern must not match");
    // $notRegex 5 → true (inverse of a never-matching pattern).
    assert!(client.is_on("not-regex-num", Some(attrs(json!(5)))), "$notRegex with a numeric pattern must be the inverse (true)");
    // Sanity: a valid string pattern still matches (coerces the user value to a string).
    assert!(client.is_on("regex-str", Some(attrs(json!(5)))), "$regex \"^5$\" must still match integer 5 via coercion");
}
