use std::collections::HashMap;

use crate::dto::{GrowthBookFeature, GrowthBookFeatureRule};
use crate::model_public::{FeatureResult, GrowthBookAttribute};

impl GrowthBookFeature {
    pub fn get_value(
        &self,
        feature_name: &str,
        feature_name_decorate: Vec<String>,
        user_attributes: &Vec<GrowthBookAttribute>,
        forced_variations: &Option<HashMap<String, i64>>,
        all_features: HashMap<String, GrowthBookFeature>,
    ) -> FeatureResult {
        if let Some(rules) = &self.rules {
            for rule in rules {
                match rule {
                    GrowthBookFeatureRule::Force(it) => {
                        if let Some(feature) = it.get_match_value(feature_name, user_attributes) {
                            return feature;
                        }
                    },
                    GrowthBookFeatureRule::Rollout(it) => {
                        if let Some(feature) = it.get_match_value(feature_name, user_attributes) {
                            return feature;
                        }
                    },
                    GrowthBookFeatureRule::Experiment(it) => {
                        if let Some(feature) = it.get_match_value(feature_name, user_attributes, forced_variations) {
                            return feature;
                        }
                    },
                    GrowthBookFeatureRule::Parent(it) => {
                        for parent in &it.parent_conditions {
                            let parent_feature_name = &parent.id;
                            if feature_name_decorate.contains(parent_feature_name) {
                                return FeatureResult::cyclic_prerequisite();
                            }

                            let mut updated_decorate = feature_name_decorate.clone();
                            updated_decorate.push(String::from(feature_name));

                            let parent_response = if let Some(parent_feature) = all_features.get(parent_feature_name) {
                                parent_feature.get_value(parent_feature_name, updated_decorate, user_attributes, forced_variations, all_features.clone())
                            } else {
                                FeatureResult::unknown_feature()
                            };

                            if parent_response.source == "cyclicPrerequisite" {
                                return FeatureResult::cyclic_prerequisite();
                            }

                            if !parent.is_met(parent_response) {
                                return FeatureResult::prerequisite();
                            }
                        }
                    },
                    GrowthBookFeatureRule::Empty(_) => {
                        continue;
                    },
                }
            }
        }

        FeatureResult::from_default_value(self.default_value.clone())
    }
}
