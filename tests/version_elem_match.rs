use growthbook_rust::growthbook::GrowthBook;
use growthbook_rust::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};
use serde_json::{json, Value};
use std::collections::HashMap;

fn evaluate(
    condition: Value,
    tags: Value,
) -> bool {
    let gb = GrowthBook {
        features: serde_json::from_value(json!({"flag": {
            "defaultValue": false,
            "rules": [{"condition": condition, "force": true}]
        }}))
        .unwrap(),
        saved_groups: HashMap::from([("versions".to_owned(), vec![GrowthBookAttributeValue::String("1.2.0".to_owned())])]),
        forced_variations: None,
        attributes: None,
        sticky_bucket_service: None,
    };
    let attributes = GrowthBookAttribute::from(json!({"tags": tags})).unwrap();
    gb.check("flag", &Some(attributes)).on
}

#[test]
fn version_operators_compare_array_members_and_not_negates_the_match() {
    for (operator, expected) in [("$vgt", true), ("$vgte", true), ("$vlt", false), ("$vlte", false), ("$veq", false), ("$vne", true)] {
        let condition = json!({"$elemMatch": {operator: "1.0.0"}});
        for tags in [json!(["1.2.0"]), json!([null, "1.2.0"])] {
            assert_eq!(evaluate(json!({"tags": condition.clone()}), tags.clone()), expected, "{operator}: {tags}");
            assert_eq!(evaluate(json!({"tags": {"$not": condition.clone()}}), tags.clone()), !expected, "negated {operator}: {tags}");
        }
        for tags in [json!([]), json!([null]), json!(null), json!("1.2.0")] {
            assert!(!evaluate(json!({"tags": condition.clone()}), tags.clone()), "{operator}: {tags}");
        }
    }
}

#[test]
fn numeric_negative_zero_compares_as_zero() {
    for (operator, expected) in [("$vgt", false), ("$vgte", true), ("$vlt", false), ("$vlte", true), ("$veq", true), ("$vne", false)] {
        for (attribute, operand) in [(json!(-0.0), json!("0")), (json!("0"), json!(-0.0))] {
            let condition = json!({operator: operand});
            assert_eq!(evaluate(json!({"tags": condition.clone()}), attribute.clone()), expected, "{operator}: {attribute}");
            assert_eq!(evaluate(json!({"tags": {"$elemMatch": condition}}), json!([attribute])), expected, "array {operator}");
        }
    }
    assert!(!evaluate(json!({"tags": {"$veq": "0"}}), json!("-0")));
}

#[test]
fn all_version_constraints_must_match_the_same_member() {
    let condition = json!({"tags": {"$elemMatch": {"$vgt": "1.0.0", "$vlt": "2.0.0"}}});
    assert!(!evaluate(condition.clone(), json!(["0.9.0", "2.1.0"])));
    assert!(evaluate(condition, json!(["0.9.0", "1.2.0", "2.1.0"])));
}

#[test]
fn version_constraints_work_in_object_members_and_nested_arrays() {
    let objects = json!({"tags": {"$elemMatch": {"version": {"$vgt": "1.0.0"}, "enabled": true}}});
    assert!(evaluate(objects.clone(), json!([{"version": "1.2.0", "enabled": true}])));
    assert!(!evaluate(objects, json!([{"version": "1.2.0", "enabled": false}, {"version": "0.9.0", "enabled": true}])));

    let nested = json!({"tags": {"$elemMatch": {"$elemMatch": {"$vgt": "1.0.0"}}}});
    assert!(evaluate(nested.clone(), json!([["0.9.0"], ["1.2.0"]])));
    assert!(!evaluate(nested, json!([["0.9.0"], []])));
}

#[test]
fn member_context_preserves_saved_groups_and_falsy_values() {
    let condition = json!({"tags": {"$elemMatch": {"$inGroup": "versions", "$vgt": "1.0.0"}}});
    assert!(evaluate(condition.clone(), json!(["1.2.0"])));
    assert!(!evaluate(condition, json!(["1.3.0"])));
    for value in [json!(0), json!(""), json!(false)] {
        assert!(evaluate(json!({"tags": {"$elemMatch": {"$veq": "0"}}}), json!([value])));
    }
}
