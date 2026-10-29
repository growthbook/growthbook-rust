//! Spec 0.12.0 behaviour that the shared cases don't cover.

use std::sync::Arc;

use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::GrowthBookAttribute;
use growthbook_rust::sticky_bucket::InMemoryStickyBucketService;
use serde_json::{json, Value};

/// Evaluate a condition through a force rule, so `is_on` is the result.
async fn matches(
    condition: Value,
    attributes: Value,
    saved_groups: Value,
) -> bool {
    let client = GrowthBookClientBuilder::new()
        .auto_refresh(false)
        .saved_groups(saved_groups)
        .features_json(json!({"flag": {"defaultValue": false, "rules": [{"condition": condition, "force": true}]}}))
        .expect("valid features")
        .build()
        .await
        .expect("offline client");
    client.is_on("flag", Some(GrowthBookAttribute::from(attributes).expect("valid attributes")))
}

#[tokio::test]
async fn conditions_match_js() {
    let cases = [
        // Arrays and objects compare by reference in JS, so they are never equal.
        (json!({"x": {"$eq": [1]}}), json!({"x": [1]}), false),
        (json!({"x": {"$ne": [1]}}), json!({"x": [1]}), true),
        (json!({"x": {"$eq": {"a": 1}}}), json!({"x": {"a": 1}}), false),
        (json!({"x": {"$ne": {"a": 1}}}), json!({"x": {"a": 1}}), true),
        (json!({"x": {"$in": [[1]]}}), json!({"x": [1]}), false),
        (json!({"x": {"$nin": [[1]]}}), json!({"x": [1]}), true),
        (json!({"x": {"$ini": [{"a": 1}]}}), json!({"x": {"a": 1}}), false),
        (json!({"x": {"$nini": [{"a": 1}]}}), json!({"x": {"a": 1}}), true),
        // A non-string regex pattern never matches.
        (json!({"x": {"$regex": 1}}), json!({"x": "1"}), false),
        (json!({"x": {"$regexi": null}}), json!({"x": "null"}), false),
        (json!({"x": {"$regex": ["a"]}}), json!({"x": "a"}), false),
        // Numbers compare by value.
        (json!({"x": {"$in": [1.0]}}), json!({"x": 1}), true),
        (json!({"x": {"$all": [1.0]}}), json!({"x": [1]}), true),
        (json!({"x": {"$elemMatch": {"$eq": 1.0}}}), json!({"x": [1]}), true),
        // A missing attribute is null.
        (json!({"x": {"$eq": null}}), json!({}), true),
        (json!({"x": {"$ne": null}}), json!({}), false),
        (json!({"x": {"$type": "null"}}), json!({}), true),
        (json!({"x": {"$type": "undefined"}}), json!({}), false),
        (json!({"x": {"$exists": true}}), json!({}), false),
        (json!({"x": {"$nin": ["a"]}}), json!({}), true),
        (json!({"x": {"$nini": ["a"]}}), json!({}), true),
        (json!({"$nor": []}), json!({"x": 1}), false),
        // A path segment indexes into an array.
        (json!({"tags.0": "a"}), json!({"tags": ["a", "b"]}), true),
        (json!({"tags.1": "a"}), json!({"tags": ["a", "b"]}), false),
        (json!({"x.1.y": 2}), json!({"x": [{"y": 1}, {"y": 2}]}), true),
    ];
    for (condition, attributes, expected) in cases {
        assert_eq!(matches(condition.clone(), attributes.clone(), json!({})).await, expected, "{condition} with {attributes}");
    }
}

#[tokio::test]
async fn saved_group_numbers_compare_by_value() {
    let groups = json!({"g": [1.0, 2]});
    assert!(matches(json!({"x": {"$inGroup": "g"}}), json!({"x": 1}), groups.clone()).await);
    assert!(matches(json!({"x": {"$inGroup": "g"}}), json!({"x": 2.0}), groups).await);
}

/// Run a two-way experiment rule that falls back to `deviceId`.
async fn experiment(
    id: Value,
    sticky: bool,
) -> (Value, Option<String>) {
    let mut builder = GrowthBookClientBuilder::new()
        .auto_refresh(false)
        .features_json(json!({
            "f": {"defaultValue": "default", "rules": [{"key": "exp", "variations": ["a", "b"], "fallbackAttribute": "deviceId"}]}
        }))
        .expect("valid features");
    if sticky {
        builder = builder.sticky_bucket_service(Arc::new(InMemoryStickyBucketService::new()));
    }
    let client = builder.build().await.expect("offline client");
    let result = client.feature_result("f", Some(GrowthBookAttribute::from(json!({"id": id, "deviceId": "d1"})).expect("valid attributes")));
    (result.value, result.experiment_result.and_then(|it| it.hash_attribute))
}

#[tokio::test]
async fn unusable_hash_attributes_count_as_missing() {
    for id in [json!(true), json!(false), json!([]), json!(["1"]), json!({}), json!(0), json!(""), json!(null)] {
        // Without sticky bucketing JS doesn't use the fallback attribute.
        assert_eq!(experiment(id.clone(), false).await, (json!("default"), None), "id={id}");
        // With it, the fallback attribute is hashed instead.
        let (value, hash_attribute) = experiment(id.clone(), true).await;
        assert_ne!(value, json!("default"), "id={id}");
        assert_eq!(hash_attribute.as_deref(), Some("deviceId"), "id={id}");
    }
}

#[tokio::test]
async fn feature_on_uses_js_truthiness() {
    for (value, on) in [
        (json!([]), true),
        (json!({}), true),
        (json!("off"), true),
        (json!(null), false),
        (json!(false), false),
        (json!(0), false),
        (json!(""), false),
    ] {
        let client = GrowthBookClientBuilder::new()
            .auto_refresh(false)
            .features_json(json!({"f": {"defaultValue": value}}))
            .expect("valid features")
            .build()
            .await
            .expect("offline client");
        let result = client.feature_result("f", None);
        assert_eq!((result.on, result.off), (on, !on), "{value}");
    }
}

#[tokio::test]
async fn single_variation_experiment_is_skipped() {
    let client = GrowthBookClientBuilder::new()
        .auto_refresh(false)
        .features_json(json!({"f": {"defaultValue": "default", "rules": [{"key": "exp", "variations": ["only"]}]}}))
        .expect("valid features")
        .build()
        .await
        .expect("offline client");
    let result = client.feature_result("f", Some(GrowthBookAttribute::from(json!({"id": "1"})).expect("valid attributes")));
    assert_eq!(result.value, json!("default"));
    assert!(result.experiment_result.is_none());
}
