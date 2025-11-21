use crate::condition::use_case::ConditionsMatchesAttributes;
use crate::coverage::model::Coverage;
use crate::dto::GrowthBookFeatureRuleForce;
use crate::extensions::FindGrowthBookAttribute;
use crate::filter::use_case::Filter;
use crate::model_public::{FeatureResult, GrowthBookAttribute};

impl GrowthBookFeatureRuleForce {
    pub fn get_match_value(
        &self,
        feature_name: &str,
        user_attributes: &Vec<GrowthBookAttribute>,
    ) -> Option<FeatureResult> {
        if let Some(filters) = &self.filters {
            let hash_attribute = self.get_fallback_attribute();
            if Filter::is_filtered_out(filters, &hash_attribute, user_attributes) {
                return None;
            }
        }

        if let Some(feature_attributes) = self.conditions() {
            if feature_attributes.matches(user_attributes) {
                self.check_range_or_force(feature_name, user_attributes)
            } else {
                None
            }
        } else {
            self.check_range_or_force(feature_name, user_attributes)
        }
    }

    fn check_range_or_force(
        &self,
        feature_name: &str,
        user_attributes: &Vec<GrowthBookAttribute>,
    ) -> Option<FeatureResult> {
        if let Some(range) = self.range() {
            let fallback_attribute = self.get_fallback_attribute();
            if let Some(user_value) = user_attributes.find_value(&fallback_attribute) {
                let seed = self.seed.clone().unwrap_or(feature_name.to_string());
                Coverage::check(&user_value, None, Some(range), &seed, self.hash_version, self.force.clone())
            } else {
                None
            }
        } else {
            Some(FeatureResult::force(self.force.clone()))
        }
    }
}
