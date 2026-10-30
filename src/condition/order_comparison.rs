use std::cmp::Ordering;

use crate::condition::eval_context::ConditionEvalContext;
use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

pub struct OrderComparison;

impl OrderComparison {
    pub fn gt(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, ctx, |ordering| ordering == Ordering::Greater)
    }

    pub fn gte(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, ctx, |ordering| ordering != Ordering::Less)
    }

    pub fn lt(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, ctx, |ordering| ordering == Ordering::Less)
    }

    pub fn lte(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, ctx, |ordering| ordering != Ordering::Greater)
    }
}

/// Compares the user value against the condition value. Both must be numbers
/// or both strings; any other pairing (including a missing attribute, null,
/// booleans and arrays) fails without coercion.
fn evaluate(
    parent_attribute: Option<&GrowthBookAttribute>,
    feature_attribute: &GrowthBookAttribute,
    ctx: &ConditionEvalContext,
    condition: fn(Ordering) -> bool,
) -> bool {
    let Some(user_value) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) else {
        return false;
    };
    compare(&user_value, &feature_attribute.value).is_some_and(condition)
}

/// Orders two values the way JavaScript's relational operators do when both
/// sides share a type: numerically for numbers, by UTF-16 code units for strings.
fn compare(
    user_value: &GrowthBookAttributeValue,
    feature_value: &GrowthBookAttributeValue,
) -> Option<Ordering> {
    if let (GrowthBookAttributeValue::String(user), GrowthBookAttributeValue::String(feature)) = (user_value, feature_value) {
        return Some(user.encode_utf16().cmp(feature.encode_utf16()));
    }
    user_value.as_number()?.partial_cmp(&feature_value.as_number()?)
}
