use std::time::Duration;

use reqwest::header::USER_AGENT;
use reqwest_middleware::ClientWithMiddleware;

use crate::dto::GrowthBookResponse;
use crate::env::Environment;
use crate::error::GrowthbookError;
use crate::infra::HttpClient;

#[derive(Clone, Debug)]
pub struct GrowthbookGateway {
    pub url: String,
    pub user_agent: String,
    sdk_key: String,
    pub client: ClientWithMiddleware,
}
impl GrowthbookGateway {
    pub fn new(
        url: &str,
        sdk_key: &str,
        timeout: Duration,
    ) -> Result<Self, GrowthbookError> {
        Ok(Self {
            url: String::from(url),
            user_agent: format!(
                "{}/{}",
                Environment::string_or_default("CARGO_PKG_NAME", "growthbook-rust-sdk"),
                Environment::string_or_default("CARGO_PKG_VERSION", "1.0.0")
            ),
            client: HttpClient::create_http_client("growthbook", timeout)?,
            sdk_key: sdk_key.to_string(),
        })
    }

    pub async fn get_features(
        &self,
        sdk_key: Option<&str>,
    ) -> Result<GrowthBookResponse, GrowthbookError> {
        let sdk = sdk_key.unwrap_or(self.sdk_key.as_str());
        let url = format!("{}/api/features/{}", self.url, sdk);

        let request = self.client.get(url).header(USER_AGENT, self.user_agent.clone());

        // Lets RedactingSpanBackend redact the key from failed-request spans.
        #[cfg(feature = "tracing")]
        let request = request.with_extension(crate::infra::SdkKeyExtension(sdk.to_string()));

        let send_result = request.send().await.map_err(|e| Self::redact_key(GrowthbookError::from(e), sdk))?;

        let status = send_result.status();

        if !status.is_success() {
            // From<Response> below can't read the body (see its doc comment),
            // so an error payload can't be mistaken for an empty success.
            return Err(Self::redact_key(GrowthbookError::from(send_result), sdk));
        }

        let response = send_result.json::<GrowthBookResponse>().await.map_err(|e| Self::redact_key(GrowthbookError::from(e), sdk))?;

        Ok(response)
    }

    /// Strips the SDK key (embedded in the URL) out of an error's message.
    fn redact_key(
        error: GrowthbookError,
        key: &str,
    ) -> GrowthbookError {
        // Guard against an empty key: `str::replace("", ..)` would splice the
        // placeholder between every character.
        if key.is_empty() {
            return error;
        }
        GrowthbookError {
            message: error.message.replace(key, "[redacted]"),
            ..error
        }
    }
}
