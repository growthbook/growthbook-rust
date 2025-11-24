use growthbook_rust::client::GrowthBookClientBuilder;
use growthbook_rust::client::GrowthBookClientTrait;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use aes::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};
use base64::{engine::general_purpose, Engine as _};

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;

#[tokio::test]
async fn test_encrypted_features() {
    // 1. Setup Mock Server
    let mock_server = MockServer::start().await;
    let sdk_key = "test_key";

    // 2. Prepare Payload
    let features = json!({
        "encrypted-feature": {
            "defaultValue": true
        }
    });
    let plaintext = features.to_string();
    
    // 3. Encrypt Payload
    let key_bytes = [0u8; 16]; // All zeros key
    let iv_bytes = [0u8; 16]; // All zeros IV
    let key_str = general_purpose::STANDARD.encode(key_bytes);
    let iv_str = general_purpose::STANDARD.encode(iv_bytes);

    let encryptor = Aes128CbcEnc::new(&key_bytes.into(), &iv_bytes.into());
    let mut buffer = [0u8; 4096];
    let pos = buffer.len();
    let ciphertext_len = encryptor.encrypt_padded_b2b_mut::<Pkcs7>(plaintext.as_bytes(), &mut buffer).unwrap().len();
    let ciphertext = &buffer[..ciphertext_len];
    let ciphertext_str = general_purpose::STANDARD.encode(ciphertext);
    
    let encrypted_string = format!("{}.{}", iv_str, ciphertext_str);

    // 4. Mock Response
    let response_body = json!({
        "status": 200,
        "encryptedFeatures": encrypted_string
    });

    Mock::given(method("GET"))
        .and(path(format!("/api/features/{}", sdk_key)))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    // 5. Initialize Client with Decryption Key
    let client = GrowthBookClientBuilder::new()
        .api_url(mock_server.uri())
        .client_key(sdk_key.to_string())
        .decryption_key(key_str)
        .build()
        .await
        .expect("Failed to build client");

    // 6. Verify Feature
    assert!(client.is_on("encrypted-feature", None));
}
