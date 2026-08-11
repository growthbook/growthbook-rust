#![cfg(feature = "tracing")]

use std::fmt;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use growthbook_rust::client::GrowthBookClientBuilder;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::Subscriber;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::{Layer, Registry};

/// Captures recorded span field values so tests can assert a secret
/// never appears in one.
#[derive(Clone, Default)]
struct RecordedFieldValues(Arc<Mutex<Vec<String>>>);

impl RecordedFieldValues {
    fn contains(
        &self,
        needle: &str,
    ) -> bool {
        self.0.lock().unwrap().iter().any(|value| value.contains(needle))
    }
}

impl Visit for RecordedFieldValues {
    fn record_str(
        &mut self,
        _field: &Field,
        value: &str,
    ) {
        self.0.lock().unwrap().push(value.to_string());
    }

    fn record_debug(
        &mut self,
        _field: &Field,
        value: &dyn fmt::Debug,
    ) {
        self.0.lock().unwrap().push(format!("{value:?}"));
    }
}

struct CaptureLayer(RecordedFieldValues);

impl<S: Subscriber> Layer<S> for CaptureLayer {
    fn on_new_span(
        &self,
        attrs: &Attributes<'_>,
        _id: &Id,
        _ctx: Context<'_, S>,
    ) {
        let mut recorder = self.0.clone();
        attrs.record(&mut recorder);
    }

    fn on_record(
        &self,
        _id: &Id,
        values: &Record<'_>,
        _ctx: Context<'_, S>,
    ) {
        let mut recorder = self.0.clone();
        values.record(&mut recorder);
    }
}

/// Regression test: on a failed request, `reqwest-tracing`'s span must
/// not record the SDK key (embedded in the URL) via error.message/cause_chain.
#[tokio::test]
async fn tracing_span_does_not_leak_sdk_key_on_connection_failure() -> Result<(), Box<dyn std::error::Error>> {
    let recorded = RecordedFieldValues::default();
    let subscriber = Registry::default().with(CaptureLayer(recorded.clone()));
    let _guard = tracing::subscriber::set_default(subscriber);

    // A port nothing is listening on, so the request fails fast with a
    // connection error instead of waiting on a timeout.
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);

    let sdk_key = "sdk-tracing-redaction-test-key";

    let client = GrowthBookClientBuilder::new()
        .api_url(format!("http://127.0.0.1:{port}"))
        .client_key(sdk_key.to_string())
        .auto_refresh(false)
        .build()
        .await;

    assert!(client.is_err(), "expected the initial load to fail against a closed port");

    assert!(
        !recorded.contains(sdk_key),
        "the SDK key leaked into a tracing span field - captured values: {:?}",
        recorded.0.lock().unwrap()
    );

    Ok(())
}
