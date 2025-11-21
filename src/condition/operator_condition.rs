use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

pub struct OperatorCondition;

impl OperatorCondition {
    pub fn not(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
        recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &[GrowthBookAttribute], bool) -> bool,
    ) -> bool {
        match &feature_attribute.value {
            GrowthBookAttributeValue::Object(it) => it.iter().all(|next| !recursive(parent_attribute, next, user_attributes, false)),
            _ => false,
        }
    }

    pub fn and(
        _parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
        recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &[GrowthBookAttribute], bool) -> bool,
    ) -> bool {
        and_nor(&feature_attribute, user_attributes, recursive, false)
    }

    pub fn nor(
        _parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
        recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &[GrowthBookAttribute], bool) -> bool,
    ) -> bool {
        and_nor(&feature_attribute, user_attributes, recursive, true)
    }

    pub fn all(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
        _recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &[GrowthBookAttribute], bool) -> bool,
    ) -> bool {
        match &feature_attribute.value {
            GrowthBookAttributeValue::Array(feature_values) => {
                if let Some(GrowthBookAttributeValue::Array(user_values)) = user_attributes.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
                    feature_values.iter().all(|feature_item| user_values.iter().any(|user_item| feature_item == user_item))
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
        user_attributes: &[GrowthBookAttribute],
        _recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &[GrowthBookAttribute], bool) -> bool,
    ) -> bool {
        if let Some(user_value) = user_attributes.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
            !match &user_value {
                GrowthBookAttributeValue::Array(it) => it.iter().any(|item| item == &feature_attribute.value),
                GrowthBookAttributeValue::Empty => true,
                it => it == &feature_attribute.value,
            }
        } else {
            true
        }
    }

    pub fn eq(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
        _recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &[GrowthBookAttribute], bool) -> bool,
    ) -> bool {
        if let Some(user_value) = user_attributes.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
            match &user_value {
                GrowthBookAttributeValue::Array(it) => it.iter().any(|item| item == &feature_attribute.value),
                GrowthBookAttributeValue::Empty => false,
                it => it.to_string() == feature_attribute.value.to_string(),
            }
        } else {
            false
        }
    }

    pub fn exists(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
        _recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &[GrowthBookAttribute], bool) -> bool,
    ) -> bool {
        if let GrowthBookAttributeValue::Bool(it) = feature_attribute.value {
            if user_attributes.find_value(&parent_attribute.unwrap_or(feature_attribute).key).is_some() {
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
        user_attributes: &[GrowthBookAttribute],
        _recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &[GrowthBookAttribute], bool) -> bool,
    ) -> bool {
        if let Some(user_value) = user_attributes.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
            match &feature_attribute.value {
                GrowthBookAttributeValue::Array(feature_array) => feature_array.iter().any(|feature_item| match &user_value {
                    GrowthBookAttributeValue::Array(user_array) => user_array.iter().any(|user_item| feature_item.to_string() == user_item.to_string()),
                    GrowthBookAttributeValue::Empty => false,
                    it => feature_item.to_string() == it.to_string(),
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
        user_attributes: &[GrowthBookAttribute],
        _recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &[GrowthBookAttribute], bool) -> bool,
    ) -> bool {
        if let Some(user_value) = user_attributes.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
            match &feature_attribute.value {
                GrowthBookAttributeValue::Array(feature_array) => feature_array.iter().all(|feature_item| !match &user_value {
                    GrowthBookAttributeValue::Array(user_array) => user_array.iter().any(|user_item| feature_item.to_string() == user_item.to_string()),
                    GrowthBookAttributeValue::Empty => false,
                    it => feature_item.to_string() == it.to_string(),
                }),
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn or(
        _parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
        recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &[GrowthBookAttribute], bool) -> bool,
    ) -> bool {
        match &feature_attribute.value {
            GrowthBookAttributeValue::Array(it) => {
                if it.is_empty() {
                    true
                } else {
                    it.iter().any(|next_value| match next_value {
                        GrowthBookAttributeValue::Object(feature_value) => feature_value.iter().all(|next_attribute| recursive(None, next_attribute, user_attributes, false)),
                        _ => false,
                    })
                }
            },
            GrowthBookAttributeValue::Empty => true,
            _ => false,
        }
    }
}

fn and_nor(
    feature_attribute: &&GrowthBookAttribute,
    user_attributes: &[GrowthBookAttribute],
    recursive: fn(Option<&GrowthBookAttribute>, &GrowthBookAttribute, &[GrowthBookAttribute], bool) -> bool,
    negate: bool,
) -> bool {
    match &feature_attribute.value {
        GrowthBookAttributeValue::Array(it) => it.iter().all(|next_value| match next_value {
            GrowthBookAttributeValue::Object(feature_value) => {
                let result = feature_value.iter().all(|next_attribute| recursive(None, next_attribute, user_attributes, false));
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
