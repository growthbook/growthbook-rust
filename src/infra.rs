use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, CONNECTION};
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, Extension};
use reqwest_tracing::{OtelName, TracingMiddleware};

use crate::error::GrowthbookError;

pub struct HttpClient;

impl HttpClient {
    pub fn create_http_client(
        name: &str,
        timeout_duration: Duration,
    ) -> Result<ClientWithMiddleware, GrowthbookError> {
        let mut default_headers = HeaderMap::new();
        //keep connection alive off by default
        default_headers.insert(CONNECTION, HeaderValue::from_static("close"));

        let default_config_client = Client::builder()
            .timeout(timeout_duration)
            .pool_idle_timeout(None)
            .default_headers(default_headers)
            .build()
            .map_err(GrowthbookError::from)?;

        let client = ClientBuilder::new(default_config_client)
            .with_init(Extension(OtelName(String::from(name).into())))
            .with(TracingMiddleware::default())
            .build();
        Ok(client)
    }
}
