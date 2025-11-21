use serde::Deserialize;

mod commons;

#[cfg(test)]
mod test {
    use growthbook_rust::client::GrowthBookClientTrait;
    use rstest::rstest;
    use test_context::test_context;

    use crate::commons::TestContext;
    use crate::ObjectValue;

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_enabled_default_when_fail_to_call_growthbook(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let result = ctx.growthbook.feature_result("flag-not-exists", None);

        assert!(!result.on);
        assert!(result.value.is_null());
        assert!(result.experiment_result.is_none());

        Ok(())
    }

    #[test_context(TestContext)]
    #[rstest]
    #[tokio::test]
    async fn should_return_value(ctx: &mut TestContext) -> Result<(), Box<dyn std::error::Error>> {
        let result = ctx.growthbook.feature_result("object-flag", None);

        assert!(result.on);
        assert!(result.value.is_object());

        let object = result.value_as::<ObjectValue>()?;
        assert_eq!("potato", object.a);
        assert_eq!("tomato", object.b);

        Ok(())
    }
}

#[derive(Deserialize)]
struct ObjectValue {
    a: String,
    b: String,
}
