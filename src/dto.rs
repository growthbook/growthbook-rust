use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use crate::extensions::JsonHelper;
use crate::model_public::{Experiment, GrowthBookAttribute, GrowthBookAttributeValue};
use crate::range::model::Range;

#[derive(Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct GrowthBookResponse {
    pub forced_variations: Option<HashMap<String, i64>>,
    pub features: Option<HashMap<String, GrowthBookFeature>>,
    pub encrypted_features: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GrowthBookFeature {
    pub default_value: Option<Value>,
    pub rules: Option<Vec<GrowthBookFeatureRule>>,
}

// A feature rule is deserialized into the flat `GrowthBookFeatureRuleDto`
// (which captures every possible field, so `parentConditions`/`condition`/
// `filters` can never be silently dropped), then normalized exactly once into
// this struct. `parent_conditions` is common to every rule kind; `kind` is a
// zero-cost enum the evaluation hot path matches on without probing options.
#[derive(Deserialize, Clone, Debug)]
#[serde(from = "GrowthBookFeatureRuleDto")]
pub struct GrowthBookFeatureRule {
    pub parent_conditions: Option<Vec<GrowthBookFeatureRuleParentData>>,
    pub kind: GrowthBookFeatureRuleKind,
}

#[derive(Clone, Debug)]
pub enum GrowthBookFeatureRuleKind {
    Experiment(GrowthBookFeatureRuleExperiment),
    Rollout(GrowthBookFeatureRuleRollout),
    Force(GrowthBookFeatureRuleForce),
    // Parent-only rules (or no-op rules) carry no value of their own.
    Empty,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GrowthBookFeatureRuleDto {
    parent_conditions: Option<Vec<GrowthBookFeatureRuleParentData>>,
    condition: Option<Value>,
    filters: Option<Value>,
    force: Option<Value>,
    coverage: Option<f32>,
    range: Option<Vec<f32>>,
    seed: Option<String>,
    hash_attribute: Option<String>,
    fallback_attribute: Option<String>,
    hash_version: Option<i64>,
    key: Option<String>,
    variations: Option<Vec<Value>>,
    name: Option<String>,
    weights: Option<Vec<f32>>,
    namespace: Option<Vec<Value>>,
    ranges: Option<Vec<Vec<f32>>>,
    meta: Option<Value>,
    bucket_version: Option<i64>,
    min_bucket_version: Option<i64>,
    disable_sticky_bucketing: Option<bool>,
}

impl From<GrowthBookFeatureRuleDto> for GrowthBookFeatureRule {
    fn from(dto: GrowthBookFeatureRuleDto) -> Self {
        let GrowthBookFeatureRuleDto {
            parent_conditions,
            condition,
            filters,
            force,
            coverage,
            range,
            seed,
            hash_attribute,
            fallback_attribute,
            hash_version,
            key,
            variations,
            name,
            weights,
            namespace,
            ranges,
            meta,
            bucket_version,
            min_bucket_version,
            disable_sticky_bucketing,
        } = dto;

        // Classification preserves the precedence of the former untagged enum
        // (Experiment → Rollout → Force → Empty).
        let kind = if let Some(variations) = variations {
            GrowthBookFeatureRuleKind::Experiment(GrowthBookFeatureRuleExperiment {
                key,
                variations,
                name,
                coverage,
                seed,
                hash_version,
                hash_attribute,
                fallback_attribute,
                weights,
                namespace,
                ranges,
                meta,
                filters,
                condition,
                bucket_version,
                min_bucket_version,
                disable_sticky_bucketing,
            })
        } else if let Some(force) = force {
            if let Some(coverage) = coverage {
                GrowthBookFeatureRuleKind::Rollout(GrowthBookFeatureRuleRollout {
                    force,
                    coverage,
                    range,
                    condition: value_to_condition_map(condition),
                    hash_attribute,
                    fallback_attribute,
                    hash_version,
                })
            } else {
                GrowthBookFeatureRuleKind::Force(GrowthBookFeatureRuleForce {
                    force,
                    coverage: None,
                    range,
                    hash_version,
                    filters,
                    seed,
                    condition: value_to_condition_map(condition),
                })
            }
        } else {
            GrowthBookFeatureRuleKind::Empty
        };

        GrowthBookFeatureRule { parent_conditions, kind }
    }
}

fn value_to_condition_map(condition: Option<Value>) -> Option<HashMap<String, Value>> {
    condition.and_then(|value| value.as_object().map(|map| map.clone().into_iter().collect()))
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GrowthBookFeatureRuleForce {
    pub force: Value,
    pub coverage: Option<f32>,
    range: Option<Vec<f32>>,
    pub hash_version: Option<i64>,
    pub filters: Option<Value>,
    pub seed: Option<String>,
    condition: Option<HashMap<String, Value>>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GrowthBookFeatureRuleParentData {
    pub id: String,
    condition: Option<HashMap<String, Value>>,
    #[serde(default)]
    pub gate: bool,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GrowthBookFeatureRuleRollout {
    pub force: Value,
    pub coverage: f32,
    range: Option<Vec<f32>>,
    condition: Option<HashMap<String, Value>>,
    pub hash_attribute: Option<String>,
    pub fallback_attribute: Option<String>,
    pub hash_version: Option<i64>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GrowthBookFeatureRuleExperiment {
    pub key: Option<String>,
    pub variations: Vec<Value>,
    name: Option<String>,
    pub coverage: Option<f32>,
    seed: Option<String>,
    pub hash_version: Option<i64>,
    pub hash_attribute: Option<String>,
    pub fallback_attribute: Option<String>,
    weights: Option<Vec<f32>>,
    pub namespace: Option<Vec<Value>>,
    pub ranges: Option<Vec<Vec<f32>>>,
    pub meta: Option<Value>,
    pub filters: Option<Value>,
    pub condition: Option<Value>,
    pub bucket_version: Option<i64>,
    pub min_bucket_version: Option<i64>,
    pub disable_sticky_bucketing: Option<bool>,
}

impl GrowthBookFeatureRuleParentData {
    pub fn conditions(&self) -> Option<Vec<GrowthBookAttribute>> {
        option_map_to_attributes(self.condition.clone())
    }
}

impl GrowthBookFeatureRuleRollout {
    pub fn conditions(&self) -> Option<Vec<GrowthBookAttribute>> {
        option_map_to_attributes(self.condition.clone())
    }

    pub fn range(&self) -> Option<Range> {
        Range::get_range(self.range.clone())
    }

    pub fn get_fallback_attribute(&self) -> String {
        self.fallback_attribute.clone().unwrap_or(String::from("id"))
    }
}

impl GrowthBookFeatureRuleForce {
    pub fn conditions(&self) -> Option<Vec<GrowthBookAttribute>> {
        option_map_to_attributes(self.condition.clone())
    }

    pub fn range(&self) -> Option<Range> {
        Range::get_range(self.range.clone())
    }

    pub fn get_fallback_attribute(&self) -> String {
        String::from("id")
    }
}

impl GrowthBookFeatureRuleExperiment {
    pub fn seed(
        &self,
        feature_name: &str,
    ) -> String {
        self.seed.clone().unwrap_or(self.key.clone().unwrap_or(feature_name.to_string()))
    }

    pub fn ranges(&self) -> Vec<Range> {
        if let Some(ranges) = self.ranges.clone() {
            ranges.iter().map(|range| Range { start: range[0], end: range[1] }).collect()
        } else {
            Range::get_bucket_range(self.variations.len() as i64, &self.coverage, self.weights.clone())
        }
    }

    pub fn namespace_range(&self) -> Option<(String, Range)> {
        self.namespace.as_ref().map(|namespace| {
            (
                namespace[0].force_string(""),
                Range {
                    start: namespace[1].force_f32(0.0),
                    end: namespace[2].force_f32(1.0),
                },
            )
        })
    }

    pub fn model_experiment(&self) -> Experiment {
        Experiment {
            name: self.name.clone(),
            seed: self.seed.clone(),
            hash_version: self.hash_version,
            hash_attribute: self.hash_attribute.clone(),
            namespace: self.namespace.clone(),
            coverage: self.coverage,
            ranges: self.ranges.clone(),
            meta: self.meta.clone(),
            filters: self.filters.clone(),
            variations: self.variations.clone(),
            weights: self.weights.clone(),
            condition: self.condition.clone(),
        }
    }
}

pub fn option_map_to_attributes(option_map: Option<HashMap<String, Value>>) -> Option<Vec<GrowthBookAttribute>> {
    option_map.map(|conditions| conditions.iter().map(|(k, v)| GrowthBookAttribute::new(k.clone(), GrowthBookAttributeValue::from(v.clone()))).collect())
}
