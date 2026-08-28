use crate::condition::eval_context::{ConditionEvalContext, SavedGroups};
use crate::condition::use_case::ConditionsMatchesAttributes;
use crate::coverage::model::Coverage;
use crate::dto::GrowthBookFeatureRuleForce;
use crate::extensions::non_empty;
use crate::feature::resolve_hash_attribute;
use crate::model_public::{FeatureResult, GrowthBookAttribute};

impl GrowthBookFeatureRuleForce {
    pub fn get_match_value(
        &self,
        feature_name: &str,
        user_attributes: &Vec<GrowthBookAttribute>,
        sticky_bucketing_available: bool,
        saved_groups: &SavedGroups,
    ) -> Option<FeatureResult> {
        // Note: `filters` are evaluated once in the rule loop (get_value) for
        // every rule kind, so the force path no longer checks them here.
        if let Some(feature_attributes) = self.conditions() {
            if feature_attributes.matches(&ConditionEvalContext::new(user_attributes, saved_groups)) {
                self.check_range_or_force(feature_name, user_attributes, sticky_bucketing_available)
            } else {
                None
            }
        } else {
            self.check_range_or_force(feature_name, user_attributes, sticky_bucketing_available)
        }
    }

    fn check_range_or_force(
        &self,
        feature_name: &str,
        user_attributes: &Vec<GrowthBookAttribute>,
        sticky_bucketing_available: bool,
    ) -> Option<FeatureResult> {
        if let Some(range) = self.range() {
            let seed = non_empty(&self.seed).cloned().unwrap_or_else(|| feature_name.to_string());

            let fallback_allowed = sticky_bucketing_available && !self.disable_sticky_bucketing.unwrap_or(false);
            let (_, user_value) = resolve_hash_attribute(&self.hash_attribute, &self.fallback_attribute, fallback_allowed, user_attributes)?;
            Coverage::check(&user_value, None, Some(range), &seed, self.hash_version, self.force.clone())
        } else {
            Some(FeatureResult::force(self.force.clone()))
        }
    }
}
