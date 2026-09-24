use crate::condition::eval_context::ConditionEvalContext;
use crate::condition::use_case::matches_value;
use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

pub struct OperatorCondition;

impl OperatorCondition {
    pub fn not(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
        recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &ConditionEvalContext, bool) -> bool,
    ) -> bool {
        match &feature_attribute.value {
            GrowthBookAttributeValue::Object(it) => !it.iter().all(|next| recursive(parent_attribute, next, ctx, false)),
            _ => false,
        }
    }

    pub fn and(
        _parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
        recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &ConditionEvalContext, bool) -> bool,
    ) -> bool {
        and_nor(&feature_attribute, ctx, recursive, false)
    }

    pub fn nor(
        _parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
        recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &ConditionEvalContext, bool) -> bool,
    ) -> bool {
        and_nor(&feature_attribute, ctx, recursive, true)
    }

    pub fn all(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
        case_insensitive: bool,
    ) -> bool {
        match &feature_attribute.value {
            GrowthBookAttributeValue::Array(feature_values) => {
                if let Some(GrowthBookAttributeValue::Array(user_values)) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
                    feature_values
                        .iter()
                        .all(|feature_item| user_values.iter().any(|user_item| matches_value(user_item, feature_item, ctx, case_insensitive)))
                } else {
                    false
                }
            },
            _ => false,
        }
    }

    pub fn ne(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
        _recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &ConditionEvalContext, bool) -> bool,
    ) -> bool {
        if let Some(user_value) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
            !match &user_value {
                GrowthBookAttributeValue::Array(it) => it.iter().any(|item| item == &feature_attribute.value),
                // inverse of `eq`: nested-object conditions resolve the parent
                // key to the whole object and rely on the flattened-string
                // comparison rather than structural `PartialEq`.
                GrowthBookAttributeValue::Object(_) => user_value.to_string() == feature_attribute.value.to_string(),
                // Scalars (and a present `null`, i.e. `Empty`) use JS `===`:
                // `null !== 5` is true, `null !== null` is false, and `Int`/`Float`
                // are one JS `number` type so `5 === 5.0`.
                it => it.strict_eq(&feature_attribute.value),
            }
        } else {
            true
        }
    }

    pub fn eq(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
        _recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &ConditionEvalContext, bool) -> bool,
    ) -> bool {
        if let Some(user_value) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
            match &user_value {
                GrowthBookAttributeValue::Array(it) => it.iter().any(|item| item == &feature_attribute.value),
                // A nested-object condition like {tags: {hello: "world"}} reaches
                // here via the recursive object path, with the parent key
                // resolving to the whole object; the flattened-string comparison
                // is load-bearing for that case, so keep it for objects.
                GrowthBookAttributeValue::Object(_) => user_value.to_string() == feature_attribute.value.to_string(),
                // Scalars (and a present `null`, i.e. `Empty`) use JS `===`: no
                // string coercion (so `$eq: 5` does NOT match "5"), `null === null`
                // but `null !== 5`, and `Int`/`Float` are one JS `number` type so
                // `$eq: 5` matches `5.0`. Coercion stays on $lt/$gt only.
                it => it.strict_eq(&feature_attribute.value),
            }
        } else {
            false
        }
    }

    pub fn exists(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
        _recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &ConditionEvalContext, bool) -> bool,
    ) -> bool {
        if let GrowthBookAttributeValue::Bool(it) = feature_attribute.value {
            if ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key).is_some() {
                it
            } else {
                !it
            }
        } else {
            true
        }
    }

    pub fn is_in(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
        case_insensitive: bool,
        _recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &ConditionEvalContext, bool) -> bool,
    ) -> bool {
        let GrowthBookAttributeValue::Array(members) = &feature_attribute.value else {
            return false;
        };
        let actual = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key).unwrap_or(GrowthBookAttributeValue::Empty);
        value_in_members(&actual, members, case_insensitive)
    }

    pub fn nin(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
        case_insensitive: bool,
        recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &ConditionEvalContext, bool) -> bool,
    ) -> bool {
        if !matches!(feature_attribute.value, GrowthBookAttributeValue::Array(_)) {
            return false;
        }
        !Self::is_in(parent_attribute, feature_attribute, ctx, case_insensitive, recursive)
    }

    pub fn in_group(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        // The condition value is a saved-group id; look it up and test membership.
        if let GrowthBookAttributeValue::String(group_id) = &feature_attribute.value {
            if let Some(members) = ctx.saved_group_values(group_id) {
                let user_value = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key).unwrap_or(GrowthBookAttributeValue::Empty);
                return value_in_members(&user_value, members, false);
            }
        }
        false
    }

    pub fn not_in_group(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        if let GrowthBookAttributeValue::String(group_id) = &feature_attribute.value {
            if let Some(members) = ctx.saved_group_values(group_id) {
                let user_value = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key).unwrap_or(GrowthBookAttributeValue::Empty);
                return !value_in_members(&user_value, members, false);
            }
        }
        false
    }

    pub fn or(
        _parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
        recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &ConditionEvalContext, bool) -> bool,
    ) -> bool {
        match &feature_attribute.value {
            GrowthBookAttributeValue::Array(it) => {
                if it.is_empty() {
                    true
                } else {
                    it.iter().any(|next_value| match next_value {
                        GrowthBookAttributeValue::Object(feature_value) => feature_value.iter().all(|next_attribute| recursive(None, next_attribute, ctx, false)),
                        _ => false,
                    })
                }
            },
            GrowthBookAttributeValue::Empty => true,
            _ => false,
        }
    }
}

/// Membership uses JS numeric equality without coercing strings. An array
/// attribute matches when any of its elements belongs to the group.
pub(super) fn value_in_members(
    user_value: &GrowthBookAttributeValue,
    members: &[GrowthBookAttributeValue],
    case_insensitive: bool,
) -> bool {
    let matches = |actual: &GrowthBookAttributeValue| {
        members.iter().any(|member| match (actual, member) {
            (GrowthBookAttributeValue::String(actual), GrowthBookAttributeValue::String(member)) if case_insensitive => actual.to_lowercase() == member.to_lowercase(),
            // JS includes uses reference identity for objects and nested arrays;
            // independently decoded payload and attribute values cannot share it.
            (GrowthBookAttributeValue::Object(_) | GrowthBookAttributeValue::Array(_), _) => false,
            _ => member.strict_eq(actual),
        })
    };
    match user_value {
        GrowthBookAttributeValue::Array(items) => items.iter().any(matches),
        other => matches(other),
    }
}

fn and_nor(
    feature_attribute: &&GrowthBookAttribute,
    ctx: &ConditionEvalContext,
    recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &ConditionEvalContext, bool) -> bool,
    negate: bool,
) -> bool {
    match &feature_attribute.value {
        GrowthBookAttributeValue::Array(it) => it.iter().all(|next_value| match next_value {
            GrowthBookAttributeValue::Object(feature_value) => {
                let result = feature_value.iter().all(|next_attribute| recursive(None, next_attribute, ctx, false));
                if negate {
                    !result
                } else {
                    result
                }
            },
            _ => false,
        }),
        _ => false,
    }
}
