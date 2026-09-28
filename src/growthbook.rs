use std::collections::HashMap;
use std::sync::Arc;

use crate::condition::eval_context::SavedGroups;
use crate::dto::GrowthBookFeature;
use crate::model_public::{FeatureResult, GrowthBookAttribute};
use crate::sticky_bucket::StickyBucketService;

#[derive(Debug, Clone)]
pub struct GrowthBook {
    pub forced_variations: Option<HashMap<String, i64>>,
    pub features: HashMap<String, GrowthBookFeature>,
    pub attributes: Option<HashMap<String, GrowthBookAttribute>>,
    pub sticky_bucket_service: Option<Arc<dyn StickyBucketService>>,
    /// Parsed saved groups. Wrap legacy value vectors in [`crate::model_public::SavedGroup::LegacyList`]
    /// when constructing this map directly, or use the client's JSON builder method.
    pub saved_groups: SavedGroups,
}

impl GrowthBook {
    pub fn check(
        &self,
        flag_name: &str,
        option_user_attributes: &Option<Vec<GrowthBookAttribute>>,
    ) -> FeatureResult {
        if let Some(feature) = self.features.get(flag_name) {
            // Merge instance attributes with call-time attributes
            let mut merged_attributes = Vec::new();

            // Add instance attributes first
            if let Some(instance_attrs) = &self.attributes {
                for attr in instance_attrs.values() {
                    merged_attributes.push(attr.clone());
                }
            }

            // Add/Override with call-time attributes
            if let Some(call_attrs) = option_user_attributes {
                merged_attributes.extend(call_attrs.clone());
            }

            feature.get_value(
                flag_name,
                vec![],
                &merged_attributes,
                &self.forced_variations,
                &self.features,
                &self.sticky_bucket_service,
                &self.saved_groups,
            )
        } else {
            FeatureResult::unknown_feature()
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use serde::Deserialize;
    use serde_json::Value;

    use crate::condition::eval_context::saved_groups_from_value;
    use crate::dto::GrowthBookFeature;
    use crate::extensions::JsonHelper;
    use crate::growthbook::GrowthBook;
    use crate::model_public::FeatureResult;
    use crate::model_public::GrowthBookAttribute;

    #[tokio::test]
    async fn evaluate_get_bucket_range() -> Result<(), Box<dyn std::error::Error>> {
        let cases = Cases::new();
        evaluate_feature_cases(cases.feature);
        Ok(())
    }

    #[tokio::test]
    #[ignore = "Contextual bandits are not supported by the Rust SDK"]
    async fn evaluate_contextual_bandits() {
        let all = crate::corpus::load();
        let active = crate::corpus::active();
        let mut cases = all["contextualBandit"].as_array().expect("bandit cases").clone();
        cases.extend(
            all["feature"]
                .as_array()
                .expect("feature cases")
                .iter()
                .filter(|case| !active["feature"].as_array().expect("active features").contains(case))
                .cloned(),
        );
        evaluate_feature_cases(cases);
    }

    fn evaluate_feature_cases(cases: Vec<Value>) {
        for value in cases {
            let feature = EvalFeature::new(value);

            let saved_groups = saved_groups_from_value(feature.feature.get("savedGroups"));
            let gb_test_res = serde_json::from_value::<GrowthBookForTest>(feature.feature.clone());
            let gb_test = gb_test_res.unwrap_or_else(|_| panic!("Failed to convert to GrowthBookForTest case='{}'", feature.name));
            let gb = GrowthBook {
                forced_variations: feature.forced_variations.clone(),
                features: gb_test.features.unwrap_or_default(),
                attributes: None,
                sticky_bucket_service: None,
                saved_groups,
            };
            let user_attributes = feature
                .attributes
                .clone()
                .map(|attr| GrowthBookAttribute::from(attr).expect("Failed to convert to GrowthBookAttribute"));
            let result = gb.check(feature.feature_name.as_str(), &user_attributes);
            validate_result(feature, result);
        }
    }

    fn validate_result(
        eval_feature: EvalFeature,
        feature_result: FeatureResult,
    ) {
        let case_name = eval_feature.name;
        let expected_result = eval_feature.result;

        assert_eq!(expected_result.get_value("value", Value::Null), feature_result.value, "Invalid value for '{case_name}'");
        assert_eq!(
            expected_result.get("on").expect("Failed to get on").as_bool().expect("Failed to convert to bool"),
            feature_result.on,
            "Invalid on for '{case_name}'"
        );
        assert_eq!(
            expected_result.get("off").expect("Failed to get off").as_bool().expect("Failed to convert to bool"),
            feature_result.off,
            "Invalid off for '{case_name}'"
        );
        assert_eq!(expected_result.get_string("source", ""), feature_result.source, "Invalid source for '{case_name}'");
    }

    #[derive(Deserialize, Clone)]
    #[serde(rename_all = "camelCase")]
    struct Cases {
        feature: Vec<Value>,
    }

    #[derive(Deserialize, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct GrowthBookForTest {
        pub features: Option<HashMap<String, GrowthBookFeature>>,
    }

    #[derive(Clone)]
    pub struct EvalFeature {
        name: String,
        attributes: Option<Value>,
        forced_variations: Option<HashMap<String, i64>>,
        feature: Value,
        feature_name: String,
        result: Value,
    }

    impl EvalFeature {
        fn new(value: Value) -> Self {
            let array = value.as_array().expect("Failed to convert to array");
            let attr = array[1].as_object().expect("Failed to convert to object").get("attributes").unwrap_or(&Value::Null).clone();
            let attributes = if attr.is_null() { None } else { Some(attr) };
            let forced = array[1].as_object().expect("Failed to convert to object").get("forcedVariations").unwrap_or(&Value::Null).clone();
            let forced_variations = forced.as_object().map(|forced_entries| {
                let mut map = HashMap::new();
                for (key, value) in forced_entries {
                    map.insert(key.clone(), value.as_i64().expect("Failed to convert to i64"));
                }
                map
            });
            Self {
                name: array[0].as_str().expect("Failed to convert to str").to_string(),
                attributes,
                forced_variations,
                feature: array[1].clone(),
                feature_name: array[2].as_str().expect("Failed to convert to str").to_string(),
                result: array[3].clone(),
            }
        }
    }

    impl Cases {
        pub fn new() -> Self {
            serde_json::from_value(crate::corpus::active()).expect("Failed to create cases")
        }
    }
}
