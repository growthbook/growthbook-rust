use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

pub struct OrderComparison;

impl OrderComparison {
    pub fn gt(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
        array_size: bool,
    ) -> bool {
        evaluate(
            parent_attribute,
            feature_attribute,
            user_attributes,
            array_size,
            |feature_number, user_number| user_number.gt(feature_number),
            |feature_string, user_string| user_string.gt(feature_string),
        )
    }

    pub fn gte(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
        array_size: bool,
    ) -> bool {
        evaluate(
            parent_attribute,
            feature_attribute,
            user_attributes,
            array_size,
            |feature_number, user_number| user_number.ge(feature_number),
            |feature_string, user_string| user_string.ge(feature_string),
        )
    }

    pub fn lt(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
        array_size: bool,
    ) -> bool {
        evaluate(
            parent_attribute,
            feature_attribute,
            user_attributes,
            array_size,
            |feature_number, user_number| user_number.lt(feature_number),
            |feature_string, user_string| user_string.lt(feature_string),
        )
    }

    pub fn lte(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
        array_size: bool,
    ) -> bool {
        evaluate(
            parent_attribute,
            feature_attribute,
            user_attributes,
            array_size,
            |feature_number, user_number| user_number.le(feature_number),
            |feature_string, user_string| user_string.le(feature_string),
        )
    }
}

fn evaluate(
    parent_attribute: Option<&GrowthBookAttribute>,
    feature_attribute: &GrowthBookAttribute,
    user_attributes: &[GrowthBookAttribute],
    array_size: bool,
    string_condition: fn(&str, &str) -> bool,
    number_condition: fn(&f64, &f64) -> bool,
) -> bool {
    if feature_attribute.value.is_number() {
        number_evaluate(parent_attribute, feature_attribute, user_attributes, array_size, number_condition)
    } else {
        string_evaluate(parent_attribute, feature_attribute, user_attributes, string_condition)
    }
}

fn string_evaluate(
    parent_attribute: Option<&GrowthBookAttribute>,
    feature_attribute: &GrowthBookAttribute,
    user_attributes: &[GrowthBookAttribute],
    condition: fn(&str, &str) -> bool,
) -> bool {
    if let Some(user_value) = user_attributes.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
        let feature_value = feature_attribute.value.to_string();
        match user_value {
            GrowthBookAttributeValue::Array(it) => it.iter().any(|item| condition(&feature_value, &item.to_string())),
            it => condition(&feature_value, &it.to_string()),
        }
    } else {
        true
    }
}

fn number_evaluate(
    parent_attribute: Option<&GrowthBookAttribute>,
    feature_attribute: &GrowthBookAttribute,
    user_attributes: &[GrowthBookAttribute],
    array_size: bool,
    condition: fn(&f64, &f64) -> bool,
) -> bool {
    if let Some(user_value) = user_attributes.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
        if let (Some(feature_number), Some(user_numbers)) = (get_feature_number(feature_attribute), get_user_numbers(&user_value, array_size)) {
            user_numbers.iter().any(|number| condition(&feature_number, number))
        } else {
            false
        }
    } else {
        true
    }
}

fn get_feature_number(feature_attribute: &GrowthBookAttribute) -> Option<f64> {
    match &feature_attribute.value {
        GrowthBookAttributeValue::Int(it) => Some(*it as f64),
        GrowthBookAttributeValue::Float(it) => Some(*it),
        GrowthBookAttributeValue::String(string_number) => string_number.replace('.', "").parse::<f64>().ok(),
        _ => None,
    }
}

fn get_user_numbers(
    user_value: &GrowthBookAttributeValue,
    array_size: bool,
) -> Option<Vec<f64>> {
    match user_value {
        GrowthBookAttributeValue::Int(it) => Some(vec![*it as f64]),
        GrowthBookAttributeValue::Float(it) => Some(vec![*it]),
        GrowthBookAttributeValue::Array(it) => {
            if array_size {
                Some(vec![it.len() as f64])
            } else {
                Some(it.iter().filter(|item| item.is_number()).map(|item| item.as_f64().expect("Failed to convert to f64")).collect())
            }
        },
        GrowthBookAttributeValue::String(string_number) => string_number.replace('.', "").parse::<f64>().ok().map(|it| vec![it]),
        _ => None,
    }
}
