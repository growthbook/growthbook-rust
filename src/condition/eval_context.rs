use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue, SavedGroup};

/// Legacy lists and typed saved groups, indexed by group id.
pub type SavedGroups = HashMap<String, SavedGroup>;

/// Bound recursive group resolution, including long chains without cycles.
const MAX_SAVED_GROUP_DEPTH: usize = 128;

/// Everything condition evaluation needs beyond the condition itself: the
/// attributes being evaluated plus the saved groups. Bundled into one context
/// (rather than threaded as separate params) so new evaluation inputs can be
/// added without re-touching every operator signature.
pub struct ConditionEvalContext<'a> {
    attributes: &'a [GrowthBookAttribute],
    saved_groups: &'a SavedGroups,
    visited: HashSet<String>,
}

impl<'a> ConditionEvalContext<'a> {
    pub fn new(
        attributes: &'a [GrowthBookAttribute],
        saved_groups: &'a SavedGroups,
    ) -> Self {
        Self {
            attributes,
            saved_groups,
            visited: HashSet::new(),
        }
    }

    /// The complete entry, preserving malformed entries separately from absent ids.
    pub fn saved_group(
        &self,
        group_id: &str,
    ) -> Option<&SavedGroup> {
        self.saved_groups.get(group_id)
    }

    /// Legacy operators treat absent ids as empty lists, but fail closed for
    /// present entries that do not contain list values.
    pub fn saved_group_values(
        &self,
        group_id: &str,
    ) -> Option<&[GrowthBookAttributeValue]> {
        match self.saved_group(group_id) {
            None => Some(&[]),
            Some(SavedGroup::LegacyList(values)) | Some(SavedGroup::List { values: Some(values), .. }) => Some(values),
            _ => None,
        }
    }

    /// Enter one branch of group resolution without marking sibling branches.
    pub fn enter_group(
        &self,
        group_id: &str,
    ) -> Option<Self> {
        if self.visited.len() >= MAX_SAVED_GROUP_DEPTH {
            return None;
        }
        let mut visited = self.visited.clone();
        if !visited.insert(group_id.to_owned()) {
            return None;
        }
        Some(Self {
            attributes: self.attributes,
            saved_groups: self.saved_groups,
            visited,
        })
    }

    /// Evaluate a nested value while retaining the current group-resolution path.
    pub fn with_attributes<'b>(
        &'b self,
        attributes: &'b [GrowthBookAttribute],
    ) -> ConditionEvalContext<'b> {
        ConditionEvalContext {
            attributes,
            saved_groups: self.saved_groups,
            visited: self.visited.clone(),
        }
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

/// Decode both payload shapes, retaining invalid entries so `$notInGroup`
/// cannot mistake them for absent ids and pass every user.
pub fn saved_groups_from_value(value: Option<&Value>) -> SavedGroups {
    let mut groups = SavedGroups::new();
    if let Some(Value::Object(map)) = value {
        for (id, entry) in map {
            groups.insert(id.clone(), SavedGroup::from(entry));
        }
    }
    groups
}
