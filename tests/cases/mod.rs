//! Shared offline corpus loader for unit and integration tests.

use serde_json::Value;

/// Combine the unmodified upstream corpus with Rust-only cases.
pub fn load() -> Value {
    let mut base: Value = serde_json::from_str(include_str!("cases.json")).expect("upstream cases");
    let rust: Value = serde_json::from_str(include_str!("rust.json")).expect("Rust cases");
    append_cases(&mut base, rust);
    base
}

/// Load supported cases. Exclusions never change the vendored upstream file.
pub fn active() -> Value {
    let mut corpus = load();
    let exclusions: Value = serde_json::from_str(include_str!("exclusions.json")).expect("execution exclusions");
    apply_exclusions(&mut corpus, &exclusions);
    corpus
}

fn append_cases(
    base: &mut Value,
    additions: Value,
) {
    match additions {
        Value::Object(suites) => {
            let base = base.as_object_mut().expect("suite object");
            for (name, cases) in suites {
                let empty = if cases.is_array() { Value::Array(vec![]) } else { serde_json::json!({}) };
                append_cases(base.entry(name).or_insert(empty), cases);
            }
        },
        Value::Array(cases) => {
            let base = base.as_array_mut().expect("case array");
            for case in cases {
                let name = case[0].as_str().expect("Rust case name");
                assert!(!base.iter().any(|existing| existing[0].as_str() == Some(name)), "Rust case shadows an existing case: {name}");
                base.push(case);
            }
        },
        _ => panic!("Rust additions must contain suites of cases, not metadata overrides"),
    }
}

fn suite_mut<'a>(
    corpus: &'a mut Value,
    path: &str,
) -> &'a mut Value {
    path.split('.')
        .fold(corpus, |suite, key| suite.get_mut(key).unwrap_or_else(|| panic!("Unknown excluded suite: {path}")))
}

fn clear_suite(suite: &mut Value) {
    match suite {
        Value::Array(cases) => cases.clear(),
        Value::Object(suites) => suites.values_mut().for_each(clear_suite),
        _ => panic!("Excluded suite must contain cases"),
    }
}

fn apply_exclusions(
    corpus: &mut Value,
    exclusions: &Value,
) {
    for (suite, cases) in exclusions["cases"].as_object().expect("case exclusions") {
        let target = suite_mut(corpus, suite).as_array_mut().expect("excluded case array");
        for (name, reason) in cases.as_object().expect("named exclusions") {
            assert!(!reason.as_str().expect("exclusion reason").trim().is_empty(), "Missing reason for {name}");
            let before = target.len();
            target.retain(|case| case[0].as_str() != Some(name));
            assert!(target.len() < before, "Stale case exclusion: {suite}/{name}");
        }
    }
    for (suite, reason) in exclusions["suites"].as_object().expect("suite exclusions") {
        assert!(!reason.as_str().expect("exclusion reason").trim().is_empty(), "Missing reason for {suite}");
        clear_suite(suite_mut(corpus, suite));
    }
}

#[cfg(test)]
mod tests {
    use super::{active, append_cases, apply_exclusions, load};
    use serde_json::json;

    #[test]
    fn exclusions_preserve_source_and_supported_cases() {
        let all = load();
        let supported = active();
        assert!(!all["contextualBandit"].as_array().unwrap().is_empty());
        assert!(supported["contextualBandit"].as_array().unwrap().is_empty());
        assert_eq!(all["feature"].as_array().unwrap().len() - supported["feature"].as_array().unwrap().len(), 4);
        assert_eq!(all["evalCondition"], supported["evalCondition"]);
        assert!(supported["feature"]
            .as_array()
            .unwrap()
            .iter()
            .any(|case| case[0] == "standard type is treated as a standard experiment"));
    }

    #[test]
    #[should_panic(expected = "shadows an existing case")]
    fn local_cases_cannot_override_upstream_expectations() {
        let mut base = json!({"feature": [["same name", true]]});
        append_cases(&mut base, json!({"feature": [["same name", false]]}));
    }

    #[test]
    #[should_panic(expected = "Stale case exclusion")]
    fn stale_exclusions_are_errors() {
        apply_exclusions(&mut json!({"feature": [["present", true]]}), &json!({"suites": {}, "cases": {"feature": {"absent": "unsupported"}}}));
    }
}
