use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use growthbook_rust::dto::GrowthBookFeature;
use growthbook_rust::growthbook::GrowthBook;
use growthbook_rust::model_public::{FeatureResult, GrowthBookAttribute, GrowthBookAttributeValue};
use growthbook_rust::sticky_bucket::{InMemoryStickyBucketService, StickyBucketService};

// -----------------------------------------------------------------------------
// Data Structures
// -----------------------------------------------------------------------------

#[derive(Deserialize)]
struct AllCases {
    #[serde(rename = "stickyBucket")]
    sticky_bucket: Vec<Value>,
}

#[derive(Deserialize)]
#[allow(dead_code)] // Used implicitly during JSON deserialization
struct TestCase {
    name: String,
    context: TestContext,
    existing_assignments: Vec<StickyAssignment>,
    feature_key: String,
    expected_result: Option<ExpectedResult>,
    expected_sticky_assignments: HashMap<String, StickyAssignmentDoc>,
}

#[derive(Deserialize)]
struct TestContext {
    attributes: HashMap<String, Value>,
    features: HashMap<String, GrowthBookFeature>,
}

#[derive(Deserialize)]
struct StickyAssignment {
    #[serde(rename = "attributeName")]
    attribute_name: String,
    #[serde(rename = "attributeValue")]
    attribute_value: String,
    assignments: HashMap<String, String>,
}

#[derive(Deserialize)]
struct StickyAssignmentDoc {
    #[allow(dead_code)]
    #[serde(rename = "attributeName")]
    attribute_name: String,
    #[allow(dead_code)]
    #[serde(rename = "attributeValue")]
    attribute_value: String,
    assignments: HashMap<String, String>,
}

#[derive(Deserialize)]
struct ExpectedResult {
    value: Value,
    #[serde(rename = "inExperiment")]
    in_experiment: bool,
    #[serde(rename = "stickyBucketUsed")]
    sticky_bucket_used: bool,
}

// -----------------------------------------------------------------------------
// Helper Functions
// -----------------------------------------------------------------------------

/// Loads and parses the sticky bucket test cases from the JSON file.
fn load_test_cases() -> Vec<TestCase> {
    let content = fs::read_to_string("tests/all_cases.json").expect("Failed to read all_cases.json");
    let all_cases: AllCases = serde_json::from_str(&content).expect("Failed to parse sections");

    all_cases.sticky_bucket.into_iter().map(|v| parse_test_case(&v)).collect()
}

/// Parses a single raw JSON value into a structured `TestCase`.
fn parse_test_case(case_value: &Value) -> TestCase {
    let array = case_value.as_array().expect("Case should be an array");
    // Manual parsing because the top-level array has mixed types
    let name = array[0].as_str().unwrap().to_string();
    let context: TestContext = serde_json::from_value(array[1].clone()).unwrap();
    let existing_assignments: Vec<StickyAssignment> = serde_json::from_value(array[2].clone()).unwrap();
    let feature_key = array[3].as_str().unwrap().to_string();
    let expected_result: Option<ExpectedResult> = serde_json::from_value(array[4].clone()).unwrap();
    let expected_sticky_assignments: HashMap<String, StickyAssignmentDoc> = serde_json::from_value(array[5].clone()).unwrap();

    TestCase {
        name,
        context,
        existing_assignments,
        feature_key,
        expected_result,
        expected_sticky_assignments,
    }
}

/// Sets up the sticky bucket service and populates it with existing assignments.
fn load_sticky_bucket_service(assignments: &[StickyAssignment]) -> Arc<InMemoryStickyBucketService> {
    let service = Arc::new(InMemoryStickyBucketService::new());
    for assignment in assignments {
        service.save_assignments(&assignment.attribute_name, &assignment.attribute_value, assignment.assignments.clone());
    }
    service
}

/// Verifies that the actual feature result matches expectations.
fn verify_feature_result(
    case_name: &str,
    actual: &FeatureResult,
    expected: &Option<ExpectedResult>,
) {
    if let Some(exp) = expected {
        assert_eq!(actual.value, exp.value, "Value mismatch for case: {}", case_name);
        if let Some(exp_res) = &actual.experiment_result {
            assert_eq!(exp_res.in_experiment, exp.in_experiment, "InExperiment mismatch for case: {}", case_name);
            assert_eq!(exp_res.sticky_bucket_used, exp.sticky_bucket_used, "StickyBucketUsed mismatch for case: {}", case_name);
        } else if exp.in_experiment {
            panic!("Expected inExperiment=true but got None for case: {}", case_name);
        }
    } else {
        // Expecting null/None (blocked/disabled)
        assert!(actual.experiment_result.is_none(), "Expected no experiment result for case: {}", case_name);
    }
}

/// Verifies that the service's state matches the expected sticky assignments.
fn verify_assignments(
    case_name: &str,
    service: &dyn StickyBucketService,
    expected_docs: &HashMap<String, StickyAssignmentDoc>,
) {
    for (doc_key, doc) in expected_docs {
        let parts: Vec<&str> = doc_key.split("||").collect();
        let attr_name = parts[0];
        let attr_value = parts[1];

        if !doc.assignments.is_empty() {
            let actual = service
                .get_assignments(attr_name, attr_value)
                .unwrap_or_else(|| panic!("Expected assignments for {} in case {}", doc_key, case_name));

            assert_eq!(actual, doc.assignments, "Assignment mismatch for {} in case {}", doc_key, case_name);
        }
    }
}

// -----------------------------------------------------------------------------
// Main Test Runner
// -----------------------------------------------------------------------------

#[test]
fn test_sticky_bucket_scenarios() {
    let cases = load_test_cases();

    for case in cases {
        println!("Running test case: {}", case.name);
        let service = load_sticky_bucket_service(&case.existing_assignments);
        // Prepare User Attributes
        let mut user_attrs = Vec::new();
        for (k, v) in case.context.attributes {
            user_attrs.push(GrowthBookAttribute::new(k, GrowthBookAttributeValue::from(v)));
        }

        // Initialize GrowthBook
        let gb = GrowthBook {
            forced_variations: None,
            features: case.context.features,
            attributes: None,
            sticky_bucket_service: Some(service.clone()),
        };

        // Execute Check
        let result = gb.check(&case.feature_key, &Some(user_attrs));

        // Verify
        verify_feature_result(&case.name, &result, &case.expected_result);
        verify_assignments(&case.name, service.as_ref(), &case.expected_sticky_assignments);
    }
}
