# GrowthBook Rust SDK

[![Crates.io](https://img.shields.io/crates/v/growthbook-rust)](https://crates.io/crates/growthbook-rust)
[![Docs](https://docs.rs/growthbook-rust/badge.svg)](https://docs.rs/growthbook-rust)

> [!NOTE]
> This repo was originally developed by the [community](https://github.com/will-bank/growthbook-rust-sdk) and later adopted by GrowthBook.

The official GrowthBook SDK for Rust. This crate provides an easy way to integrate feature flagging and experimentation into your Rust applications.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
growthbook-rust = "0.0.1"
```

## Quick Usage

### Initialization

Use the `GrowthBookClientBuilder` to create a client instance. This supports auto-refreshing features, caching, and callbacks.

```rust
use growthbook_rust::client::GrowthBookClientBuilder;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = GrowthBookClientBuilder::new()
        .api_url("https://cdn.growthbook.io".to_string())
        .client_key("sdk-key".to_string())
        .ttl(Duration::from_secs(60)) // Cache TTL
        .auto_refresh(true) // Enable background updates
        .refresh_interval(Duration::from_secs(30))
        .build()
        .await?;

    Ok(())
}
```

### Checking Features

You can check if a feature is enabled or get its value.

```rust
// Simple check
if client.is_on("my-feature", None) {
    println!("Feature is enabled!");
}

// Get typed value
let value = client.feature_result("my-config", None).value_as::<String>()?;
```

### Context & Attributes

You can set global attributes that apply to all evaluations, and override them per-check.

```rust
use std::collections::HashMap;
use growthbook_rust::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

// Global attributes
let mut global_attrs = HashMap::new();
global_attrs.insert("tenantId".to_string(), GrowthBookAttributeValue::String("123".to_string()));

let client = GrowthBookClientBuilder::new()
    .api_url(api_url)
    .client_key(sdk_key)
    .attributes(global_attrs)
    .build()
    .await?;

// Per-check attributes (merged with global)
let mut user_attrs = Vec::new();
user_attrs.push(GrowthBookAttribute::new("userId".to_string(), GrowthBookAttributeValue::String("456".to_string())));

if client.is_on("my-feature", Some(user_attrs)) {
    // ...
}
```

## Tracking Callbacks

You can subscribe to events for tracking and analytics.

```rust
let client = GrowthBookClientBuilder::new()
    // ...
    .on_feature_usage(Box::new(|key, result| {
        println!("Feature '{}' evaluated: {:?}", key, result.value);
    }))
    .on_experiment_viewed(Box::new(|experiment_result| {
        // Track experiment impression
        println!("Experiment viewed: {}", experiment_result.key);
    }))
    .build()
    .await?;
```

## Configuration

The SDK can also be configured via environment variables if not explicitly set in the builder:

| Env Var                | Description                                                                    |
|------------------------|--------------------------------------------------------------------------------|
| GB_HTTP_CLIENT_TIMEOUT | Timeout for HTTP requests. Default: 10s                                        |
| GB_UPDATE_INTERVAL     | Interval for auto-refresh (if enabled). Default: 60s                           |
| GB_URL                 | GrowthBook API URL                                                             |
| GB_SDK_KEY             | GrowthBook SDK Key                                                             |

## Refreshing features & Caching

The SDK supports automated feature updates via a background task. This is enabled by default when using `auto_refresh(true)` in the builder.

- **Caching**: Features are cached in memory by default. You can configure the TTL using `.ttl(Duration::from_secs(60))`.
- **Background Sync**: When `auto_refresh` is enabled, a background task periodically fetches features from the API and updates the cache.
- **On Refresh Callback**: You can listen for updates using `.add_on_refresh(...)`.

## Manual Feature Management

If you prefer to manage feature updates manually or want to start with a specific set of features (e.g., from a file or another source), you can disable auto-refresh and provide initial features.

```rust
use serde_json::json;

let features_json = json!({
    "my-feature": {
        "defaultValue": true
    }
});

let client = GrowthBookClientBuilder::new()
    .api_url(api_url)
    .client_key(sdk_key)
    .auto_refresh(false) // Disable background sync
    .features_json(features_json)? // Set initial features
    .build()
    .await?;

// You can manually refresh features later
client.refresh().await;
```

## Encrypted Features

If you are using encrypted features, you can provide the decryption key to the builder.

```rust
let client = GrowthBookClientBuilder::new()
    .api_url(api_url)
    .client_key(sdk_key)
    .decryption_key("your-decryption-key".to_string())
    .build()
    .await?;
```