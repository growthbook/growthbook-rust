use regex::Regex;

use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

pub struct VersionComparison;

impl VersionComparison {
    pub fn vgt(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, user_attributes, |feature_version, user_version| user_version.gt(feature_version))
    }

    pub fn vgte(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, user_attributes, |feature_version, user_version| user_version.ge(feature_version))
    }

    pub fn vlt(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, user_attributes, |feature_version, user_version| user_version.lt(feature_version))
    }

    pub fn vlte(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, user_attributes, |feature_version, user_version| user_version.le(feature_version))
    }

    pub fn veq(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, user_attributes, |feature_version, user_version| user_version.eq(feature_version))
    }

    pub fn vne(
        parent_attribute: Option<&GrowthBookAttribute>,
        feature_attribute: &GrowthBookAttribute,
        user_attributes: &[GrowthBookAttribute],
    ) -> bool {
        evaluate(parent_attribute, feature_attribute, user_attributes, |feature_version, user_version| user_version.ne(feature_version))
    }
}

fn evaluate(
    parent_attribute: Option<&GrowthBookAttribute>,
    feature_attribute: &GrowthBookAttribute,
    user_attributes: &[GrowthBookAttribute],
    condition: fn(&str, &str) -> bool,
) -> bool {
    if let Some(GrowthBookAttributeValue::String(user_version)) = user_attributes.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
        let feature_version = feature_attribute.value.to_string();
        condition(&normalize(&feature_version), &normalize(&user_version))
    } else {
        true
    }
}

fn normalize(version: &str) -> String {
    if let Ok(regex1) = Regex::new("(^v|\\+.*$)") {
        if let Ok(regex2) = Regex::new("[-.]") {
            if let Ok(regex3) = Regex::new("^\\d+") {
                let string = regex1.replace_all(version, "").to_string();
                let mut split = regex2.split(&string).filter(|item| !item.is_empty()).collect::<Vec<&str>>();
                if split.len() == 3 {
                    split.push("~");
                }
                split
                    .iter()
                    .map(|part| if regex3.is_match(part) { format!("{:0>5}", part) } else { part.to_string() })
                    .filter(|part| !part.is_empty())
                    .reduce(|a, b| format!("{a}-{b}"))
                    .unwrap_or(version.to_string())
            } else {
                version.to_string()
            }
        } else {
            version.to_string()
        }
    } else {
        version.to_string()
    }
}
