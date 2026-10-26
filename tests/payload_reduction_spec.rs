//! Runs the corpus's `pruneCondition` and `reducePayload` sections.

use growthbook_rust::payload::{prune_condition, reduce_payload, ReducePayloadOptions};
use serde_json::{json, Value};

#[path = "cases/mod.rs"]
mod corpus;

fn suite(name: &str) -> Vec<Value> {
    let cases = corpus::active()[name].as_array().expect("suite").clone();
    assert!(!cases.is_empty(), "{name} has no cases");
    cases
}

#[test]
fn prune_condition_conformance() {
    let failures: Vec<String> = suite("pruneCondition")
        .iter()
        .filter_map(|case| {
            let saved_groups = case.get(4).cloned().unwrap_or_else(|| json!({}));
            let actual = prune_condition(&case[2], &case[1], &saved_groups);
            (actual != case[3]).then(|| format!("{}\n    expected {}\n    actual   {}", case[0], case[3], actual))
        })
        .collect();
    assert!(failures.is_empty(), "{} failed:\n  {}", failures.len(), failures.join("\n  "));
}

#[test]
fn reduce_payload_conformance() {
    let failures: Vec<String> = suite("reducePayload")
        .iter()
        .filter_map(|case| {
            let input = &case[1];
            // The user's attributes take precedence over global ones
            let mut attributes = input.get("globalAttributes").cloned().unwrap_or_else(|| json!({}));
            if let (Some(merged), Some(user)) = (attributes.as_object_mut(), input.get("attributes").and_then(Value::as_object)) {
                merged.extend(user.clone());
            }
            let options: ReducePayloadOptions = serde_json::from_value(json!({
                "include": input.get("include").cloned().unwrap_or_else(|| json!({})),
                "exclude": input.get("exclude").cloned().unwrap_or_else(|| json!({})),
                "prerequisites": input.get("prerequisites").cloned().unwrap_or_else(|| json!("keep")),
            }))
            .expect("valid options");
            let actual = reduce_payload(&input["payload"], &attributes, &options);
            (actual != case[2]).then(|| format!("{}\n    expected {}\n    actual   {}", case[0], case[2], actual))
        })
        .collect();
    assert!(failures.is_empty(), "{} failed:\n  {}", failures.len(), failures.join("\n  "));
}
