mod commons;

#[cfg(test)]
mod test {
    use growthbook_rust::{client::GrowthBookClientTrait, model_public::GrowthBookAttribute};
    use rstest::rstest;
    use serde_json::json;
    use test_context::test_context;

    use crate::commons::TestContext;

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_default_when_fail_to_call_growthbook(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let result = ctx.growthbook.feature_result("flag-not-exist", None);

        assert!(!result.on);

        Ok(())
    }

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_false_when_experiment_variant_is_disabled(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let vec = GrowthBookAttribute::from(json!({
            "any-id": "01901d5c-fb74-743d-a532-ed582d29a7e1",
        }))
        .expect("Failed to create attributes");

        let result = ctx.growthbook.feature_result("experiment-rule-condition-flag", Some(vec));

        assert!(!result.on);
        assert!(result.value.is_boolean());
        assert!(!result.value.as_bool().expect("Failed to convert to bool"));
        assert_eq!("0", result.experiment_result.expect("Failed to get experiment_result").key);

        Ok(())
    }

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_true_when_experiment_variant_is_enabled(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let vec = GrowthBookAttribute::from(json!({
            "any-id": "01901d5e-5b0e-75bf-92a3-7658d932634d",
        }))
        .expect("Failed to create attributes");

        let result = ctx.growthbook.feature_result("experiment-rule-condition-ninety-coverage-flag", Some(vec));

        assert!(result.on);
        assert!(result.value.is_boolean());
        assert!(result.value.as_bool().expect("Failed to convert to bool"));
        assert_eq!("2", result.experiment_result.expect("Failed to get experiment_result").key);

        Ok(())
    }

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_false_when_attribute_is_on_disabled_range(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let vec = GrowthBookAttribute::from(json!({
            "any-id": "018fd040-de77-72c7-af6e-6a67d430c0e6",
        }))
        .expect("Failed to create attributes");

        let result = ctx.growthbook.feature_result("experiment-rule-condition-zero-coverage-flag", Some(vec));

        assert!(!result.on);
        assert!(result.value.is_boolean());
        assert!(!result.value.as_bool().expect("Failed to convert to bool"));
        assert!(result.experiment_result.is_none());

        Ok(())
    }
}
