use crate::condition::eval_context::ConditionEvalContext;
use crate::condition::use_case::{any_of, evaluate, is_operator_object, matches_value, Tri, CONDITION_OPERATORS};
use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

pub struct ElemMatchComparison;

impl ElemMatchComparison {
    pub fn matches(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> Tri {
        let GrowthBookAttributeValue::Object(condition) = &feature_attribute.value else {
            return Some(false);
        };
        let Some(GrowthBookAttributeValue::Array(items)) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) else {
            return Some(false);
        };
        // Operators test each element itself. A condition, including one that
        // only uses $or, $and, $nor, or $savedGroup, tests each element's fields.
        let on_value = is_operator_object(condition) && !condition.iter().any(|field| CONDITION_OPERATORS.contains(&field.key.as_str()));
        any_of(
            items
                .iter()
                // Only skip null elements; falsy values like 0, "" and false are still tested.
                .filter(|item| !matches!(item, GrowthBookAttributeValue::Empty))
                .map(|item| {
                    if on_value {
                        matches_value(item, &feature_attribute.value, ctx, false)
                    } else if let GrowthBookAttributeValue::Object(attributes) = item {
                        evaluate(condition, &ctx.with_attributes(attributes))
                    } else {
                        // Like JS getPath on a primitive, every field is missing.
                        evaluate(condition, &ctx.with_attributes(&[]))
                    }
                }),
        )
    }
}
