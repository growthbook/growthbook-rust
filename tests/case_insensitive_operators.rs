use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::GrowthBookAttribute;
use serde_json::json;

#[tokio::test]
async fn test_case_insensitive_operators() {
    let features_json = json!({
        "ini-test": {
            "defaultValue": false,
            "rules": [
                {
                    "condition": {
                        "tag": {
                            "$ini": ["Foo", "Bar", "115"]
                        }
                    },
                    "force": true
                }
            ]
        },
        "nini-test": {
            "defaultValue": false,
            "rules": [
                {
                    "condition": {
                        "tag": {
                            "$nini": ["Foo", "Bar"]
                        }
                    },
                    "force": true
                }
            ]
        },
        "alli-test": {
            "defaultValue": false,
            "rules": [
                {
                    "condition": {
                        "tags": {
                            "$alli": ["Foo", "Bar"]
                        }
                    },
                    "force": true
                }
            ]
        },
        "regexi-test": {
            "defaultValue": false,
            "rules": [
                {
                    "condition": {
                        "email": {
                            "$regexi": "^[a-z]+@example\\.com$"
                        }
                    },
                    "force": true
                }
            ]
        },
        "not-regex-test": {
            "defaultValue": false,
            "rules": [
                {
                    "condition": {
                        "email": {
                            "$notRegex": "^TEST@EXAMPLE\\.COM$"
                        }
                    },
                    "force": true
                }
            ]
        },
        "not-regexi-test": {
            "defaultValue": false,
            "rules": [
                {
                    "condition": {
                        "email": {
                            "$notRegexi": "^test@example\\.com$"
                        }
                    },
                    "force": true
                }
            ]
        }
    });

    let client = GrowthBookClientBuilder::new().features_json(features_json).unwrap().build().await.expect("Failed to build client");

    // Test $ini
    // Case 1: Exact match - should pass
    let created_attrs = GrowthBookAttribute::from(json!({"tag": "Foo"})).unwrap();
    assert!(client.is_on("ini-test", Some(created_attrs)));

    // Case 2: Case insensitive match - should pass
    let created_attrs = GrowthBookAttribute::from(json!({"tag": "foo"})).unwrap();
    assert!(client.is_on("ini-test", Some(created_attrs)));
    let created_attrs = GrowthBookAttribute::from(json!({"tag": "BAR"})).unwrap();
    assert!(client.is_on("ini-test", Some(created_attrs)));
    let created_attrs = GrowthBookAttribute::from(json!({"tag": "115"})).unwrap();
    assert!(client.is_on("ini-test", Some(created_attrs)));

    // Case 3: No match - should fail
    let created_attrs = GrowthBookAttribute::from(json!({"tag": "Baz"})).unwrap();
    assert!(!client.is_on("ini-test", Some(created_attrs)));

    // Test $nini
    // Case 1: Exact match - should fail (because it's in the list)
    let created_attrs = GrowthBookAttribute::from(json!({"tag": "Foo"})).unwrap();
    assert!(!client.is_on("nini-test", Some(created_attrs)));

    // Case 2: Case insensitive match - should fail
    let created_attrs = GrowthBookAttribute::from(json!({"tag": "foo"})).unwrap();
    assert!(!client.is_on("nini-test", Some(created_attrs)));

    // Case 3: No match - should pass
    let created_attrs = GrowthBookAttribute::from(json!({"tag": "Baz"})).unwrap();
    assert!(client.is_on("nini-test", Some(created_attrs)));

    // Test $alli
    // Case 1: All match mixed case - should pass
    let created_attrs = GrowthBookAttribute::from(json!({"tags": ["foo", "bar", "baz"]})).unwrap();
    assert!(client.is_on("alli-test", Some(created_attrs)));
    let created_attrs = GrowthBookAttribute::from(json!({"tags": ["FOO", "BaR"]})).unwrap();
    assert!(client.is_on("alli-test", Some(created_attrs)));

    // Case 2: One missing - should fail
    let created_attrs = GrowthBookAttribute::from(json!({"tags": ["foo", "baz"]})).unwrap();
    assert!(!client.is_on("alli-test", Some(created_attrs)));

    // Test $regexi
    // Case 1: Exact match - should pass
    let created_attrs = GrowthBookAttribute::from(json!({"email": "test@example.com"})).unwrap();
    assert!(client.is_on("regexi-test", Some(created_attrs)));

    // Case 2: Mixed case match - should pass
    let created_attrs = GrowthBookAttribute::from(json!({"email": "TEST@example.com"})).unwrap();
    assert!(client.is_on("regexi-test", Some(created_attrs)));
    let created_attrs = GrowthBookAttribute::from(json!({"email": "test@EXAMPLE.com"})).unwrap();
    assert!(client.is_on("regexi-test", Some(created_attrs)));

    // Case 3: No match - should fail
    let created_attrs = GrowthBookAttribute::from(json!({"email": "test@other.com"})).unwrap();
    assert!(!client.is_on("regexi-test", Some(created_attrs)));

    // Test $notRegex
    // Case 1: Exact match - should fail
    let created_attrs = GrowthBookAttribute::from(json!({"email": "TEST@EXAMPLE.COM"})).unwrap();
    assert!(!client.is_on("not-regex-test", Some(created_attrs)));

    // Case 2: Lowercase (no match because it is case sensitive) - should pass
    let created_attrs = GrowthBookAttribute::from(json!({"email": "test@example.com"})).unwrap();
    assert!(client.is_on("not-regex-test", Some(created_attrs)));

    // Test $notRegexi
    // Case 1: Exact match - should fail
    let created_attrs = GrowthBookAttribute::from(json!({"email": "test@example.com"})).unwrap();
    assert!(!client.is_on("not-regexi-test", Some(created_attrs)));

    // Case 2: Mixed case match - should fail
    let created_attrs = GrowthBookAttribute::from(json!({"email": "TEST@EXAMPLE.COM"})).unwrap();
    assert!(!client.is_on("not-regexi-test", Some(created_attrs)));

    // Case 3: No match - should pass
    let created_attrs = GrowthBookAttribute::from(json!({"email": "other@example.com"})).unwrap();
    assert!(client.is_on("not-regexi-test", Some(created_attrs)));
}
