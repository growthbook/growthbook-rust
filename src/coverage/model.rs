use serde_json::Value;

use crate::hash::{HashCode, HashCodeVersion};
use crate::model_public::{FeatureResult, GrowthBookAttributeValue};
use crate::range::model::Range;

pub struct Coverage;

impl Coverage {
    pub fn check(
        value: &GrowthBookAttributeValue,
        option_coverage: Option<f32>,
        option_range: Option<Range>,
        feature_name: &str,
        hash_version: Option<i64>,
        force_value: Value,
    ) -> Option<FeatureResult> {
        if let Some(user_weight) = HashCode::hash_code(&value.to_string(), feature_name, HashCodeVersion::from(hash_version)) {
            if let Some(range) = option_range {
                if range.in_range(&user_weight) {
                    Some(FeatureResult::force(force_value.clone()))
                } else {
                    None
                }
            } else if let Some(coverage) = option_coverage {
                if coverage.gt(&user_weight) {
                    Some(FeatureResult::force(force_value.clone()))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}
