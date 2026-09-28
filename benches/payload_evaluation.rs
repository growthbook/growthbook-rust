//! Release-mode payload and evaluation benchmark; emits JSON Lines to stdout.
//! BENCH_V2=0 restricts workloads to capabilities available before this PR.

use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

use growthbook_rust::cache::{FeatureCache, InMemoryCache};
use growthbook_rust::client::{GrowthBookClient, GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::dto::GrowthBookResponse;
use growthbook_rust::model_public::GrowthBookAttribute;
use serde_json::{json, Map, Value};
use tokio::runtime::Runtime;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn measure(
    name: &str,
    mut operation: impl FnMut() -> bool,
) {
    let samples: usize = std::env::var("BENCH_SAMPLES").ok().and_then(|v| v.parse().ok()).unwrap_or(9);
    let sample_ms: f64 = std::env::var("BENCH_SAMPLE_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(40.0);
    assert!(samples > 0 && sample_ms > 0.0);
    let mut errors = 0usize;
    let mut batch = 1usize;
    let elapsed = loop {
        let start = Instant::now();
        for _ in 0..batch {
            errors += usize::from(!black_box(operation()));
        }
        let elapsed = start.elapsed();
        if elapsed >= Duration::from_millis(10) || batch >= 1_048_576 {
            break elapsed;
        }
        batch *= 2;
    };
    let iterations = ((sample_ms * 1_000_000.0 * batch as f64 / elapsed.as_nanos() as f64) as usize).clamp(1, 1_048_576);
    let mut timings = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        for _ in 0..iterations {
            errors += usize::from(!black_box(operation()));
        }
        timings.push(start.elapsed().as_nanos() as f64 / iterations as f64);
    }
    let mut sorted = timings.clone();
    sorted.sort_by(f64::total_cmp);
    println!(
        "{}",
        json!({"name": name, "median_ns": sorted[samples / 2], "min_ns": sorted[0], "max_ns": sorted[samples - 1], "samples_ns": timings, "iterations_per_sample": iterations, "errors": errors})
    );
}

fn offline(
    runtime: &Runtime,
    bytes: &[u8],
) -> GrowthBookClient {
    let response: GrowthBookResponse = serde_json::from_slice(bytes).expect("payload");
    runtime
        .block_on(
            GrowthBookClientBuilder::new()
                .features(response.features.unwrap_or_default())
                .saved_groups(response.saved_groups.unwrap_or(Value::Null))
                .build(),
        )
        .expect("offline client")
}

#[allow(clippy::too_many_arguments)]
fn bench_payload(
    runtime: &Runtime,
    name: &str,
    payload: Value,
    attributes: Value,
    key: &str,
    expected: Value,
    encryption_key: Option<&str>,
    http: bool,
) {
    let bytes = serde_json::to_vec(&payload).expect("serialize");
    let attributes = Some(GrowthBookAttribute::from(attributes).expect("attributes"));
    let encrypted = payload.get("encryptedFeatures").is_some();
    println!("{}", json!({"workload": name, "payload_bytes": bytes.len()}));
    measure(&format!("{name}/decode_json"), || {
        black_box(serde_json::from_slice::<GrowthBookResponse>(black_box(&bytes)).expect("decode"));
        true
    });
    if !encrypted {
        measure(&format!("{name}/load_offline"), || {
            black_box(offline(runtime, black_box(&bytes)));
            true
        });
    }

    let cache = Arc::new(InMemoryCache::new(Duration::from_secs(3600)));
    runtime.block_on(cache.set("features", serde_json::from_slice(&bytes).unwrap()));
    let mut builder = GrowthBookClientBuilder::new().api_url("http://127.0.0.1:1".to_owned()).client_key("benchmark".to_owned()).cache(cache);
    if let Some(key) = encryption_key {
        builder = builder.decryption_key(key.to_owned());
    }
    let client = runtime.block_on(builder.build()).expect("cached initial load");
    assert_eq!(client.feature_result(key, attributes.clone()).value, expected, "{name}");
    measure(&format!("{name}/refresh_cached"), || runtime.block_on(client.try_refresh()).is_ok());

    let gb = client.gb.read().unwrap().clone();
    measure(&format!("{name}/evaluate_core"), || {
        black_box(gb.check(black_box(key), black_box(&attributes)));
        true
    });
    measure(&format!("{name}/evaluate_client"), || {
        black_box(client.feature_result(black_box(key), black_box(attributes.clone())));
        true
    });
    assert_eq!(gb.check(key, &attributes).value, expected);

    if http {
        let server = runtime.block_on(MockServer::start());
        runtime.block_on(
            Mock::given(method("GET"))
                .and(path("/api/features/benchmark"))
                .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes))
                .mount(&server),
        );
        let mut builder = GrowthBookClientBuilder::new().api_url(server.uri()).client_key("benchmark".to_owned()).ttl(Duration::ZERO);
        if let Some(key) = encryption_key {
            builder = builder.decryption_key(key.to_owned());
        }
        let client = runtime.block_on(builder.build()).expect("HTTP initial load");
        measure(&format!("{name}/refresh_http"), || runtime.block_on(client.try_refresh()).is_ok());
        assert_eq!(client.feature_result(key, attributes).value, expected);
    }
}

