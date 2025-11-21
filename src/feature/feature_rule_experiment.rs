use std::collections::HashMap;

use serde_json::Value;

use crate::dto::GrowthBookFeatureRuleExperiment;
use crate::extensions::{FindGrowthBookAttribute, JsonHelper};
use crate::hash::{HashCode, HashCodeVersion};
use crate::model_public::{ExperimentResult, FeatureResult, GrowthBookAttribute};
use crate::namespace::use_case::Namespace;
use crate::range::model::Range;

impl GrowthBookFeatureRuleExperiment {
    pub fn get_match_value(
        &self,
        feature_name: &str,
        user_attributes: &Vec<GrowthBookAttribute>,
        forced_variations: &Option<HashMap<String, i64>>,
    ) -> Option<FeatureResult> {
        if let Some(feature_attribute) = &self.hash_attribute {
            self.check_experiment(&feature_name, user_attributes, forced_variations, feature_attribute)
        } else {
            let fallback_attribute = self.get_fallback_attribute();
            self.check_experiment(&feature_name, user_attributes, forced_variations, &fallback_attribute)
        }
    }

    fn check_experiment(
        &self,
        feature_name: &&str,
        user_attributes: &Vec<GrowthBookAttribute>,
        forced_variations: &Option<HashMap<String, i64>>,
        feature_attribute: &str,
    ) -> Option<FeatureResult> {
        if let Some(user_value) = user_attributes.find_value(feature_attribute) {
            if let Some((namespace, range)) = &self.namespace_range() {
                if !Namespace::is_in(&user_value, namespace, range) {
                    return None;
                }
            }

            if let Some(forced_variation) = self.forced_variation(feature_name, user_attributes, forced_variations) {
                return Some(forced_variation);
            }

            let user_weight = HashCode::hash_code(&user_value.to_string(), &self.seed(feature_name), HashCodeVersion::from(self.hash_version)).unwrap_or(-1.0);
            let ranges = self.ranges();
            let index = choose_variation(user_weight, ranges);
            if index >= 0 {
                let usize_index = index as usize;
                let value = self.variations[usize_index].clone();
                let (meta_value, pass_through) = self.get_meta_value(usize_index);
                if !pass_through {
                    return Some(FeatureResult::experiment(
                        value.clone(),
                        self.model_experiment(),
                        create_experiment_result(
                            feature_name,
                            value.clone(),
                            index,
                            true,
                            Some(feature_attribute.to_string()),
                            Some(user_value.to_value()),
                            Some(user_weight),
                            meta_value,
                        ),
                    ));
                }
            }
        }

        None
    }

    fn forced_variation(
        &self,
        feature_name: &str,
        user_attributes: &Vec<GrowthBookAttribute>,
        forced_variations: &Option<HashMap<String, i64>>,
    ) -> Option<FeatureResult> {
        if let Some(forced_variations) = forced_variations {
            if let Some(found_forced_variation) = forced_variations.get(feature_name) {
                let hash_attribute = self.hash_attribute.clone().unwrap_or(self.get_fallback_attribute());
                if let Some(user_value) = user_attributes.find_value(&hash_attribute) {
                    let forced_variation_index = *found_forced_variation as usize;
                    let value = self.variations[forced_variation_index].clone();
                    let (meta_value, pass_through) = self.get_meta_value(forced_variation_index);
                    if !pass_through {
                        return Some(FeatureResult::experiment(
                            value.clone(),
                            self.model_experiment(),
                            create_experiment_result(
                                feature_name,
                                value.clone(),
                                *found_forced_variation,
                                true,
                                self.hash_attribute.clone(),
                                Some(user_value.to_value()),
                                None,
                                meta_value,
                            ),
                        ));
                    }
                }
            }
        }
        None
    }

    fn get_meta_value(
        &self,
        usize_index: usize,
    ) -> (String, bool) {
        match &self.meta {
            None => (format!("{usize_index}"), false),
            Some(it) => {
                if let Some(meta_value) = it.force_array(vec![]).get(usize_index) {
                    let pass_through = if let Some(pass_through_value) = meta_value.get("passthrough") {
                        pass_through_value.force_bool(false)
                    } else {
                        false
                    };

                    if let Some(key) = meta_value.get("key") {
                        (key.force_string(""), pass_through)
                    } else {
                        (format!("{usize_index}"), pass_through)
                    }
                } else {
                    (format!("{usize_index}"), false)
                }
            },
        }
    }

    fn get_fallback_attribute(&self) -> String {
        self.fallback_attribute.clone().unwrap_or(String::from("id"))
    }
}

#[allow(clippy::too_many_arguments)]
fn create_experiment_result(
    feature_name: &str,
    value: Value,
    variation_id: i64,
    hash_used: bool,
    hash_attribute: Option<String>,
    hash_value: Option<Value>,
    bucket: Option<f32>,
    key: String,
) -> ExperimentResult {
    ExperimentResult {
        feature_id: String::from(feature_name),
        value,
        variation_id,
        in_experiment: true,
        hash_used,
        hash_attribute,
        hash_value,
        bucket,
        key,
        sticky_bucket_used: false,
    }
}

fn choose_variation(
    user_weight: f32,
    ranges: Vec<Range>,
) -> i64 {
    for (index, range) in ranges.iter().enumerate() {
        if range.in_range(&user_weight) {
            return index as i64;
        }
    }
    -1
}

#[cfg(test)]
mod test {
    use std::fs;

    use serde::Deserialize;
    use serde_json::Value;

    use crate::feature::feature_rule_experiment::choose_variation;
    use crate::range::model::Range;

    #[tokio::test]
    async fn evaluate_choose_variation() -> Result<(), Box<dyn std::error::Error>> {
        let cases = Cases::new();

        for value in cases.choose_variation {
            let eval_choose_variation = EvalChooseVariation::new(value);
            let index = choose_variation(eval_choose_variation.weight, eval_choose_variation.ranges);
            if eval_choose_variation.index != index {
                panic!(
                    "EvalChooseVariation failed; name='{}' expected_index={} index={index}",
                    eval_choose_variation.name, eval_choose_variation.index
                )
            }
        }

        Ok(())
    }

    #[derive(Deserialize, Clone)]
    #[serde(rename_all = "camelCase")]
    struct Cases {
        choose_variation: Vec<Value>,
    }

    pub struct EvalChooseVariation {
        name: String,
        weight: f32,
        ranges: Vec<Range>,
        index: i64,
    }

    impl EvalChooseVariation {
        fn new(value: Value) -> Self {
            let array = value.as_array().expect("Failed to convert to array");
            Self {
                name: array[0].as_str().expect("Failed to convert to str").to_string(),
                weight: array[1].as_f64().expect("Failed to convert to f64") as f32,
                ranges: array[2]
                    .as_array()
                    .expect("Failed to convert to array")
                    .iter()
                    .map(|it| {
                        let array = it.as_array().expect("Failed to convert to array [2]");
                        Range {
                            start: array[0].as_f64().expect("Failed to convert to f64") as f32,
                            end: array[1].as_f64().expect("Failed to convert to f64") as f32,
                        }
                    })
                    .collect(),
                index: array[3].as_i64().expect("Failed to convert to i64"),
            }
        }
    }

    impl Cases {
        pub fn new() -> Self {
            let contents = fs::read_to_string("./tests/all_cases.json").expect("Should have been able to read the file");

            serde_json::from_str(&contents).expect("Failed to create cases")
        }
    }
}
