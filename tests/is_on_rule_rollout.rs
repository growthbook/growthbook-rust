mod commons;

#[cfg(test)]
mod test {
    use growthbook_rust::{client::GrowthBookClientTrait, model_public::GrowthBookAttribute};
    use rstest::rstest;
    use serde_json::json;
    use test_context::test_context;
    use uuid::Uuid;

    use crate::commons::TestContext;

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_default_when_fail_to_call_growthbook(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let on = ctx.growthbook.is_on("flag-not-exists", None);

        assert!(!on);

        Ok(())
    }

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_false_when_percentage_is_0(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let vec = GrowthBookAttribute::from(json!({
            "any-id": Uuid::now_v7(),
        }))
        .expect("Failed to create attributes");

        let on = ctx.growthbook.is_on("rollout-zero-percentage-flag-condition-by-attribute", Some(vec));

        assert!(!on);

        Ok(())
    }

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_true_when_percentage_is_100(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let vec = GrowthBookAttribute::from(json!({
            "any-id": Uuid::now_v7(),
        }))
        .expect("Failed to create attributes");

        let on = ctx.growthbook.is_on("rollout-one-hundred-percentage-flag-condition-by-attribute", Some(vec));

        assert!(on);

        Ok(())
    }

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_true_when_percentage_is_50_and_attribute_is_inside_range(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let vec = GrowthBookAttribute::from(json!({
            "any-id": "018fde8a-77e6-7c15-93d3-d4cc4f018442",
        }))
        .expect("Failed to create attributes");

        let on = ctx.growthbook.is_on("rollout-flag-condition-by-attribute", Some(vec));

        assert!(on);

        Ok(())
    }

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_false_when_percentage_is_50_and_attribute_is_outside_range(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let vec = GrowthBookAttribute::from(json!({
            "any-id": "01901d5f-fc5a-7dd4-9f60-7f0381d4ad33",
        }))
        .expect("Failed to create attributes");

        let on = ctx.growthbook.is_on("rollout-flag-condition-by-attribute", Some(vec));

        assert!(!on);

        Ok(())
    }

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_false_when_required_attribute_is_missing(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let vec = GrowthBookAttribute::from(json!({
            "any-other-id": Uuid::now_v7(),
        }))
        .expect("Failed to create attributes");

        let on = ctx.growthbook.is_on("rollout-flag-condition-by-attribute", Some(vec));

        assert!(!on);

        Ok(())
    }
}
