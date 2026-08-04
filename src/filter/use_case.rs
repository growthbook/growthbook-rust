use serde_json::Value;

use crate::extensions::{FindGrowthBookAttribute, JsonHelper};
use crate::hash::{HashCode, HashCodeVersion};
use crate::model_public::GrowthBookAttribute;
use crate::range::model::Range;

pub struct Filter;

impl Filter {
    /// Whether the user is excluded by any of the filters (JS `isFilteredOut`).
    ///
    /// A user is filtered out if *any* filter excludes them: they have no value
    /// for that filter's hash attribute, or their hashed weight isn't in any of
    /// the filter's ranges. Each filter hashes on its own `attribute`
    /// (JS `getHashAttribute(filter.attribute)`), falling back to
    /// `default_attribute` (callers pass `"id"`, matching JS).
    pub fn is_filtered_out(
        filters: &Value,
        default_attribute: &str,
        user_attributes: &Vec<GrowthBookAttribute>,
    ) -> bool {
        filters.force_array(vec![]).iter().any(|filter| {
            let attribute = filter.get_string("attribute", default_attribute);
            let Some(user_value) = user_attributes.find_value(&attribute) else {
                return true;
            };

            let hash_version = filter.get("hashVersion").and_then(|it| it.as_i64()).unwrap_or(2);
            let Some(user_weight) = HashCode::hash_code(&user_value.to_string(), &filter.get_string("seed", ""), HashCodeVersion::from(hash_version)) else {
                return true;
            };

            // Excluded by this filter unless the weight is in one of its ranges.
            !filter.get_array("ranges", vec![]).iter().any(|array| {
                Range {
                    start: array[0].force_f32(0.0),
                    end: array[1].force_f32(1.0),
                }
                .in_range(&user_weight)
            })
        })
    }
}
