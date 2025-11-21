mod commons;

#[cfg(test)]
mod test {
    use growthbook_rust::client::GrowthBookClientTrait;
    use rstest::rstest;
    use test_context::test_context;

    use crate::commons::TestContext;

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_default_when_fail_to_call_growthbook(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let result = ctx.growthbook.feature_result("not-found", None);

        assert!(!result.on);
        assert!(result.value.is_null());

        Ok(())
    }

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_value(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let result = ctx.growthbook.feature_result("fixed-value", None);

        assert!(result.on);
        assert!(result.value.is_string());

        let string = result.value_as::<String>()?;
        assert_eq!("018fcf11-bb67-7789-8d10-fcbb7de4ff7b", string);

        Ok(())
    }
}
