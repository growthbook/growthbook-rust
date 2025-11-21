use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

pub struct TypeComparison;

impl TypeComparison {
    pub fn matches(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
    ) -> bool {
        if let GrowthBookAttributeValue::String(feature_type) = &feature_attribute.value {
            if let Some(user_value) = user_attributes.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
                match user_value {
                    GrowthBookAttributeValue::String(_) => feature_type == "string",
                    GrowthBookAttributeValue::Int(_) => feature_type == "number",
                    GrowthBookAttributeValue::Float(_) => feature_type == "number",
                    GrowthBookAttributeValue::Bool(_) => feature_type == "boolean",
                    GrowthBookAttributeValue::Array(_) => feature_type == "array",
                    GrowthBookAttributeValue::Object(it) => {
                        if it.is_empty() {
                            feature_type == "null"
                        } else {
                            feature_type == "object"
                        }
                    },
                    GrowthBookAttributeValue::Empty => feature_type == "null",
                }
            } else {
                feature_type == "null"
            }
        } else {
            false
        }
    }
}