fn synthetic(
    features: usize,
    members: usize,
    format: &str,
) -> Value {
    let values: Vec<_> = (0..members).map(|i| json!(format!("user-{i}"))).collect();
    let (condition, groups) = match format {
        "plain" => (json!({"country": "US"}), json!({})),
        "legacy" => (json!({"id": {"$inGroup": "members"}}), json!({"members": values})),
        "v2-list" => (json!({"$savedGroup": {"id": "members"}}), json!({"members": {"type": "list", "attributeKey": "id", "values": values}})),
        "v2-condition" => (
            json!({"$savedGroup": {"id": "nested"}}),
            json!({
                "nested": {"type": "condition", "condition": {"$savedGroup": {"id": "eligible"}}},
                "eligible": {"type": "condition", "condition": {"$and": [{"country": "US"}, {"$savedGroup": {"id": "members"}}]}},
                "members": {"type": "list", "attributeKey": "id", "values": values}
            }),
        ),
        _ => unreachable!(),
    };
    let mut flags = Map::new();
    for i in 0..features {
        flags.insert(format!("flag-{i}"), json!({"defaultValue": false}));
    }
    flags.insert("target".to_owned(), json!({"defaultValue": false, "rules": [{"condition": condition, "force": true}]}));
    json!({"features": flags, "savedGroups": groups})
}

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().expect("runtime");
    let v2 = std::env::var("BENCH_V2").as_deref() != Ok("0");
    let filter = std::env::var("BENCH_FILTER").unwrap_or_default();
    for (features, members) in [(100, 1000), (1000, 10000)] {
        for format in ["plain", "legacy", "v2-list", "v2-condition"] {
            if !v2 && format.starts_with("v2") {
                continue;
            }
            let name = format!("synthetic-{features}-{members}-{format}");
            if !name.contains(&filter) {
                continue;
            }
            bench_payload(
                &runtime,
                &name,
                synthetic(features, members, format),
                json!({"id": format!("user-{}", members - 1), "country": "US"}),
                "target",
                json!(true),
                None,
                true,
            );
        }
    }
    let fixtures: Value = serde_json::from_str(include_str!("../tests/fixtures/saved_group_server_payloads.json")).expect("fixtures");
    for entry in fixtures["payloads"].as_array().unwrap() {
        let format = entry["name"].as_str().unwrap();
        if !v2 && !["inline", "inline-encrypted", "referencesV1"].contains(&format) {
            continue;
        }
        let name = format!("server-{format}");
        if !name.contains(&filter) {
            continue;
        }
        bench_payload(
            &runtime,
            &name,
            entry["payload"].clone(),
            fixtures["evaluations"][0]["attributes"].clone(),
            "list",
            fixtures["evaluations"][0]["features"]["list"]["value"].clone(),
            fixtures["decryptionKey"].as_str(),
            true,
        );
    }
}
