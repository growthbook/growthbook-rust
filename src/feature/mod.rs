pub mod feature_rule_experiment;
pub mod feature_rule_force;
mod feature_rule_parent;
pub mod feature_rule_rollout;
pub mod use_case;

use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

/// Resolve the attribute to hash on, mirroring JS `getHashAttribute`
/// (core.ts): use `hashAttribute` (default "id"); if the user is missing it,
/// try `fallbackAttribute` only when `fallback_allowed` (a sticky bucket
/// service is configured and the rule doesn't disable sticky bucketing).
/// Returns the chosen attribute name and its value, or None to skip the rule —
/// never silently falling back to "id".
pub(crate) fn resolve_hash_attribute(
    hash_attribute: &Option<String>,
    fallback_attribute: &Option<String>,
    fallback_allowed: bool,
    user_attributes: &Vec<GrowthBookAttribute>,
) -> Option<(String, GrowthBookAttributeValue)> {
    let attribute = hash_attribute.clone().unwrap_or(String::from("id"));
    if let Some(value) = user_attributes.find_value(&attribute) {
        return Some((attribute, value));
    }

    if fallback_allowed {
        if let Some(fallback) = fallback_attribute {
            if let Some(value) = user_attributes.find_value(fallback) {
                return Some((fallback.clone(), value));
            }
        }
    }

    None
}
