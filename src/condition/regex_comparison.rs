use crate::condition::eval_context::ConditionEvalContext;
use regex::Regex;

use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

pub struct RegexComparison;

impl RegexComparison {
    pub fn matches(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        if let GrowthBookAttributeValue::String(feature_value) = &feature_attribute.value {
            if let Ok(regex) = Regex::new(feature_value) {
                if let Some(user_value) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
                    match &user_value {
                        GrowthBookAttributeValue::Array(it) => it.iter().any(|item| regex.is_match(&item.to_string())),
                        it => regex.is_match(&it.to_string()),
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            // A non-string regex pattern (e.g. `{x: {$regex: 5}}`) matches nothing.
            // JS `getRegex(expected)` calls `expected.replace(...)` before compiling,
            // so a non-string pattern throws and is caught as `false` — it is *not*
            // coerced to `/5/`. Mirror that: no match.
            false
        }
    }

    pub fn matches_ignore_case(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        if let GrowthBookAttributeValue::String(feature_value) = &feature_attribute.value {
            if let Ok(regex) = regex::RegexBuilder::new(feature_value).case_insensitive(true).build() {
                if let Some(user_value) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
                    match &user_value {
                        GrowthBookAttributeValue::Array(it) => it.iter().any(|item| regex.is_match(&item.to_string())),
                        it => regex.is_match(&it.to_string()),
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            // See `matches`: a non-string regex pattern matches nothing, mirroring
            // JS `getRegex` throwing before it can compile the pattern.
            false
        }
    }

    pub fn not_matches(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        !Self::matches(parent_attribute, feature_attribute, ctx)
    }

    pub fn not_matches_ignore_case(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        !Self::matches_ignore_case(parent_attribute, feature_attribute, ctx)
    }
}
