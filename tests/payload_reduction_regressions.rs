use growthbook_rust::payload::prune_condition;
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
