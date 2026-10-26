use crate::condition::eval_context::ConditionEvalContext;
use crate::condition::use_case::{all_of, any_of, implicit_equals, is_operator_object, matches_value, Tri};
use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue, SavedGroup};

pub struct OperatorCondition;

type Recursive = fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &ConditionEvalContext) -> Tri;

impl OperatorCondition {
    pub fn not(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
        recursive: Recursive,
    ) -> Tri {
        let result = match (&feature_attribute.value, parent_attribute) {
            (GrowthBookAttributeValue::Object(it), None) => all_of(it.iter().map(|next| recursive(None, next, ctx))),
            (GrowthBookAttributeValue::Object(it), Some(_)) if is_operator_object(it) => all_of(it.iter().map(|next| recursive(parent_attribute, next, ctx))),
            // Any other value is an implicit equality check on the attribute.
            (expected, Some(parent)) => implicit_equals(&parent.key, expected, ctx),
            // JS throws on `{"$not": null}`
            (GrowthBookAttributeValue::Empty, None) => return malformed(ctx),
            _ => return Some(false),
        };
        result.map(|it| !it)
    }

    pub fn and(
        _parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
        recursive: Recursive,
    ) -> Tri {
        match &feature_attribute.value {
            GrowthBookAttributeValue::Array(it) => all_of(it.iter().map(|next| each_condition(next, ctx, recursive))),
            GrowthBookAttributeValue::Empty => malformed(ctx),
            _ => Some(false),
        }
    }

    pub fn nor(
        _parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
        recursive: Recursive,
    ) -> Tri {
        match &feature_attribute.value {
            GrowthBookAttributeValue::Array(it) => any_of(it.iter().map(|next| each_condition(next, ctx, recursive))).map(|it| !it),
            GrowthBookAttributeValue::Empty => malformed(ctx),
            _ => Some(false),
        }
    }

    pub fn all(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
        case_insensitive: bool,
    ) -> Tri {
        match &feature_attribute.value {
            GrowthBookAttributeValue::Array(feature_values) => {
                if let Some(GrowthBookAttributeValue::Array(user_values)) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
                    all_of(
                        feature_values
                            .iter()
                            .map(|feature_item| any_of(user_values.iter().map(|user_item| matches_value(user_item, feature_item, ctx, case_insensitive)))),
                    )
                } else {
                    Some(false)
                }
            },
            _ => Some(false),
        }
    }

    pub fn ne(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        !Self::eq(parent_attribute, feature_attribute, ctx)
    }

    pub fn eq(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        // JS `===`: no coercion between types, and arrays and objects are
        // never equal to a condition value. `Int`/`Float` are one JS `number`
        // type, so `$eq: 5` matches `5.0`. Like JS getPath, a missing
        // attribute is null.
        match ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key).unwrap_or(GrowthBookAttributeValue::Empty) {
            GrowthBookAttributeValue::Array(_) | GrowthBookAttributeValue::Object(_) => false,
            user_value => user_value.strict_eq(&feature_attribute.value),
        }
    }

    pub fn exists(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        if let GrowthBookAttributeValue::Bool(it) = feature_attribute.value {
            // Like JS `actual != null`, a present null does not exist.
            if ctx
                .find_value(&parent_attribute.unwrap_or(feature_attribute).key)
                .is_some_and(|value| value != GrowthBookAttributeValue::Empty)
            {
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
    ) -> bool {
        if !matches!(feature_attribute.value, GrowthBookAttributeValue::Array(_)) {
            return false;
        }
        !Self::is_in(parent_attribute, feature_attribute, ctx, case_insensitive)
    }

    pub fn in_group(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> Tri {
        group_membership(parent_attribute, feature_attribute, ctx).map(|(found, _)| found)
    }

    pub fn not_in_group(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> Tri {
        // No values means matching nobody. An empty list would pass everyone.
        group_membership(parent_attribute, feature_attribute, ctx).map(|(found, listed)| listed && !found)
    }

    pub fn or(
        _parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
        recursive: Recursive,
    ) -> Tri {
        match &feature_attribute.value {
            GrowthBookAttributeValue::Array(it) if it.is_empty() => Some(true),
            GrowthBookAttributeValue::Array(it) => any_of(it.iter().map(|next| each_condition(next, ctx, recursive))),
            GrowthBookAttributeValue::Empty => malformed(ctx),
            _ => Some(false),
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

/// One condition of an `$and`, `$or`, or `$nor` list. Anything but an object fails.
fn each_condition(
    condition: &GrowthBookAttributeValue,
    ctx: &ConditionEvalContext,
    recursive: Recursive,
) -> Tri {
    match condition {
        GrowthBookAttributeValue::Object(fields) => all_of(fields.iter().map(|field| recursive(None, field, ctx))),
        GrowthBookAttributeValue::Empty => malformed(ctx),
        _ => Some(false),
    }
}

/// A part JS throws on, such as a null `$or` list or a null condition in one.
/// It fails the whole condition, so `$not` and `$nor` can't turn it into a pass.
fn malformed(ctx: &ConditionEvalContext) -> Tri {
    ctx.set_malformed();
    Some(false)
}

/// Whether the attribute is in a legacy or list saved group, and whether the
/// group has values at all. `None` for a group type this SDK doesn't know.
fn group_membership(
    parent_attribute: Option<&GrowthBookAttribute>,
    feature_attribute: &GrowthBookAttribute,
    ctx: &ConditionEvalContext,
) -> Option<(bool, bool)> {
    // The condition value is a saved-group id; look it up and test membership.
    let GrowthBookAttributeValue::String(group_id) = &feature_attribute.value else {
        return Some((false, false));
    };
    if matches!(ctx.saved_group(group_id), Some(SavedGroup::Unknown)) {
        return None;
    }
    let Some(members) = ctx.saved_group_values(group_id) else {
        return Some((false, false));
    };
    let user_value = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key).unwrap_or(GrowthBookAttributeValue::Empty);
    Some((value_in_members(&user_value, members, false), true))
}
