use crate::condition::use_case::ConditionsMatchesAttributes;
use crate::dto::GrowthBookFeatureRuleParentData;
use crate::model_public::{FeatureResult, GrowthBookAttribute, GrowthBookAttributeValue};

impl GrowthBookFeatureRuleParentData {
    pub fn is_met(
        &self,
        feature: FeatureResult,
    ) -> bool {
        if let Some(feature_attributes) = self.conditions() {
            feature_attributes.matches(&[GrowthBookAttribute::new(String::from("value"), GrowthBookAttributeValue::from(feature.value))])
        } else {
            true
        }
    }
}
