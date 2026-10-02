//! Runs the savedGroupReferencesV2 suites from sdk-js at
//! growthbook/growthbook commit 023c21040b8878650f0e2cf0eb7ca44ed4b6d2f1,
//! plus regressions for malformed payloads and recursive evaluation.

use growthbook_rust::client::{GrowthBookClient, GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::{FeatureResult, GrowthBookAttribute};
use serde_json::{json, Value};

#[path = "cases/mod.rs"]
mod corpus;

fn suite(name: &str) -> Vec<Value> {
    let corpus = corpus::active();
    corpus["savedGroupReferencesV2"][name].as_array().expect("saved group v2 suite").clone()
}

async fn client(context: &Value) -> GrowthBookClient {
    GrowthBookClientBuilder::new()
        .auto_refresh(false)
        .saved_groups(context["savedGroups"].clone())
        .features_json(context["features"].clone())
        .expect("valid features")
        .build()
        .await
        .expect("offline client")
}

async fn evaluate(
    condition: Value,
    attributes: Value,
    saved_groups: Value,
) -> bool {
    let context = json!({
        "savedGroups": saved_groups,
        "features": {"flag": {"defaultValue": false, "rules": [{"condition": condition, "force": true}]}}
    });
    client(&context).await.is_on("flag", Some(GrowthBookAttribute::from(attributes).expect("valid attributes")))
}

fn feature_result(
    client: &GrowthBookClient,
    context: &Value,
    key: &str,
) -> FeatureResult {
    let attributes = GrowthBookAttribute::from(context.get("attributes").cloned().unwrap_or(json!({}))).expect("valid attributes");
    client.feature_result(key, Some(attributes))
}

#[tokio::test]
async fn saved_group_v2_condition_conformance() {
    for case in suite("evalCondition") {
        assert_eq!(
            evaluate(case[1].clone(), case[2].clone(), case[4].clone()).await,
            case[3].as_bool().expect("expected boolean"),
            "{}",
            case[0]
        );
    }
}

#[tokio::test]
async fn saved_group_v2_feature_conformance() {
    for case in suite("feature") {
        let context = &case[1];
        let result = feature_result(&client(context).await, context, case[2].as_str().expect("feature key"));
        let expected = &case[3];
        // Rust's FeatureResult does not expose the corpus's ruleId field.
        assert_eq!(result.value, expected["value"], "{}: value", case[0]);
        assert_eq!(result.on, expected["on"], "{}: on", case[0]);
        assert_eq!(result.off, expected["off"], "{}: off", case[0]);
        assert_eq!(result.source, expected["source"], "{}: source", case[0]);
    }
}

#[tokio::test]
async fn saved_group_v2_experiment_conformance() {
    for case in suite("run") {
        let mut context = case[1].clone();
        let experiment = &case[2];
        // Rust exposes experiments through feature rules, not a standalone run
        // method. Use the same rule (including prerequisites) with control as
        // the default when targeting skips the experiment.
        let mut features = context.get("features").cloned().unwrap_or(json!({}));
        features["experiment"] = json!({"defaultValue": experiment["variations"][0], "rules": [experiment]});
        context["features"] = features;
        let result = feature_result(&client(&context).await, &context, "experiment");
        assert_eq!(result.value, case[3], "{}: value", case[0]);
        assert_eq!(
            result.experiment_result.as_ref().is_some_and(|result| result.in_experiment),
            case[4].as_bool().expect("inExperiment"),
            "{}: inExperiment",
            case[0]
        );
        assert_eq!(
            result.experiment_result.as_ref().is_some_and(|result| result.hash_used),
            case[5].as_bool().expect("hashUsed"),
            "{}: hashUsed",
            case[0]
        );
    }
}

#[tokio::test]
async fn malformed_references_and_entries_fail_closed() {
    let valid = json!({"grp": {"type": "list", "attributeKey": "id", "values": ["u_1"]}});
    for reference in [
        json!(null),
        json!(false),
        json!(42),
        json!([]),
        json!("grp"),
        json!({}),
        json!({"id": 1}),
        json!({"id": "grp", "attributeKey": null}),
    ] {
        assert!(!evaluate(json!({"$savedGroup": reference}), json!({"id": "u_1"}), valid.clone()).await, "reference: {reference}");
    }
    for entry in [
        json!(null),
        json!(false),
        json!(42),
        json!("invalid"),
        json!({}),
        json!({"type": "future", "values": ["u_1"]}),
        json!({"type": "list", "attributeKey": "id"}),
        json!({"type": "list", "attributeKey": "id", "values": "u_1"}),
        json!({"type": "condition"}),
        json!({"type": "condition", "condition": null}),
        json!({"type": "condition", "condition": []}),
        json!({"type": "condition", "condition": "bad"}),
    ] {
        let groups = json!({"grp": entry});
        for condition in [json!({"$savedGroup": {"id": "grp"}}), json!({"id": {"$inGroup": "grp"}}), json!({"id": {"$notInGroup": "grp"}})] {
            assert!(!evaluate(condition.clone(), json!({"id": "u_1"}), groups.clone()).await, "entry: {entry}, condition: {condition}");
        }
    }
}

#[tokio::test]
async fn group_membership_uses_strict_types_and_dot_paths() {
    let groups = json!({"grp": {"type": "list", "attributeKey": "user.id", "values": [2]}});
    for condition in [json!({"$savedGroup": {"id": "grp"}}), json!({"user.id": {"$inGroup": "grp"}}), json!({"user.id": {"$in": [2]}})] {
        for (value, expected) in [
            (json!(2), true),
            (json!(2.0), true),
            (json!("2"), false),
            (json!(["2", 2]), true),
            (json!(["2"]), false),
            (json!(null), false),
        ] {
            assert_eq!(evaluate(condition.clone(), json!({"user": {"id": value}}), groups.clone()).await, expected, "{condition}, {value}");
        }
    }
}

#[tokio::test]
async fn group_references_are_top_level_only_and_unknown_operators_fail_closed() {
    let groups = json!({"grp": {"type": "list", "attributeKey": "id", "values": ["u_1"]}});
    for condition in [
        json!({"id": {"$savedGroup": {"id": "grp"}}}),
        json!({"id": {"$and": [{"$savedGroup": {"id": "grp"}}]}}),
        json!({"$savedGroups": null}),
        json!({"$savedGroups": []}),
        json!({"$savedGroups": {}}),
        json!({"id": {"$future": null}}),
    ] {
        assert!(!evaluate(condition.clone(), json!({"id": "u_1"}), groups.clone()).await, "{condition}");
    }
}

#[tokio::test]
async fn cycles_preserve_boolean_semantics_and_sibling_independence() {
    let reference = json!({"$savedGroup": {"id": "grp"}});
    for (condition, expected) in [
        (reference.clone(), false),
        (json!({"$and": [reference]}), false),
        (json!({"$or": [reference, {"ok": false}]}), false),
        (json!({"$or": [reference, {"ok": true}]}), true),
        (json!({"$nor": [reference]}), true),
        (json!({"$not": reference}), true),
    ] {
        let groups = json!({"grp": {"type": "condition", "condition": condition}});
        assert_eq!(evaluate(reference.clone(), json!({"ok": true}), groups).await, expected, "{condition}");
    }
    let groups = json!({"grp": {"type": "condition", "condition": {"ok": true}}});
    assert!(evaluate(json!({"$and": [reference, reference]}), json!({"ok": true}), groups).await);
}

#[tokio::test]
async fn cycles_through_nested_array_conditions_terminate() {
    let reference = json!({"$savedGroup": {"id": "grp"}});
    // Include a plain key so $elemMatch treats its operand as an object
    // condition, where $savedGroup is valid, rather than an attribute operator.
    let element = json!({"tag": "x", "$savedGroup": {"id": "grp"}});
    for (condition, attributes) in [
        (json!({"items": {"$elemMatch": element}}), json!({"items": [{"tag": "x"}]})),
        (json!({"items": {"$all": [{"$elemMatch": element}]}}), json!({"items": [[{"tag": "x"}]]})),
        (json!({"items": {"$size": {"$savedGroup": {"id": "grp"}}}}), json!({"items": [1]})),
    ] {
        // Dropping the visited set when rebinding attributes would let the
        // nested reference match the tag branch instead of detecting the cycle.
        let groups = json!({"grp": {"type": "condition", "condition": {"$or": [condition, {"tag": "x"}]}}});
        assert!(!evaluate(reference.clone(), attributes, groups).await, "{condition}");
    }
}

#[tokio::test]
async fn nested_groups_evaluate_against_each_array_element() {
    let groups = json!({"grp": {"type": "condition", "condition": {"plan": "pro"}}});
    let element = json!({"plan": {"$exists": true}, "$savedGroup": {"id": "grp"}});
    assert!(
        evaluate(
            json!({"items": {"$elemMatch": element}}),
            json!({"plan": "free", "items": [{"plan": "free"}, {"plan": "pro"}]}),
            groups.clone()
        )
        .await
    );
    assert!(!evaluate(json!({"items": {"$elemMatch": element}}), json!({"plan": "pro", "items": [{"plan": "free"}]}), groups.clone()).await);
    assert!(evaluate(json!({"items": {"$all": [{"$elemMatch": element}]}}), json!({"items": [[{"plan": "pro"}]]}), groups).await);
}

fn group_chain(references: usize) -> Value {
    let mut groups = serde_json::Map::new();
    for i in 0..references - 1 {
        groups.insert(format!("grp_{i}"), json!({"type": "condition", "condition": {"$savedGroup": {"id": format!("grp_{}", i + 1)}}}));
    }
    groups.insert(format!("grp_{}", references - 1), json!({"type": "list", "attributeKey": "id", "values": ["u_1"]}));
    Value::Object(groups)
}

#[tokio::test]
async fn group_chains_are_bounded_without_restricting_siblings() {
    let reference = json!({"$savedGroup": {"id": "grp_0"}});
    for depth in [1, 101, 128] {
        assert!(evaluate(reference.clone(), json!({"id": "u_1"}), group_chain(depth)).await, "depth {depth}");
    }
    for depth in [129, 2000] {
        assert!(!evaluate(reference.clone(), json!({"id": "u_1"}), group_chain(depth)).await, "depth {depth}");
    }
    assert!(evaluate(json!({"$and": [reference.clone(), reference]}), json!({"id": "u_1"}), group_chain(128)).await);
}

#[tokio::test]
async fn all_error_markers_fail_and_negated_markers_pass() {
    for marker in ["__sgInvalid__", "__sgUnknown__", "__sgCycle__", "__sgMaxDepth__"] {
        let condition = json!({marker: "grp"});
        assert!(!evaluate(condition.clone(), json!({}), json!({})).await, "{marker}");
        assert!(evaluate(json!({"$not": condition}), json!({}), json!({})).await, "{marker}");
    }
}

#[tokio::test]
async fn list_attribute_overrides_do_not_fall_back_and_legacy_operators_ignore_them() {
    for entry in [
        json!({"type": "list", "values": ["u_1"]}),
        json!({"type": "list", "attributeKey": null, "values": ["u_1"]}),
        json!({"type": "list", "attributeKey": 42, "values": ["u_1"]}),
    ] {
        let groups = json!({"grp": entry});
        assert!(!evaluate(json!({"$savedGroup": {"id": "grp"}}), json!({"id": "u_1"}), groups.clone()).await);
        assert!(evaluate(json!({"$savedGroup": {"id": "grp", "attributeKey": "id"}}), json!({"id": "u_1"}), groups.clone()).await);
        assert!(evaluate(json!({"id": {"$inGroup": "grp"}}), json!({"id": "u_1"}), groups).await);
    }
    let groups = json!({"grp": {"type": "list", "attributeKey": "id", "values": ["u_1"]}});
    assert!(!evaluate(json!({"$savedGroup": {"id": "grp", "attributeKey": "missing"}}), json!({"id": "u_1"}), groups).await);
    let groups = json!({"grp": {"type": "condition", "condition": {}}});
    assert!(evaluate(json!({"$savedGroup": {"id": "grp"}}), json!({}), groups.clone()).await);
    assert!(!evaluate(json!({"$savedGroup": {"id": "grp", "attributeKey": null}}), json!({}), groups).await);
}

#[tokio::test]
async fn not_negates_the_whole_condition_including_group_and_attribute() {
    let groups = json!({"grp": {"type": "list", "attributeKey": "id", "values": ["u_1"]}});
    let condition = json!({"$not": {"$savedGroup": {"id": "grp"}, "country": "US"}});
    assert!(evaluate(condition.clone(), json!({"id": "u_1", "country": "CA"}), groups.clone()).await);
    assert!(!evaluate(condition, json!({"id": "u_1", "country": "US"}), groups).await);
}

#[tokio::test]
async fn saved_list_paths_handle_arrays_and_missing_intermediate_values() {
    for (path, attributes, expected) in [
        ("user.id", json!({"user": {"id": "u_1"}}), true),
        ("user.id", json!({"user": "u_1"}), false),
        ("user.id", json!({"user": null}), false),
        ("user.id", json!({}), false),
        ("users.0.id", json!({"users": [{"id": "u_1"}]}), true),
        ("users.1.id", json!({"users": [{"id": "u_1"}]}), false),
        ("users.00.id", json!({"users": [{"id": "u_1"}]}), false),
        ("users.+0.id", json!({"users": [{"id": "u_1"}]}), false),
        ("users.0.id.missing", json!({"users": [{"id": "u_1"}]}), false),
    ] {
        let groups = json!({"grp": {"type": "list", "attributeKey": path, "values": ["u_1"]}});
        for condition in [json!({"$savedGroup": {"id": "grp"}}), json!({path: {"$inGroup": "grp"}}), json!({path: {"$in": ["u_1"]}})] {
            assert_eq!(evaluate(condition, attributes.clone(), groups.clone()).await, expected, "{path}: {attributes}");
        }
    }
}

#[tokio::test]
async fn api_refresh_replaces_groups_and_preserves_malformed_entries() {
    use std::time::Duration;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    let features = json!({
        "v2": {"defaultValue": false, "rules": [{"condition": {"$savedGroup": {"id": "grp"}}, "force": true}]},
        "v1": {"defaultValue": false, "rules": [{"condition": {"id": {"$notInGroup": "grp"}}, "force": true}]}
    });
    let groups = json!({"grp": {"type": "list", "attributeKey": "id", "values": ["u_1"]}});
    Mock::given(method("GET"))
        .and(path("/api/features/test_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"features": features, "savedGroups": groups})))
        .expect(1)
        .mount(&server)
        .await;
    let client = GrowthBookClientBuilder::new()
        .api_url(server.uri())
        .client_key("test_key".into())
        .auto_refresh(false)
        .ttl(Duration::ZERO)
        .build()
        .await
        .expect("API client");
    let attributes = Some(GrowthBookAttribute::from(json!({"id": "u_1"})).expect("attributes"));
    assert!(client.is_on("v2", attributes.clone()));
    assert!(!client.is_on("v1", attributes.clone()));

    for (groups, expected_v1) in [(json!({"grp": null}), false), (json!({}), true)] {
        server.reset().await;
        Mock::given(method("GET"))
            .and(path("/api/features/test_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"features": features, "savedGroups": groups})))
            .expect(1)
            .mount(&server)
            .await;
        client.try_refresh().await.expect("refresh succeeds");
        assert!(!client.is_on("v2", attributes.clone()));
        assert_eq!(client.is_on("v1", attributes.clone()), expected_v1);
    }
}
