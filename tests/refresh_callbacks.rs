use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Duration;

use growthbook_rust::cache::{FeatureCache, InMemoryCache};
use growthbook_rust::client::{GrowthBookClient, GrowthBookClientBuilder, GrowthBookClientTrait};
use rstest::rstest;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[rstest]
#[case::network(false)]
#[case::cache(true)]
#[tokio::test]
async fn refresh_callbacks_can_evaluate_committed_features(#[case] cached: bool) {
    let server = MockServer::start().await;
    let cache = Arc::new(InMemoryCache::new(if cached { Duration::from_secs(60) } else { Duration::ZERO }));
    let client_slot = Arc::new(OnceLock::<Weak<GrowthBookClient>>::new());
    let notifications = Arc::new(Mutex::new(Vec::new()));
    let mut builder = GrowthBookClientBuilder::new()
        .api_url(server.uri())
        .client_key("callback-test".to_owned())
        .cache(cache.clone())
        .auto_refresh(false)
        .features_json(json!({"flag": {"defaultValue": false}, "old": {"defaultValue": true}}))
        .expect("initial features");

    for index in 0..2 {
        let client_slot = client_slot.clone();
        let notifications = notifications.clone();
        builder = builder.add_on_refresh(Box::new(move || {
            let client = client_slot.get().expect("client installed before refresh").upgrade().expect("live client");
            // Fail finitely on the regression instead of blocking this test's runtime.
            // Drop the probe before exercising the public evaluation workflow.
            drop(client.gb.try_read().expect("refresh callback must be able to read committed features"));
            assert!(client.is_on("flag", None));
            assert!(client.feature_result("flag", None).value_as::<bool>().expect("boolean flag"));
            assert!(!client.is_on("old", None), "the previous feature set was replaced");
            notifications.lock().expect("notifications").push(index);
        }));
    }

    let client = Arc::new(builder.build().await.expect("manual initial load"));
    client_slot.set(Arc::downgrade(&client)).expect("install client");
    assert!(!client.is_on("flag", None));
    assert!(notifications.lock().expect("notifications").is_empty());

    let payload = json!({"features": {"flag": {"defaultValue": true}}});
    if cached {
        cache.set("features", serde_json::from_value(payload.clone()).expect("cached payload")).await;
    }
    Mock::given(method("GET"))
        .and(path("/api/features/callback-test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(payload))
        .expect(if cached { 0 } else { 1 })
        .mount(&server)
        .await;

    client.try_refresh().await.expect("refresh completes after callbacks");
    assert_eq!(*notifications.lock().expect("notifications"), vec![0, 1]);
    assert!(client.is_on("flag", None));
    assert_eq!(server.received_requests().await.expect("requests").len(), if cached { 0 } else { 1 });

    // A rejected replacement must neither commit its features nor notify observers.
    server.reset().await;
    let invalid = json!({"features": {"flag": {"defaultValue": false}}, "encryptedSavedGroups": "invalid"});
    if cached {
        cache.set("features", serde_json::from_value(invalid.clone()).expect("invalid cached payload")).await;
    }
    Mock::given(method("GET"))
        .and(path("/api/features/callback-test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(invalid))
        .expect(if cached { 0 } else { 2 })
        .mount(&server)
        .await;

    assert!(client.try_refresh().await.is_err());
    client.refresh().await;
    assert!(client.is_on("flag", None), "failed refresh preserves committed features");
    assert_eq!(*notifications.lock().expect("notifications"), vec![0, 1], "failed refresh does not invoke callbacks");
    if !cached {
        assert!(client.cache.as_ref().expect("cache").get("features").await.is_none(), "rejected response is not cached");
    }
    drop(client);
    assert!(client_slot.get().expect("client slot").upgrade().is_none(), "callbacks must not retain the client");
}
