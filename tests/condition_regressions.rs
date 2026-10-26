use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::GrowthBookAttribute;
use serde_json::{json, Value};

async fn evaluate(
    condition: Value,
    attributes: Value,
) -> bool {
    let client = GrowthBookClientBuilder::new()
        .auto_refresh(false)
        .features_json(json!({"flag": {"defaultValue": false, "rules": [{"condition": condition, "force": true}]}}))
        .unwrap()
        .build()
        .await
        .unwrap();
    client.is_on("flag", Some(GrowthBookAttribute::from(attributes).unwrap()))
}

#[tokio::test]
async fn dollar_prefixed_attributes_support_missing_values_and_exclusions() {
    for key in ["$groups", "$custom"] {
        for (condition, expected) in [
            (json!({"$nin": ["staff"]}), true),
            (json!({"$not": {"$in": ["staff"]}}), true),
            (json!({"$exists": false}), true),
            (json!({"$in": ["staff"]}), false),
        ] {
            assert_eq!(evaluate(json!({key: condition}), json!({})).await, expected, "{key}");
        }
        assert!(!evaluate(json!({key: {"$nin": ["staff"]}}), json!({key: ["staff"]})).await);
        assert!(evaluate(json!({key: "staff"}), json!({key: "staff"})).await);
    }
    assert!(!evaluate(json!({"groups": {"$unknown": true}}), json!({"groups": ["staff"]})).await);
}

#[tokio::test]
async fn missing_paths_fail_order_comparisons() {
    for operator in ["$gt", "$gte", "$lt", "$lte"] {
        for threshold in [json!(100), json!("zzz")] {
            let expected = false;
            let comparison = json!({operator: threshold});
            assert_eq!(
                evaluate(json!({"items.price": comparison.clone()}), json!({"items": [{"price": 50}]})).await,
                expected,
                "dotted {operator}"
            );
            assert_eq!(
                evaluate(json!({"items": {"$elemMatch": {"price": comparison.clone()}}}), json!({"items": [{}]})).await,
                expected,
                "element {operator}"
            );
            assert_eq!(evaluate(json!({"missing": comparison}), json!({})).await, expected, "missing {operator}");
        }
    }
    assert!(evaluate(json!({"items.0.price": {"$gt": 100}}), json!({"items": [{"price": 150}]})).await);
    assert!(evaluate(json!({"items": {"$elemMatch": {"price": {"$gt": 100}}}}), json!({"items": [{}, {"price": 150}]})).await);
}

#[tokio::test]
async fn all_with_nonnumeric_members_does_not_panic() {
    for operator in ["$gt", "$gte", "$lt", "$lte"] {
        assert!(!evaluate(json!({"arr": {"$all": [{operator: 5}]}}), json!({"arr": [["a1"]]})).await, "{operator}");
    }
    assert!(evaluate(json!({"arr": {"$all": [{"$gt": 5}]}}), json!({"arr": [["a1"], 10]})).await);
    // Arrays are not ordered, so a nested `[10]` does not satisfy `$gt: 5`.
    assert!(!evaluate(json!({"arr": {"$all": [{"$gt": 5}]}}), json!({"arr": [["a1"], [10]]})).await);
}

// A plain object or array condition is deep equality, like JS comparing
// `JSON.stringify` output: `1` and `1.0` are the same number at any depth,
// while booleans, strings and numbers stay distinct.
#[tokio::test]
async fn plain_object_and_array_equality_compares_numbers_by_value() {
    for (condition, attribute, expected) in [
        (json!({"a": 1}), json!({"a": 1.0}), true),
        (json!({"a": 1.5}), json!({"a": 1.5}), true),
        (json!({"a": {"b": [1, 2]}}), json!({"a": {"b": [1.0, 2.0]}}), true),
        (json!({"a": 1, "b": 2}), json!({"b": 2, "a": 1}), true),
        (json!({"a": 1}), json!({"a": true}), false),
        (json!({"a": 1}), json!({"a": "1"}), false),
        (json!({"a": 1}), json!({"a": 1, "b": 2}), false),
        (json!([1, 2]), json!([1.0, 2.0]), true),
        (json!([[1], {"a": 2}]), json!([[1.0], {"a": 2.0}]), true),
        (json!([1]), json!([true]), false),
        (json!([1, 2]), json!([2, 1]), false),
    ] {
        assert_eq!(evaluate(json!({"x": condition}), json!({"x": attribute})).await, expected, "{condition} vs {attribute}");
    }
}

// Strings order by UTF-16 code units, as in JS. U+10000 is a surrogate pair
// (0xD800 0xDC00), so it sorts before U+E000 even though its code point is higher.
#[tokio::test]
async fn string_order_uses_utf16_code_units() {
    let attributes = json!({"x": "\u{10000}"});
    assert!(evaluate(json!({"x": {"$lt": "\u{E000}"}}), attributes.clone()).await);
    assert!(evaluate(json!({"x": {"$lte": "\u{E000}"}}), attributes.clone()).await);
    assert!(!evaluate(json!({"x": {"$gt": "\u{E000}"}}), attributes.clone()).await);
    assert!(!evaluate(json!({"x": {"$gte": "\u{E000}"}}), attributes).await);
}

#[tokio::test]
async fn elem_match_conditions_treat_primitive_elements_as_objects_without_fields() {
    // Like JS getPath on a primitive, every field of a non-object element is missing.
    assert!(evaluate(json!({"tags": {"$elemMatch": {"$nor": [{"name": "x"}]}}}), json!({"tags": [5]})).await);
    assert!(!evaluate(json!({"tags": {"$elemMatch": {"$or": [{"name": "x"}]}}}), json!({"tags": ["x"]})).await);
    assert!(evaluate(json!({"tags": {"$elemMatch": {"$or": [{"name": "x"}]}}}), json!({"tags": [5, {"name": "x"}]})).await);
}

#[tokio::test]
async fn unknown_operators_fail_under_negation() {
    for condition in [
        json!({"$not": {"x": {"$future": 1}}}),
        json!({"$nor": [{"x": {"$future": 1}}]}),
        json!({"x": {"$not": {"$future": 1}}}),
        json!({"x": {"$elemMatch": {"$future": 1}}}),
    ] {
        assert!(!evaluate(condition.clone(), json!({"x": [1]})).await, "{condition}");
    }
    // A known operator that fails still decides the condition, and an $or another condition passes.
    assert!(evaluate(json!({"$or": [{"x": {"$future": 1}}, {"y": 1}]}), json!({"y": 1})).await);
}

#[tokio::test]
async fn malformed_logical_operators_fail() {
    for condition in [json!({"$or": null, "id": "123"}), json!({"$and": null}), json!({"$nor": "x"}), json!({"$or": [null]})] {
        assert!(!evaluate(condition.clone(), json!({"id": "123"})).await, "{condition}");
    }
}
