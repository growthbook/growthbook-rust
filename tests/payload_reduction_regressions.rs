use growthbook_rust::payload::{prune_condition, reduce_payload, ReducePayloadOptions};
use serde_json::{json, Map, Value};

/// A chain of condition groups where each references the next twice, ending in `{"id": "123"}`.
fn doubling_groups(levels: usize) -> Value {
    let mut groups = Map::new();
    for level in 0..levels {
        let next = json!({"$savedGroup": {"id": format!("g{}", level + 1)}});
        groups.insert(format!("g{level}"), json!({"type": "condition", "condition": {"$and": [next.clone(), next]}}));
    }
    groups.insert(format!("g{levels}"), json!({"type": "condition", "condition": {"id": "123"}}));
    Value::Object(groups)
}

#[test]
fn pruning_known_attributes_keeps_the_saved_group_path() {
    // Evaluating `items` inside group g must see that g is already being resolved,
    // as the evaluator does, so the nested reference fails as a cycle
    let groups = json!({"g": {"type": "condition", "condition": {"$or": [
        {"items": {"$elemMatch": {"tag": "x", "$savedGroup": {"id": "g"}}}},
        {"tag": "x"}
    ]}}});
    let pruned = prune_condition(&json!({"items": [{"tag": "x"}]}), &json!({"$savedGroup": {"id": "g"}}), &groups);
    assert_ne!(pruned, Value::Bool(true));
}

#[test]
fn pruning_stops_at_the_saved_group_work_limit() {
    // The evaluator fails a condition past 4096 group visits, so pruning can't
    // decide it and keeps it as written, without walking every reference
    let condition = json!({"$savedGroup": {"id": "g0"}});
    for levels in [13, 30] {
        let pruned = prune_condition(&json!({"id": "123"}), &condition, &doubling_groups(levels));
        assert_eq!(pruned, condition, "{levels} levels");
    }
    // Within the limit it is still decided
    assert_eq!(prune_condition(&json!({"id": "123"}), &condition, &doubling_groups(5)), Value::Bool(true));
}

/// Run on a thread with a normal 2 MiB stack, so unbounded recursion fails the test.
fn on_small_stack(run: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new().stack_size(2 * 1024 * 1024).spawn(run).unwrap().join().unwrap();
}

/// A chain of condition groups `g1` → `g2` → … ending in `{"country": "US"}`.
fn chain(
    groups: &mut Map<String, Value>,
    length: usize,
) {
    for level in 1..length {
        let next = json!({"$savedGroup": {"id": format!("g{}", level + 1)}});
        groups.insert(format!("g{level}"), json!({"type": "condition", "condition": {"$or": [{"id": "x"}, next]}}));
    }
    groups.insert(format!("g{length}"), json!({"type": "condition", "condition": {"country": "US"}}));
}

#[test]
fn long_group_chains_reduce_without_exhausting_the_stack() {
    on_small_stack(|| {
        // Referenced from $elemMatch, every group in the chain keeps its condition as written
        let mut groups = Map::new();
        chain(&mut groups, 20_000);
        let groups = Value::Object(groups);
        let payload = json!({
            "features": {"f": {"defaultValue": false, "rules": [{"condition": {"items": {"$elemMatch": {"$savedGroup": {"id": "g1"}}}}, "force": true}]}},
            "savedGroups": groups.clone(),
        });
        let reduced = reduce_payload(&payload, &json!({"id": "u"}), &ReducePayloadOptions::default());
        assert_eq!(reduced["savedGroups"], groups);

        // Referenced from the user's condition, an $or branch that passes drops the rest of the chain
        let mut groups = Map::new();
        chain(&mut groups, 20_000);
        groups.insert(
            String::from("g0"),
            json!({"type": "condition", "condition": {"$and": [{"$or": [{"id": "u"}, {"$savedGroup": {"id": "g1"}}]}, {"country": "US"}]}}),
        );
        let payload = json!({
            "features": {"f": {"defaultValue": false, "rules": [{"condition": {"$savedGroup": {"id": "g0"}}, "force": true}]}},
            "savedGroups": groups,
        });
        let reduced = reduce_payload(&payload, &json!({"id": "u"}), &ReducePayloadOptions::default());
        assert_eq!(reduced["savedGroups"], json!({"g0": {"type": "condition", "condition": {"$and": [{"country": "US"}]}}}));
    });
}

/// The rules left for feature `f` after reducing a one-rule force payload for `attributes`.
fn reduced_rules(
    attributes: Value,
    condition: Value,
) -> Value {
    let payload = json!({"features": {"f": {"defaultValue": false, "rules": [{"condition": condition, "force": true}]}}});
    reduce_payload(&payload, &attributes, &ReducePayloadOptions::default())["features"]["f"]["rules"].clone()
}

#[test]
fn elem_match_conditions_see_array_element_indices_and_length() {
    // As in JS getPath, an array element exposes its indices and length as fields
    for condition in [
        json!({"x": {"$elemMatch": {"$or": [{"0": 1}]}}}),
        json!({"x": {"$elemMatch": {"$or": [{"length": 1}]}}}),
        json!({"x": {"$elemMatch": {"$and": [{"0": 1}, {"length": {"$gt": 0}}]}}}),
        json!({"x": {"$elemMatch": {"$nor": [{"0": 2}]}}}),
        json!({"x": {"$elemMatch": {"$savedGroup": {"id": "g"}}}}),
    ] {
        let groups = json!({"g": {"type": "list", "attributeKey": "0", "values": [1]}});
        assert_eq!(prune_condition(&json!({"x": [[1]]}), &condition, &groups), Value::Bool(true), "{condition}");
    }
    assert_eq!(
        prune_condition(&json!({"x": [[1]]}), &json!({"x": {"$elemMatch": {"$or": [{"1": 1}]}}}), &json!({})),
        Value::Bool(false)
    );
    assert_eq!(reduced_rules(json!({"x": [[1]]}), json!({"x": {"$elemMatch": {"$or": [{"0": 1}]}}})), json!([{"force": true}]));
}

#[test]
fn unsigned_numbers_beyond_i64_stay_numbers() {
    let big = json!(9_223_372_036_854_775_808_u64);
    assert_eq!(prune_condition(&json!({"x": big}), &json!({"x": {"$eq": null}}), &json!({})), Value::Bool(false));
    assert_eq!(prune_condition(&json!({"x": big}), &json!({"x": {"$exists": true}}), &json!({})), Value::Bool(true));
    assert_eq!(prune_condition(&json!({"x": big}), &json!({"x": {"$gt": 1}}), &json!({})), Value::Bool(true));
    assert_eq!(prune_condition(&json!({"x": 9.3e18}), &json!({"x": {"$gt": big}}), &json!({})), Value::Bool(true));
    assert_eq!(prune_condition(&json!({"x": 1}), &json!({"x": {"$lt": big}}), &json!({})), Value::Bool(true));
    assert_eq!(prune_condition(&json!({"x": big}), &json!({"x": {"$eq": 9_223_372_036_854_775_808.0}}), &json!({})), Value::Bool(true));
    assert_ne!(reduced_rules(json!({"x": big}), json!({"x": {"$eq": null}})), json!([{"force": true}]));
}
