use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

pub struct ElemMatchComparison;

impl ElemMatchComparison {
    pub fn matches(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
        array_size: bool,
        recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &[GrowthBookAttribute], bool) -> bool,
    ) -> bool {
        match &feature_attribute.value {
            GrowthBookAttributeValue::Object(it) => it.iter().any(|condition_attribute| recursive(parent_attribute, condition_attribute, user_attributes, array_size)),
            _ => false,
        }
    }
}
