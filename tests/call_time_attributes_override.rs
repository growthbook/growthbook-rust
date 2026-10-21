use std::collections::HashMap;

use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};
use serde_json::json;

// Call-time attributes override global attributes with the same key, as in JS
// where per-call attributes replace the context's.
#[tokio::test]
async fn call_time_attributes_override_global_attributes() {
    let features_json = json!({
        "f": {
            "defaultValue": "default",
            "rules": [
                { "condition": { "country": "US" }, "force": "us" },
                { "condition": { "country": "CA" }, "force": "ca" }
            ]
        }
    });
    let global = HashMap::from([
        (
            String::from("country"),
            GrowthBookAttribute::new(String::from("country"), GrowthBookAttributeValue::String(String::from("CA"))),
        ),
        (String::from("id"), GrowthBookAttribute::new(String::from("id"), GrowthBookAttributeValue::String(String::from("u1")))),
    ]);

    let client = GrowthBookClientBuilder::new()
        .features_json(features_json)
        .unwrap()
        .attributes(global)
        .build()
        .await
        .expect("Failed to build client");

    assert_eq!(client.feature_result("f", None).value, json!("ca"));
    let call_time = GrowthBookAttribute::from(json!({ "country": "US" })).unwrap();
    assert_eq!(client.feature_result("f", Some(call_time)).value, json!("us"));
}
