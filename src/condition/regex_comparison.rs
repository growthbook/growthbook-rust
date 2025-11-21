use regex::Regex;

use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

pub struct RegexComparison;

impl RegexComparison {
    pub fn matches(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
    ) -> bool {
        if let GrowthBookAttributeValue::String(feature_value) = &feature_attribute.value {
            if let Ok(regex) = Regex::new(feature_value) {
                if let Some(user_value) = user_attributes.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
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
            true
        }
    }
}
