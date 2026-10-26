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
        evaluate(parent_attribute, feature_attribute, ctx, false, |feature_version, user_version| user_version.gt(feature_version))
    }

    pub fn vgte(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, ctx, false, |feature_version, user_version| user_version.ge(feature_version))
    }

    pub fn vlt(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, ctx, false, |feature_version, user_version| user_version.lt(feature_version))
    }

    pub fn vlte(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, ctx, false, |feature_version, user_version| user_version.le(feature_version))
    }

    pub fn veq(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, ctx, false, |feature_version, user_version| user_version.eq(feature_version))
    }

    pub fn vne(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        ctx: &ConditionEvalContext,
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, ctx, true, |feature_version, user_version| user_version.ne(feature_version))
    }
}

/// Both sides must be strings, or both numbers. On any other pairing the
/// comparison fails, so only `$vne` (the negation of `$veq`) matches.
fn evaluate(
    parent_attribute: Option<&GrowthBookAttribute>,
    feature_attribute: &GrowthBookAttribute,
    ctx: &ConditionEvalContext,
    on_type_mismatch: bool,
    condition: fn(&str, &str) -> bool,
) -> bool {
    let Some(user_version) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) else {
        return on_type_mismatch;
    };
    let comparable = match (&feature_attribute.value, &user_version) {
        (GrowthBookAttributeValue::String(_), GrowthBookAttributeValue::String(_)) => true,
        (feature, user) => feature.as_number().is_some() && user.as_number().is_some(),
    };
    if !comparable {
        return on_type_mismatch;
    }
    condition(&normalize(&feature_attribute.value), &normalize(&user_version))
}

/// Match the JavaScript SDK's paddedVersionString, including empty segments.
fn normalize(value: &GrowthBookAttributeValue) -> String {
    let version = match value {
        GrowthBookAttributeValue::String(s) if !s.is_empty() => s.clone(),
        GrowthBookAttributeValue::Int(_) | GrowthBookAttributeValue::Float(_) => js_number_to_string(value.as_number().unwrap_or_default()),
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

/// Format a number like JavaScript's `String(n)` (ECMAScript Number::toString):
/// the shortest round-trip digits, in plain notation for exponents from 1e-7
/// up to 1e21, and scientific notation such as `1e-7` or `1.5e+21` otherwise.
fn js_number_to_string(n: f64) -> String {
    if n.is_nan() {
        return String::from("NaN");
    }
    if n == 0.0 {
        return String::from("0");
    }
    if n < 0.0 {
        return format!("-{}", js_number_to_string(-n));
    }
    if n.is_infinite() {
        return String::from("Infinity");
    }
    // `{:e}` gives the shortest round-trip digits, e.g. "1.23e-18"
    let scientific = format!("{n:e}");
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let digits = mantissa.replace('.', "");
    let k = digits.len() as i32;
    // n is the decimal exponent with the value written as 0.digits × 10^n
    let n = exponent.parse::<i32>().unwrap_or_default() + 1;
    if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat(-n as usize))
    } else {
        let sign = if n - 1 < 0 { '-' } else { '+' };
        let fraction = if k > 1 { format!(".{}", &digits[1..]) } else { String::new() };
        format!("{}{fraction}e{sign}{}", &digits[..1], (n - 1).abs())
    }
}

#[cfg(test)]
mod tests {
    use super::js_number_to_string;

    #[test]
    fn js_number_to_string_matches_javascript() {
        for (number, expected) in [
            (1.0, "1"),
            (1.5, "1.5"),
            (1e21, "1e+21"),
            (1e-7, "1e-7"),
            (0.000002, "0.000002"),
            (123e-20, "1.23e-18"),
            (1e23, "1e+23"),
            (-0.0, "0"),
            (-1.5, "-1.5"),
            (123456789012345680000.0, "123456789012345680000"),
            (1.5e21, "1.5e+21"),
            (0.1, "0.1"),
            (9007199254740993.0, "9007199254740992"),
        ] {
            assert_eq!(js_number_to_string(number), expected, "{number}");
        }
    }
}
