use crate::condition::eval_context::ConditionEvalContext;

use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

pub struct VersionComparison;

impl VersionComparison {
    pub fn vgt(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, ctx, |feature_version, user_version| user_version.gt(feature_version))
    }

    pub fn vgte(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, ctx, |feature_version, user_version| user_version.ge(feature_version))
    }

    pub fn vlt(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, ctx, |feature_version, user_version| user_version.lt(feature_version))
    }

    pub fn vlte(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, ctx, |feature_version, user_version| user_version.le(feature_version))
    }

    pub fn veq(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, ctx, |feature_version, user_version| user_version.eq(feature_version))
    }

    pub fn vne(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, ctx, |feature_version, user_version| user_version.ne(feature_version))
    }
}

fn evaluate(
    parent_attribute: Option<&GrowthBookAttribute>,
    feature_attribute: &GrowthBookAttribute,
    ctx: &ConditionEvalContext,
    condition: fn(&str, &str) -> bool,
) -> bool {
    let user_version = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key).unwrap_or(GrowthBookAttributeValue::Empty);
    condition(&normalize(&feature_attribute.value), &normalize(&user_version))
}

/// Match the JavaScript SDK's paddedVersionString, including empty segments.
fn normalize(value: &GrowthBookAttributeValue) -> String {
    let version = match value {
        GrowthBookAttributeValue::String(s) if !s.is_empty() => s.clone(),
        // JavaScript stringifies both numeric zero signs as "0".
        GrowthBookAttributeValue::Float(n) if *n == 0.0 => String::from("0"),
        GrowthBookAttributeValue::Int(_) | GrowthBookAttributeValue::Float(_) => value.to_string(),
        _ => String::from("0"),
    };
    let version = version.strip_prefix('v').unwrap_or(&version).split('+').next().unwrap_or("");
    let mut parts = version.split(['-', '.']).collect::<Vec<_>>();
    if parts.len() == 3 {
        parts.push("~");
    }
    parts
        .iter()
        .map(|part| {
            if !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()) {
                format!("{part:>5}")
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("-")
}
