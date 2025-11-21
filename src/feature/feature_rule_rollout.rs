use crate::condition::use_case::ConditionsMatchesAttributes;
use crate::coverage::model::Coverage;
use crate::dto::GrowthBookFeatureRuleRollout;
use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{FeatureResult, GrowthBookAttribute};

impl GrowthBookFeatureRuleRollout {
    pub fn get_match_value(
        &self,
        feature_name: &str,
        user_attributes: &Vec<GrowthBookAttribute>,
    ) -> Option<FeatureResult> {
        if let Some(feature_attributes) = &self.conditions() {
            if feature_attributes.matches(user_attributes) {
                self.check_coverage(feature_name, user_attributes)
            } else {
                None
            }
        } else {
            self.check_coverage(feature_name, user_attributes)
        }
    }

    fn check_coverage(
        &self,
        feature_name: &str,
        user_attributes: &Vec<GrowthBookAttribute>,
    ) -> Option<FeatureResult> {
        if let Some(hash_attribute) = &self.hash_attribute {
            if let Some(user_value) = user_attributes.find_value(hash_attribute) {
                return Coverage::check(&user_value, Some(self.coverage), self.range(), feature_name, self.hash_version, self.force.clone());
            }
        }

        let fallback_attribute = self.get_fallback_attribute();
        if let Some(user_value) = user_attributes.find_value(&fallback_attribute) {
            return Coverage::check(&user_value, Some(self.coverage), self.range(), feature_name, self.hash_version, self.force.clone());
        }

        None
    }
}
