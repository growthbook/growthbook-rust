use crate::condition::eval_context::ConditionEvalContext;
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
            GrowthBookAttributeValue::Object(it) => it.iter().all(|next| !recursive(parent_attribute, next, ctx, false)),
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
        _recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &ConditionEvalContext, bool) -> bool,
    ) -> bool {
        match &feature_attribute.value {
            GrowthBookAttributeValue::Array(feature_values) => {
                if let Some(GrowthBookAttributeValue::Array(user_values)) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
                    feature_values.iter().all(|feature_item| {
                        user_values.iter().any(|user_item| {
                            if case_insensitive {
                                match (feature_item, user_item) {
                                    (GrowthBookAttributeValue::String(f), GrowthBookAttributeValue::String(u)) => f.to_lowercase() == u.to_lowercase(),
                                    _ => feature_item == user_item,
                                }
                            } else {
                                feature_item == user_item
                            }
                        })
                    })
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
                GrowthBookAttributeValue::Empty => true,
                // inverse of `eq`: nested-object conditions resolve the parent
                // key to the whole object and rely on the flattened-string
                // comparison rather than structural `PartialEq`.
                GrowthBookAttributeValue::Object(_) => user_value.to_string() == feature_attribute.value.to_string(),
                // Scalars use strict equality, matching JS `actual !== expected`.
                it => it == &feature_attribute.value,
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
                GrowthBookAttributeValue::Empty => false,
                // A nested-object condition like {tags: {hello: "world"}} reaches
                // here via the recursive object path, with the parent key
                // resolving to the whole object; the flattened-string comparison
                // is load-bearing for that case, so keep it for objects.
                GrowthBookAttributeValue::Object(_) => user_value.to_string() == feature_attribute.value.to_string(),
                // Scalars use strict equality, matching JS `actual === expected`:
                // no coercion, so `$eq: 5` does NOT match the string "5". The
                // numeric-coercion path is intentionally kept for $lt/$gt only.
                it => it == &feature_attribute.value,
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
        if let Some(user_value) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
            match &feature_attribute.value {
                GrowthBookAttributeValue::Array(feature_array) => feature_array.iter().any(|feature_item| match &user_value {
                    GrowthBookAttributeValue::Array(user_array) => user_array.iter().any(|user_item| {
                        if case_insensitive {
                            feature_item.to_string().to_lowercase() == user_item.to_string().to_lowercase()
                        } else {
                            feature_item.to_string() == user_item.to_string()
                        }
                    }),
                    GrowthBookAttributeValue::Empty => false,
                    it => {
                        if case_insensitive {
                            feature_item.to_string().to_lowercase() == it.to_string().to_lowercase()
                        } else {
                            feature_item.to_string() == it.to_string()
                        }
                    },
                }),
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn nin(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
        case_insensitive: bool,
        _recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &ConditionEvalContext, bool) -> bool,
    ) -> bool {
        if let Some(user_value) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
            match &feature_attribute.value {
                GrowthBookAttributeValue::Array(feature_array) => feature_array.iter().all(|feature_item| !match &user_value {
                    GrowthBookAttributeValue::Array(user_array) => user_array.iter().any(|user_item| {
                        if case_insensitive {
                            feature_item.to_string().to_lowercase() == user_item.to_string().to_lowercase()
                        } else {
                            feature_item.to_string() == user_item.to_string()
                        }
                    }),
                    GrowthBookAttributeValue::Empty => false,
                    it => {
                        if case_insensitive {
                            feature_item.to_string().to_lowercase() == it.to_string().to_lowercase()
                        } else {
                            feature_item.to_string() == it.to_string()
                        }
                    },
                }),
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn in_group(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        // The condition value is a saved-group id; look it up and test membership.
        if let GrowthBookAttributeValue::String(group_id) = &feature_attribute.value {
            if let Some(members) = ctx.saved_group(group_id) {
                if let Some(user_value) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
                    return value_in_members(&user_value, members);
                }
            }
        }
        false
    }

    pub fn not_in_group(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        // Negation: an unknown group or a missing attribute means "not in group".
        !Self::in_group(parent_attribute, feature_attribute, ctx)
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

// Saved-group membership uses type-strict equality (via `PartialEq`), so the
// string "2" matches "2" but not the integer 2. An array attribute matches if
// any of its elements is a member.
fn value_in_members(
    user_value: &GrowthBookAttributeValue,
    members: &[GrowthBookAttributeValue],
) -> bool {
    match user_value {
        GrowthBookAttributeValue::Array(items) => items.iter().any(|item| members.contains(item)),
        other => members.contains(other),
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
