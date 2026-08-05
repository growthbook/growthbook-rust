use std::collections::HashMap;
use std::sync::Arc;

use growthbook_rust::dto::GrowthBookFeature;
use growthbook_rust::growthbook::GrowthBook;
use growthbook_rust::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};
use growthbook_rust::sticky_bucket::{InMemoryStickyBucketService, StickyBucketService};
use serde_json::json;

// #20: when a user already has a sticky-bucket assignment, JS runExperiment
// skips the namespace and condition checks (they live inside
// `if (!foundStickyBucket)`). So a user with an existing assignment who no
// longer matches a newly-added targeting condition must stay bucketed.
//
// Here the experiment rule has `condition: {country: "US"}` and the user has no
// `country`, but the user already has a sticky assignment for the rule. Expected
// (JS parity): the sticky variation ("treat") is served. Before the fix, Rust
// evaluated the condition first, excluded the user, and returned the default.
#[tokio::test]
async fn sticky_assignment_wins_over_unmatched_condition() {
    let service = Arc::new(InMemoryStickyBucketService::new());
    // Pre-seed: rule key "exp1", bucketVersion 0 -> "exp1__0" -> variation 1.
    let mut seed = HashMap::new();
    seed.insert("exp1__0".to_string(), "1".to_string());
    service.save_assignments("id", "abc", seed);

    let features: HashMap<String, GrowthBookFeature> = serde_json::from_value(json!({
        "f": {
            "defaultValue": "off",
            "rules": [{
                "key": "exp1",
                "variations": ["control", "treat"],
                "condition": { "country": "US" }
            }]
        }
    }))
    .unwrap();

    let gb = GrowthBook {
        forced_variations: None,
        features,
        attributes: None,
        sticky_bucket_service: Some(service.clone()),
        saved_groups: Default::default(),
    };

    // User has `id` (the hash attribute) but no `country`, so the condition fails.
    let user_attrs = vec![GrowthBookAttribute::new("id".to_string(), GrowthBookAttributeValue::String("abc".to_string()))];

    let result = gb.check("f", &Some(user_attrs));

    assert_eq!(result.value, json!("treat"), "sticky assignment must win over the unmatched condition");
    let exp = result.experiment_result.expect("expected an experiment result (sticky bucket)");
    assert!(exp.in_experiment, "user should be in the experiment via sticky bucket");
    assert!(exp.sticky_bucket_used, "result should be flagged as sticky-bucket-used");
}
