use crate::condition::eval_context::ConditionEvalContext;
use crate::condition::use_case::ConditionsMatchesAttributes;
use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

pub struct ElemMatchComparison;

impl ElemMatchComparison {
    pub fn matches(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
        _array_size: bool,
        recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &ConditionEvalContext, bool) -> bool,
    ) -> bool {
        let GrowthBookAttributeValue::Object(condition) = &feature_attribute.value else {
            return false;
        };
        let Some(GrowthBookAttributeValue::Array(items)) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) else {
            return false;
        };
        let operators = !condition.is_empty() && condition.iter().all(|field| field.key.starts_with('$'));
        items.iter().any(|item| {
            if matches!(item, GrowthBookAttributeValue::Empty) {
                return false;
            }
            if operators {
                let attributes = [GrowthBookAttribute::new(String::from("value"), item.clone())];
                let nested = ctx.with_attributes(&attributes);
                condition.iter().all(|field| recursive(Some(&attributes[0]), field, &nested, false))
            } else if let GrowthBookAttributeValue::Object(attributes) = item {
                condition.matches(&ctx.with_attributes(attributes))
            } else {
                false
            }
        })
    }
}
