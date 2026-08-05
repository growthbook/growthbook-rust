mod commons;

#[cfg(test)]
mod test {
    use growthbook_rust::client::{GrowthBookClient, GrowthBookClientBuilder, GrowthBookClientTrait};
    use uuid::Uuid;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Regression test: a non-2xx response (e.g. invalid key) must not
    /// deserialize as an empty success and wipe out build()'s state.
    #[tokio::test]
    async fn build_fails_when_initial_refresh_gets_a_non_2xx_response() -> Result<(), Box<dyn std::error::Error>> {
        let mock_server = MockServer::start().await;
        let sdk_key = Uuid::now_v7();

        Mock::given(method("GET"))
            .and(path(format!("/api/features/{sdk_key}")))
            .respond_with(ResponseTemplate::new(400).set_body_raw(r#"{"status":400,"error":"Invalid API Key"}"#, "application/json"))
            .mount(&mock_server)
            .await;

        let result = GrowthBookClientBuilder::new()
            .api_url(mock_server.uri())
            .client_key(sdk_key.to_string())
            .auto_refresh(false)
            .build()
            .await;

        assert!(
            result.is_err(),
            "build() should return Err when the initial refresh gets a non-2xx response, not silently succeed with zero features"
        );

        Ok(())
    }

    /// A failed refresh() must not clear out previously loaded features.
    #[tokio::test]
    async fn refresh_does_not_clear_existing_features_on_non_2xx_response() -> Result<(), Box<dyn std::error::Error>> {
        let mock_server = MockServer::start().await;
        let sdk_key = Uuid::now_v7();

        // First response: a healthy load with a feature turned on.
        Mock::given(method("GET"))
            .and(path(format!("/api/features/{sdk_key}")))
            .respond_with(ResponseTemplate::new(200).set_body_raw(r#"{"features":{"my-feature":{"defaultValue":true,"rules":[]}}}"#, "application/json"))
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;

        // Every subsequent request: an invalid-key-style error response.
        Mock::given(method("GET"))
            .and(path(format!("/api/features/{sdk_key}")))
            .respond_with(ResponseTemplate::new(400).set_body_raw(r#"{"status":400,"error":"Invalid API Key"}"#, "application/json"))
            .mount(&mock_server)
            .await;

        let client = GrowthBookClient::new(&mock_server.uri(), sdk_key.to_string().as_str(), None, None).await?;

        assert!(client.is_on("my-feature", None), "initial load should have picked up the feature");

        let refresh_result = client.refresh().await;
        assert!(refresh_result.is_err(), "refresh() should surface the non-2xx response as an error");

        assert!(client.is_on("my-feature", None), "a failed refresh must not clear out previously loaded features");

        Ok(())
    }
}
