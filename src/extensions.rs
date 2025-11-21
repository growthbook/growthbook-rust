use serde_json::Value;

use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

pub trait FindGrowthBookAttribute {
    fn find_value(
        &self,
        attribute_key: &str,
    ) -> Option<GrowthBookAttributeValue>;
}

pub trait JsonHelper {
    fn get_value(
        &self,
        name: &str,
        default: Value,
    ) -> Value;
    fn get_string(
        &self,
        name: &str,
        default: &str,
    ) -> String;
    fn get_array(
        &self,
        name: &str,
        default: Vec<Value>,
    ) -> Vec<Value>;

    fn force_string(
        &self,
        default: &str,
    ) -> String;
    fn force_f32(
        &self,
        default: f32,
    ) -> f32;
    fn force_f64(
        &self,
        default: f64,
    ) -> f64;
    fn force_bool(
        &self,
        default: bool,
    ) -> bool;
    fn force_array(
        &self,
        default: Vec<Value>,
    ) -> Vec<Value>;
}

impl JsonHelper for Value {
    fn get_value(
        &self,
        name: &str,
        default: Value,
    ) -> Value {
        self.get(name).unwrap_or(&default).clone()
    }

    fn get_string(
        &self,
        name: &str,
        default: &str,
    ) -> String {
        self.get_value(name, Value::String(String::from(default))).force_string(default)
    }

    fn get_array(
        &self,
        name: &str,
        default: Vec<Value>,
    ) -> Vec<Value> {
        self.get(name).unwrap_or(&Value::Null).force_array(default)
    }

    fn force_string(
        &self,
        default: &str,
    ) -> String {
        if self.is_string() {
            self.as_str().unwrap_or(default).to_string()
        } else {
            self.to_string()
        }
    }

    fn force_f32(
        &self,
        default: f32,
    ) -> f32 {
        self.force_f64(default as f64) as f32
    }

    fn force_f64(
        &self,
        default: f64,
    ) -> f64 {
        self.as_f64().unwrap_or(default)
    }

    fn force_bool(
        &self,
        default: bool,
    ) -> bool {
        self.as_bool().unwrap_or(default)
    }

    fn force_array(
        &self,
        default: Vec<Value>,
    ) -> Vec<Value> {
        self.as_array().unwrap_or(&default).clone()
    }
}

impl FindGrowthBookAttribute for Vec<GrowthBookAttribute> {
    fn find_value(
        &self,
        attribute_key: &str,
    ) -> Option<GrowthBookAttributeValue> {
        look_for_attribute(0, attribute_key, self).map(|it| it.value)
    }
}

impl FindGrowthBookAttribute for &[GrowthBookAttribute] {
    fn find_value(
        &self,
        attribute_key: &str,
    ) -> Option<GrowthBookAttributeValue> {
        look_for_attribute(0, attribute_key, self).map(|it| it.value)
    }
}

fn look_for_attribute(
    split_index: usize,
    attribute_key: &str,
    user_attributes: &[GrowthBookAttribute],
) -> Option<GrowthBookAttribute> {
    let split = attribute_key.split('.').collect::<Vec<&str>>();
    let key_part = split[split_index];
    let option_attribute = user_attributes.iter().find(|item| item.key == key_part);
    if let Some(found_attribute) = option_attribute {
        if split.len().gt(&(split_index + 1)) {
            match found_attribute.value.clone() {
                GrowthBookAttributeValue::Object(it) => look_for_attribute(split_index + 1, attribute_key, &it),
                GrowthBookAttributeValue::Empty => None,
                _ => Some(found_attribute.clone()),
            }
        } else {
            Some(found_attribute.clone())
        }
    } else {
        None
    }
}
