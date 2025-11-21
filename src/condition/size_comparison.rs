use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

pub struct SizeComparison;

impl SizeComparison {
    pub fn matches(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
        recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &[GrowthBookAttribute], bool) -> bool,
    ) -> bool {
        match &feature_attribute.value {
            GrowthBookAttributeValue::Int(feature_value) => {
                if let Some(GrowthBookAttributeValue::Array(user_value)) = user_attributes.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
                    feature_value == &(user_value.len() as i64)
                } else {
                    false
                }
            },
            GrowthBookAttributeValue::Object(feature_value) => feature_value.iter().all(|next| recursive(parent_attribute, next, user_attributes, true)),
            _ => false,
        }
    }
}
