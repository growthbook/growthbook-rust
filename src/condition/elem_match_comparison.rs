use crate::condition::eval_context::ConditionEvalContext;
use crate::condition::use_case::{is_operator_object, matches_value, ConditionsMatchesAttributes};
use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

pub struct ElemMatchComparison;

impl ElemMatchComparison {
    pub fn matches(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        let GrowthBookAttributeValue::Object(condition) = &feature_attribute.value else {
            return false;
        };
        let Some(GrowthBookAttributeValue::Array(items)) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) else {
            return false;
        };
        items.iter().any(|item| {
            if matches!(item, GrowthBookAttributeValue::Empty) {
                return false;
            }
            if is_operator_object(condition) {
                matches_value(item, &feature_attribute.value, ctx, false)
            } else if let GrowthBookAttributeValue::Object(attributes) = item {
                condition.matches(&ctx.with_attributes(attributes))
            } else {
                false
            }
        })
    }
}
