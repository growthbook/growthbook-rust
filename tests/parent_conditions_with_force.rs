use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use serde_json::json;

// R1 regression: a rule shaped `{parentConditions: [...], force: "X"}` must
// evaluate its parentConditions before forcing. Previously the untagged enum
// matched the `Force` variant and silently dropped `parentConditions`, so
// rule1's force fired unconditionally and `child` returned "prereqon".
//
// Two rules each gate on a different value of the same prereq. With `probe`
// defaulting to "ui_default", rule1's parent (value == "ON") fails and rule2's
// parent (value != "ON") passes — so JS/Python/Go all return "prereqoff".
#[tokio::test]
async fn parent_conditions_gate_force_rule() {
    let features_json = json!({
        "probe": { "defaultValue": "ui_default" },
        "child": {
            "defaultValue": "fallback",
            "rules": [
                { "parentConditions": [{ "id": "probe", "condition": { "value": "ON" }, "gate": false }], "force": "prereqon" },
                { "parentConditions": [{ "id": "probe", "condition": { "value": { "$ne": "ON" } }, "gate": false }], "force": "prereqoff" }
            ]
        }
    });

    let client = GrowthBookClientBuilder::new().features_json(features_json).unwrap().build().await.expect("Failed to build client");

    let result = client.feature_result("child", None);
    assert_eq!(result.value, json!("prereqoff"), "expected the second (prereqoff) rule to win, got source={}", result.source);
}
