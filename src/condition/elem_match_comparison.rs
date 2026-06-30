use crate::condition::eval_context::ConditionEvalContext;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

pub struct ElemMatchComparison;

impl ElemMatchComparison {
    pub fn matches(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
        array_size: bool,
        recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &ConditionEvalContext, bool) -> bool,
    ) -> bool {
        match &feature_attribute.value {
            GrowthBookAttributeValue::Object(it) => it.iter().any(|condition_attribute| recursive(parent_attribute, condition_attribute, ctx, array_size)),
            _ => false,
        }
    }
}
