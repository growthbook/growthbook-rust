use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use serde_json::json;

// a parentCondition with `gate: false` that fails should skip
// only that rule and fall through to the next, not short-circuit the whole
// feature with source=prerequisite.
//
// rule1's non-gating parent (B == "on") fails because B defaults to "off", so
// rule1 is skipped and rule2's unconditional force fires → "fallback".
#[tokio::test]
async fn non_gating_parent_failure_skips_rule() {
    let features_json = json!({
        "B": { "defaultValue": "off" },
        "A": {
            "defaultValue": "hello",
            "rules": [
                { "parentConditions": [{ "id": "B", "condition": { "value": "on" }, "gate": false }], "force": "rule1_value" },
                { "force": "fallback" }
            ]
        }
    });

    let client = GrowthBookClientBuilder::new().features_json(features_json).unwrap().build().await.expect("Failed to build client");

    let result = client.feature_result("A", None);
    assert_eq!(result.value, json!("fallback"), "expected rule1 skipped and fallback forced, got source={}", result.source);
}

// Sanity check the gate=true path still short-circuits: a gating parent that
// fails must block the feature with source=prerequisite, never reaching rule2.
#[tokio::test]
async fn gating_parent_failure_short_circuits() {
    let features_json = json!({
        "B": { "defaultValue": "off" },
        "A": {
            "defaultValue": "hello",
            "rules": [
                { "parentConditions": [{ "id": "B", "condition": { "value": "on" }, "gate": true }], "force": "rule1_value" },
                { "force": "fallback" }
            ]
        }
    });

    let client = GrowthBookClientBuilder::new().features_json(features_json).unwrap().build().await.expect("Failed to build client");

    let result = client.feature_result("A", None);
    assert_eq!(result.source, "prerequisite", "expected gating prerequisite to short-circuit, got value={}", result.value);
}
