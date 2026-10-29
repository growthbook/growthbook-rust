use serde_json::Value;

use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

/// JS-style `||` for optional strings: an empty string counts as absent, so
/// `rule.seed || featureKey` / `hashAttribute || "id"` defaults apply to `""`.
pub fn non_empty(option: &Option<String>) -> Option<&String> {
    option.as_ref().filter(|value| !value.is_empty())
}

/// Whether an attribute value can identify a user for hashing, like JS
/// `isHashable` (core.ts): only a non-empty string or a finite, non-zero
/// number. Everything else (`null`, booleans, `0`, `""`, arrays, objects)
/// counts as missing.
pub fn hashable(value: &GrowthBookAttributeValue) -> bool {
    match value {
        GrowthBookAttributeValue::String(it) => !it.is_empty(),
        GrowthBookAttributeValue::Int(it) => *it != 0,
        GrowthBookAttributeValue::Float(it) => *it != 0.0 && it.is_finite(),
        GrowthBookAttributeValue::Empty | GrowthBookAttributeValue::Bool(_) | GrowthBookAttributeValue::Array(_) | GrowthBookAttributeValue::Object(_) => false,
    }
}

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
        look_for_attribute(attribute_key, self)
    }
}

impl FindGrowthBookAttribute for &[GrowthBookAttribute] {
    fn find_value(
        &self,
        attribute_key: &str,
    ) -> Option<GrowthBookAttributeValue> {
        look_for_attribute(attribute_key, self)
    }
}

fn look_for_attribute(
    attribute_key: &str,
    user_attributes: &[GrowthBookAttribute],
) -> Option<GrowthBookAttributeValue> {
    let mut parts = attribute_key.split('.').peekable();
    let first = parts.next()?;
    let mut value = &user_attributes.iter().find(|attribute| attribute.key == first)?.value;
    while let Some(part) = parts.next() {
        value = match value {
            GrowthBookAttributeValue::Object(fields) => &fields.iter().find(|attribute| attribute.key == part)?.value,
            GrowthBookAttributeValue::Array(items) => {
                if part == "length" && parts.peek().is_none() {
                    return Some(GrowthBookAttributeValue::Int(items.len() as i64));
                }
                let index = part.parse::<usize>().ok()?;
                // JS array property names use canonical indices, not "01" or "+1".
                if index.to_string() != part {
                    return None;
                }
                items.get(index)?
            },
            // A missing path segment must not return its scalar parent: doing
            // so could match a saved list against the wrong attribute value.
            _ => return None,
        };
    }
    Some(value.clone())
}
