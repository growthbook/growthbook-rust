use growthbook_rust::client::GrowthBookClientBuilder;
use growthbook_rust::client::GrowthBookClientTrait;
use growthbook_rust::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_encrypted_features() {
    let _ = tracing_subscriber::fmt::try_init();
    // 1. Setup Mock Server
    let mock_server = MockServer::start().await;
    let sdk_key = "test_key";

    // 2. Sample Data
    let key_str = "Ns04T5n9+59rl2x3SlNHtQ==";
    let encrypted_string = "vMSg2Bj/IurObDsWVmvkUg==.L6qtQkIzKDoE2Dix6IAKDcVel8PHUnzJ7JjmLjFZFQDqidRIoCxKmvxvUj2kTuHFTQ3/NJ3D6XhxhXXv2+dsXpw5woQf0eAgqrcxHrbtFORs18tRXRZza7zqgzwvcznx";

    // 3. Mock Response
    let response_body = json!({
        "status": 200,
        "encryptedFeatures": encrypted_string
    });

    Mock::given(method("GET"))
        .and(path(format!("/api/features/{}", sdk_key)))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    // 4. Initialize Client with Decryption Key
    let client = GrowthBookClientBuilder::new()
        .api_url(mock_server.uri())
        .client_key(sdk_key.to_string())
        .decryption_key(key_str.to_string())
        .build()
        .await
        .expect("Failed to build client");

    // 5. Verify Feature
    // Expected JSON:
    // {
    //     "testfeature1": {
    //         "defaultValue": true,
    //         "rules": [{"condition": { "id": "1234" }, "force": false}]
    //       }
    // }
    
    assert!(client.is_on("testfeature1", None));
    assert!(client.is_off("testfeature1", Some(vec![GrowthBookAttribute::new("id".to_string(), GrowthBookAttributeValue::String("1234".to_string()))])));
}
