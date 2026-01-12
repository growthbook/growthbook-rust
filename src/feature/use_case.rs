use std::collections::HashMap;
use std::sync::Arc;

use crate::dto::{GrowthBookFeature, GrowthBookFeatureRule};
use crate::model_public::{FeatureResult, GrowthBookAttribute};
use crate::sticky_bucket::StickyBucketService;

impl GrowthBookFeature {
    pub fn get_value(
        &self,
        feature_name: &str,
        feature_name_decorate: Vec<String>,
        user_attributes: &Vec<GrowthBookAttribute>,
        forced_variations: &Option<HashMap<String, i64>>,
        all_features: &HashMap<String, GrowthBookFeature>,
        sticky_bucket_service: &Option<Arc<dyn StickyBucketService>>,
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
                        if let Some(feature) = it.get_match_value(feature_name, user_attributes, forced_variations, sticky_bucket_service) {
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
                                parent_feature.get_value(parent_feature_name, updated_decorate, user_attributes, forced_variations, all_features, sticky_bucket_service)
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
