//! Runs the corpus's top-level `run` section. Rust has no inline experiment
//! API, so each experiment is evaluated as the only rule of a feature whose
//! default is the control value.

use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::GrowthBookAttribute;
use serde_json::{json, Value};

#[path = "cases/mod.rs"]
mod corpus;

/// Context options that have no feature-rule equivalent.
const UNSUPPORTED_CONTEXT_KEYS: [&str; 4] = ["enabled", "qaMode", "url", "forcedVariations"];
/// Experiment fields that change meaning (or are ignored) inside a feature rule.
const UNSUPPORTED_EXPERIMENT_KEYS: [&str; 2] = ["active", "force"];

fn unsupported_reason(case: &Value) -> Option<String> {
    let context = case[1].as_object().expect("context object");
    let experiment = case[2].as_object().expect("experiment object");
    UNSUPPORTED_CONTEXT_KEYS
        .iter()
        .find(|key| context.contains_key(**key))
        .map(|key| format!("context.{key}"))
        .or_else(|| UNSUPPORTED_EXPERIMENT_KEYS.iter().find(|key| experiment.contains_key(**key)).map(|key| format!("experiment.{key}")))
}

#[tokio::test]
async fn run_conformance() {
    let corpus = corpus::active();
    let cases = corpus["run"].as_array().expect("run suite");
    let mut executed = 0;
    let mut skipped = vec![];

    for case in cases {
        let name = case[0].as_str().expect("case name");
        if let Some(reason) = unsupported_reason(case) {
            skipped.push(format!("{name} ({reason})"));
            continue;
        }

        let context = &case[1];
        let experiment = &case[2];
        let mut features = context.get("features").cloned().unwrap_or(json!({}));
        features["experiment"] = json!({"defaultValue": experiment["variations"][0], "rules": [experiment]});

        let client = GrowthBookClientBuilder::new()
            .auto_refresh(false)
            .saved_groups(context["savedGroups"].clone())
            .features_json(features)
            .expect("valid features")
            .build()
            .await
            .expect("offline client");
        let attributes = GrowthBookAttribute::from(context.get("attributes").cloned().unwrap_or(json!({}))).expect("valid attributes");
        let result = client.feature_result("experiment", Some(attributes));

        assert_eq!(result.value, case[3], "{name}: value");
        assert_eq!(
            result.experiment_result.as_ref().is_some_and(|result| result.in_experiment),
            case[4].as_bool().expect("inExperiment"),
            "{name}: inExperiment"
        );
        assert_eq!(
            result.experiment_result.as_ref().is_some_and(|result| result.hash_used),
            case[5].as_bool().expect("hashUsed"),
            "{name}: hashUsed"
        );
        executed += 1;
    }

    println!("run: executed {executed}, skipped {}:\n  {}", skipped.len(), skipped.join("\n  "));
    assert_eq!(executed + skipped.len(), cases.len());
}
