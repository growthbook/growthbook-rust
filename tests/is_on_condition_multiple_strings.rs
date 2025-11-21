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
        let on = ctx.growthbook.is_on("flag-not-found", None);

        assert!(!on);

        Ok(())
    }

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_false_when_none_attribute_match(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let vec = GrowthBookAttribute::from(json!({
            "any-id": Uuid::now_v7(),
            "any-key": Uuid::now_v7(),
        }))
        .expect("Failed to create attributes");

        let on = ctx.growthbook.is_on("flag", Some(vec));

        assert!(!on);

        Ok(())
    }

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_false_when_only_id_match(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let vec = GrowthBookAttribute::from(json!({
            "any-id": "018fcf11-bb67-7789-8d10-fcbb7de4ff7b",
            "any-key": Uuid::now_v7(),
        }))
        .expect("Failed to create attributes");

        let on = ctx.growthbook.is_on("flag", Some(vec));

        assert!(!on);

        Ok(())
    }

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_false_when_only_key_match(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let vec = GrowthBookAttribute::from(json!({
            "any-id": Uuid::now_v7(),
            "any-key": "018fcf64-1827-709a-a8ae-7d206aafb5e2",
        }))
        .expect("Failed to create attributes");

        let on = ctx.growthbook.is_on("flag", Some(vec));

        assert!(!on);

        Ok(())
    }

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_true_when_all_attributes_matches(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let vec = GrowthBookAttribute::from(json!({
            "any-id": "018fcf11-bb67-7789-8d10-fcbb7de4ff7b",
            "any-key": "018fcf64-1827-709a-a8ae-7d206aafb5e2",
        }))
        .expect("Failed to create attributes");

        let on = ctx.growthbook.is_on("flag", Some(vec));

        assert!(on);

        Ok(())
    }
}
