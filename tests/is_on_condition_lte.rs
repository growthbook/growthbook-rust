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
        let on = ctx.growthbook.is_on("lte-flag-not-exist", None);

        assert!(!on);

        Ok(())
    }

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_true_when_is_equals(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let vec = GrowthBookAttribute::from(json!({
            "version": "1.2.3"
        }))
        .expect("Failed to create attributes");

        let on = ctx.growthbook.is_on("lte-flag", Some(vec));

        assert!(on);

        Ok(())
    }

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_true_when_is_less_then(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let vec = GrowthBookAttribute::from(json!({
            "version": "1.2.2"
        }))
        .expect("Failed to create attributes");

        let on = ctx.growthbook.is_on("lte-flag", Some(vec));

        assert!(on);

        Ok(())
    }

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_false_when_is_greater_then(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let vec = GrowthBookAttribute::from(json!({
            "version": "1.2.4"
        }))
        .expect("Failed to create attributes");

        let on = ctx.growthbook.is_on("lte-flag", Some(vec));

        assert!(!on);

        Ok(())
    }

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_true_when_attribute_is_missing(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let vec = GrowthBookAttribute::from(json!({
            "version": "1.2.4"
        }))
        .expect("Failed to create attributes");

        let on = ctx.growthbook.is_on("lte-flag", Some(vec));

        assert!(!on);

        Ok(())
    }
}
