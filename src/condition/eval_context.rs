use std::collections::HashMap;

use serde_json::Value;

use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

/// Saved groups available to condition evaluation: a group id mapped to its
/// list of member values. Used by the `$inGroup` / `$notInGroup` operators.
pub type SavedGroups = HashMap<String, Vec<GrowthBookAttributeValue>>;

/// Everything condition evaluation needs beyond the condition itself: the
/// attributes being evaluated plus the saved groups. Bundled into one context
/// (rather than threaded as separate params) so new evaluation inputs can be
/// added without re-touching every operator signature.
pub struct ConditionEvalContext<'a> {
    attributes: &'a [GrowthBookAttribute],
    saved_groups: &'a SavedGroups,
}

impl<'a> ConditionEvalContext<'a> {
    pub fn new(
        attributes: &'a [GrowthBookAttribute],
        saved_groups: &'a SavedGroups,
    ) -> Self {
        Self { attributes, saved_groups }
    }

    /// Members of a saved group by id, if the group is known.
    pub fn saved_group(
        &self,
        group_id: &str,
    ) -> Option<&[GrowthBookAttributeValue]> {
        self.saved_groups.get(group_id).map(|members| members.as_slice())
    }
}

// Lets every existing `ctx.find_value(key)` call site keep working after the
// parameter type changed from `&[GrowthBookAttribute]` to `&ConditionEvalContext`.
impl FindGrowthBookAttribute for ConditionEvalContext<'_> {
    fn find_value(
        &self,
        attribute_key: &str,
    ) -> Option<GrowthBookAttributeValue> {
        self.attributes.find_value(attribute_key)
    }
}

/// Build a `SavedGroups` map from the raw `{ id: [values] }` payload shape.
pub fn saved_groups_from_value(value: Option<&Value>) -> SavedGroups {
    let mut groups = SavedGroups::new();
    if let Some(Value::Object(map)) = value {
        for (id, members) in map {
            if let Value::Array(items) = members {
                groups.insert(id.clone(), items.iter().map(|item| GrowthBookAttributeValue::from(item.clone())).collect());
            }
        }
    }
    groups
}
