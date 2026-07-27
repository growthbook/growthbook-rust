use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, CONNECTION};
use reqwest::Client;
#[cfg(feature = "tracing")]
use reqwest_middleware::Extension;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
#[cfg(feature = "tracing")]
use reqwest_tracing::{reqwest_otel_span, OtelName, TracingMiddleware};

use crate::error::GrowthbookError;

pub struct HttpClient;

impl HttpClient {
    pub fn create_http_client(
        #[allow(unused_variables)] name: &str,
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

        #[allow(unused_mut)]
        let mut client_builder = ClientBuilder::new(default_config_client);

        #[cfg(feature = "tracing")]
        {
            // See RedactingSpanBackend below for why not ::default().
            client_builder = client_builder
                .with_init(Extension(OtelName(String::from(name).into())))
                .with(TracingMiddleware::<RedactingSpanBackend>::new());
        }

        Ok(client_builder.build())
    }
}

/// Carries the SDK key so [`RedactingSpanBackend`] can redact it.
#[cfg(feature = "tracing")]
#[derive(Clone)]
pub(crate) struct SdkKeyExtension(pub(crate) String);

/// Like `DefaultSpanBackend`, but on failure redacts the SDK key from
/// `error.message`/`error.cause_chain`, since the URL contains it and
/// `DefaultSpanBackend` records those fields as-is.
#[cfg(feature = "tracing")]
pub(crate) struct RedactingSpanBackend;

#[cfg(feature = "tracing")]
impl reqwest_tracing::ReqwestOtelSpanBackend for RedactingSpanBackend {
    fn on_request_start(
        req: &reqwest::Request,
        ext: &mut http::Extensions,
    ) -> tracing::Span {
        let name = reqwest_tracing::default_span_name(req, ext);
        reqwest_otel_span!(name = name, req)
    }

    fn on_request_end(
        span: &tracing::Span,
        outcome: &reqwest_middleware::Result<reqwest::Response>,
        ext: &mut http::Extensions,
    ) {
        match outcome {
            Ok(response) => reqwest_tracing::default_on_request_success(span, response),
            Err(error) => Self::record_redacted_failure(span, error, ext),
        }
    }
}

#[cfg(feature = "tracing")]
impl RedactingSpanBackend {
    fn record_redacted_failure(
        span: &tracing::Span,
        error: &reqwest_middleware::Error,
        ext: &http::Extensions,
    ) {
        let mut message = error.to_string();
        let mut cause_chain = format!("{error:?}");

        if let Some(SdkKeyExtension(key)) = ext.get::<SdkKeyExtension>() {
            message = message.replace(key.as_str(), "[redacted]");
            cause_chain = cause_chain.replace(key.as_str(), "[redacted]");
        }

        span.record(reqwest_tracing::OTEL_STATUS_CODE, "ERROR");
        span.record(reqwest_tracing::ERROR_MESSAGE, message.as_str());
        span.record(reqwest_tracing::ERROR_CAUSE_CHAIN, cause_chain.as_str());

        if let reqwest_middleware::Error::Reqwest(inner) = error {
            if let Some(status) = inner.status() {
                span.record(reqwest_tracing::HTTP_RESPONSE_STATUS_CODE, status.as_u16());
            }
        }
    }
}
