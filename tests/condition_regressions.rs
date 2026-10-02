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
async fn missing_paths_follow_javascript_order_comparisons() {
    for operator in ["$gt", "$gte", "$lt", "$lte"] {
        for threshold in [json!(100), json!("zzz")] {
            let expected = threshold.is_number() && matches!(operator, "$lt" | "$lte");
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
    assert!(evaluate(json!({"arr": {"$all": [{"$gt": 5}]}}), json!({"arr": [["a1"], [10]]})).await);
}
