use std::collections::HashMap;
use std::sync::Arc;

use crate::condition::eval_context::SavedGroups;
use crate::dto::{GrowthBookFeature, GrowthBookFeatureRuleKind, GrowthBookFeatureRuleParentData};
use crate::model_public::{FeatureResult, GrowthBookAttribute};
use crate::sticky_bucket::StickyBucketService;

/// Outcome of evaluating a rule's `parentConditions`.
enum ParentOutcome {
    /// All prerequisites passed; continue evaluating this rule.
    Continue,
    /// A non-gating prerequisite failed; skip this rule, try the next one.
    SkipRule,
    /// A gating prerequisite failed (or a cycle was hit); short-circuit the
    /// whole feature with this result. Boxed because `FeatureResult` is large
    /// relative to the empty variants.
    ShortCircuit(Box<FeatureResult>),
}

impl GrowthBookFeature {
    #[allow(clippy::too_many_arguments)]
    pub fn get_value(
        &self,
        feature_name: &str,
        feature_name_decorate: Vec<String>,
        user_attributes: &Vec<GrowthBookAttribute>,
        forced_variations: &Option<HashMap<String, i64>>,
        all_features: &HashMap<String, GrowthBookFeature>,
        sticky_bucket_service: &Option<Arc<dyn StickyBucketService>>,
        saved_groups: &SavedGroups,
    ) -> FeatureResult {
        if let Some(rules) = &self.rules {
            for rule in rules {
                // parentConditions are evaluated first, for every rule kind.
                if let Some(parents) = &rule.parent_conditions {
                    match evaluate_parent_conditions(
                        parents,
                        feature_name,
                        &feature_name_decorate,
                        user_attributes,
                        forced_variations,
                        all_features,
                        sticky_bucket_service,
                        saved_groups,
                    ) {
                        ParentOutcome::ShortCircuit(result) => return *result,
                        ParentOutcome::SkipRule => continue,
                        ParentOutcome::Continue => {},
                    }
                }

                match &rule.kind {
                    GrowthBookFeatureRuleKind::Force(it) => {
                        if let Some(feature) = it.get_match_value(feature_name, user_attributes, saved_groups) {
                            return feature;
                        }
                    },
                    GrowthBookFeatureRuleKind::Rollout(it) => {
                        if let Some(feature) = it.get_match_value(feature_name, user_attributes, saved_groups) {
                            return feature;
                        }
                    },
                    GrowthBookFeatureRuleKind::Experiment(it) => {
                        if let Some(feature) = it.get_match_value(feature_name, user_attributes, forced_variations, sticky_bucket_service, saved_groups) {
                            return feature;
                        }
                    },
                    GrowthBookFeatureRuleKind::Empty => {},
                }
            }
        }

        FeatureResult::from_default_value(self.default_value.clone())
    }
}

#[allow(clippy::too_many_arguments)]
fn evaluate_parent_conditions(
    parents: &[GrowthBookFeatureRuleParentData],
    feature_name: &str,
    feature_name_decorate: &[String],
    user_attributes: &Vec<GrowthBookAttribute>,
    forced_variations: &Option<HashMap<String, i64>>,
    all_features: &HashMap<String, GrowthBookFeature>,
    sticky_bucket_service: &Option<Arc<dyn StickyBucketService>>,
    saved_groups: &SavedGroups,
) -> ParentOutcome {
    for parent in parents {
        let parent_feature_name = &parent.id;
        if feature_name_decorate.contains(parent_feature_name) {
            return ParentOutcome::ShortCircuit(Box::new(FeatureResult::cyclic_prerequisite()));
        }

        let mut updated_decorate = feature_name_decorate.to_vec();
        updated_decorate.push(String::from(feature_name));

        let parent_response = if let Some(parent_feature) = all_features.get(parent_feature_name) {
            parent_feature.get_value(
                parent_feature_name,
                updated_decorate,
                user_attributes,
                forced_variations,
                all_features,
                sticky_bucket_service,
                saved_groups,
            )
        } else {
            FeatureResult::unknown_feature()
        };

        if parent_response.source == "cyclicPrerequisite" {
            return ParentOutcome::ShortCircuit(Box::new(FeatureResult::cyclic_prerequisite()));
        }

        if !parent.is_met(parent_response, saved_groups) {
            if parent.gate {
                // Gating prerequisite failed → block the whole feature.
                return ParentOutcome::ShortCircuit(Box::new(FeatureResult::prerequisite()));
            }
            // Non-gating prerequisite failed → skip this rule, try the next.
            return ParentOutcome::SkipRule;
        }
    }

    ParentOutcome::Continue
}
