use crate::condition::eval_context::ConditionEvalContext;
use crate::condition::use_case::matches_value;
use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

pub struct SizeComparison;

impl SizeComparison {
    pub fn matches(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        if let Some(GrowthBookAttributeValue::Array(items)) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
            matches_value(&GrowthBookAttributeValue::Int(items.len() as i64), &feature_attribute.value, ctx, false)
        } else {
            false
        }
    }
}
