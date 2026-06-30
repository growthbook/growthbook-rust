use crate::condition::eval_context::{ConditionEvalContext, SavedGroups};
use crate::condition::use_case::ConditionsMatchesAttributes;
use crate::dto::GrowthBookFeatureRuleParentData;
use crate::model_public::{FeatureResult, GrowthBookAttribute, GrowthBookAttributeValue};

impl GrowthBookFeatureRuleParentData {
    pub fn is_met(
        &self,
        feature: FeatureResult,
        saved_groups: &SavedGroups,
    ) -> bool {
        if let Some(feature_attributes) = self.conditions() {
            let attributes = [GrowthBookAttribute::new(String::from("value"), GrowthBookAttributeValue::from(feature.value))];
            feature_attributes.matches(&ConditionEvalContext::new(&attributes, saved_groups))
        } else {
            true
        }
    }
}
