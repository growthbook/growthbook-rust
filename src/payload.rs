//! Payload reduction for server-side rendering: shrink a features payload for a
//! user whose attributes are only partly known, so a client SDK can be
//! bootstrapped with less data. Ports `pruneCondition` and `reducePayload` from
//! the JavaScript SDK.

use std::cell::Cell;
use std::collections::{BTreeMap, HashSet};

use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::condition::eval_context::{saved_groups_from_value, ConditionEvalContext, SavedGroups, MAX_SAVED_GROUP_DEPTH, MAX_SAVED_GROUP_EVALUATIONS};
use crate::condition::use_case::evaluate;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

/// Filters on payload metadata. Each one is a list, and any one of its values is enough.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadFilters {
    pub projects: Option<Vec<Value>>,
    pub tags: Option<Vec<Value>>,
    pub custom_fields: Option<BTreeMap<String, Vec<Value>>>,
}

/// Whether prerequisites follow the filters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Prerequisites {
    /// Send the prerequisites of what is kept even when the filters remove
    /// them, so the client evaluates the same as with the full payload.
    #[default]
    Keep,
    /// Apply the filters to prerequisites too. Holdouts are always kept.
    Filter,
}

/// Options for [`reduce_payload`].
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReducePayloadOptions {
    /// Keep only the features and auto experiments that match every filter.
    #[serde(default)]
    pub include: PayloadFilters,
    /// Remove the features and auto experiments that match any filter.
    #[serde(default)]
    pub exclude: PayloadFilters,
    #[serde(default)]
    pub prerequisites: Prerequisites,
}

/// Partially evaluate a condition when only some attributes are known, e.g. on
/// a server. Attributes missing from `attributes` are unknown.
///
/// Returns `true` or `false` once the result can't depend on them, otherwise
/// the condition minus every branch that is already decided. A malformed
/// condition is returned as written.
pub fn prune_condition(
    attributes: &Value,
    condition: &Value,
    saved_groups: &Value,
) -> Value {
    Pruner::new(attributes, saved_groups).prune_root(condition)
}

/// Shrink a decrypted features payload for a user whose attributes are only
/// partly known. The result evaluates the same as the full payload for anyone
/// who agrees with the known attributes. Keys this SDK doesn't use are kept.
pub fn reduce_payload(
    payload: &Value,
    attributes: &Value,
    options: &ReducePayloadOptions,
) -> Value {
    let empty = Map::new();
    let all_features = payload.get("features").and_then(Value::as_object).unwrap_or(&empty);
    let all_experiments = payload.get("experiments").and_then(Value::as_array).cloned().unwrap_or_default();
    let saved_groups = payload.get("savedGroups").cloned().unwrap_or_else(|| json!({}));
    let pruner = Pruner::new(attributes, &saved_groups);
    let filters = Filters(options);

    let mut reducer = Reducer {
        all_features,
        pruner: &pruner,
        filters: &filters,
        prerequisites: options.prerequisites,
        added: Map::new(),
    };
    for (id, feature) in all_features {
        if filters.matches(feature.get("metadata")) {
            reducer.add(id);
        }
    }
    let experiments: Vec<Value> = all_experiments.iter().filter(|it| filters.matches(it.get("metadata"))).map(|it| pruner.prune_item(it, false)).collect();
    for experiment in &experiments {
        reducer.add_parents(experiment);
    }
    let features = reducer.added;

    // The contextual bandits that kept rules and experiments use. A context
    // whose condition never passes can't be the first that does, so it goes.
    let all_bandits = payload.get("contextualBandits").and_then(Value::as_object).unwrap_or(&empty);
    let mut bandits = Map::new();
    let rules = features.values().filter_map(|it| it.get("rules").and_then(Value::as_array)).flatten();
    for item in rules.chain(experiments.iter()) {
        let Some(reference) = item.get("contextualBanditRef").and_then(Value::as_str) else {
            continue;
        };
        let Some(bandit) = all_bandits.get(reference) else {
            continue;
        };
        if bandits.contains_key(reference) {
            continue;
        }
        let mut bandit = bandit.clone();
        if let Some(fields) = bandit.as_object_mut() {
            let contexts = fields.get("contexts").and_then(Value::as_array).cloned().unwrap_or_default();
            let contexts = contexts
                .into_iter()
                .filter_map(|mut context| {
                    let condition = context.get("condition").filter(|it| is_truthy(it)).cloned().unwrap_or_else(|| json!({}));
                    let pruned = pruner.prune_root(&condition);
                    if pruned == Value::Bool(false) {
                        return None;
                    }
                    if let Some(context) = context.as_object_mut() {
                        context.insert(String::from("condition"), if pruned == Value::Bool(true) { json!({}) } else { pruned });
                    }
                    Some(context)
                })
                .collect();
            fields.insert(String::from("contexts"), Value::Array(contexts));
        }
        bandits.insert(reference.to_owned(), bandit);
    }

    // Keep each saved group whose id appears as a string anywhere, with
    // condition groups reduced. This needs no list of the operators that
    // reference groups.
    let roots = Value::Array(vec![Value::Object(features.clone()), Value::Array(experiments.clone()), Value::Object(bandits.clone())]);
    let mut marks = GroupMarks::default();
    marks.mark(&roots, &saved_groups, true);
    let mut groups = Map::new();
    keep_groups(&roots, &pruner, &marks.as_written, &mut groups);

    let mut reduced = payload.as_object().cloned().unwrap_or_default();
    reduced.insert(String::from("features"), Value::Object(features));
    reduced.insert(String::from("experiments"), Value::Array(experiments));
    reduced.insert(String::from("savedGroups"), Value::Object(groups));
    // Only when there were any, as with the payload's other optional keys
    if reduced.get("contextualBandits").is_some_and(is_truthy) {
        reduced.insert(String::from("contextualBandits"), Value::Object(bandits));
    }
    Value::Object(reduced)
}

/// A condition JS would throw on, or one that hits the evaluator's saved-group
/// limits. Pruning leaves the whole condition as written.
struct Malformed;

struct Pruner<'a> {
    attributes: Map<String, Value>,
    attribute_list: Vec<GrowthBookAttribute>,
    saved_groups: &'a Value,
    typed_groups: SavedGroups,
    /// Saved groups entered while pruning one condition, shared with the
    /// evaluations it runs, as in the evaluator.
    group_evaluations: Cell<usize>,
}

impl<'a> Pruner<'a> {
    fn new(
        attributes: &Value,
        saved_groups: &'a Value,
    ) -> Self {
        let attributes = attributes.as_object().cloned().unwrap_or_default();
        Self {
            attribute_list: attributes
                .iter()
                .map(|(key, value)| GrowthBookAttribute::new(key.clone(), GrowthBookAttributeValue::from(value.clone())))
                .collect(),
            attributes,
            saved_groups,
            typed_groups: saved_groups_from_value(Some(saved_groups)),
            group_evaluations: Cell::new(0),
        }
    }

    /// Prune a whole condition, with a fresh saved-group work limit.
    fn prune_root(
        &self,
        condition: &Value,
    ) -> Value {
        self.group_evaluations.set(0);
        self.prune(condition, &HashSet::new())
    }

    /// Prune a saved group's own condition, with a fresh saved-group work limit.
    fn prune_root_group(
        &self,
        reference: &Value,
    ) -> Result<Value, Malformed> {
        self.group_evaluations.set(0);
        self.prune_saved_group(reference, &HashSet::new(), true)
    }

    fn prune(
        &self,
        condition: &Value,
        visited: &HashSet<String>,
    ) -> Value {
        self.prune_entries(condition, visited).unwrap_or_else(|Malformed| condition.clone())
    }

    /// Reduce a rule or an auto experiment.
    fn prune_item(
        &self,
        item: &Value,
        is_rule: bool,
    ) -> Value {
        let mut item = item.clone();
        let Some(fields) = item.as_object_mut() else {
            return item;
        };
        let Some(condition) = fields.get("condition").filter(|it| is_truthy(it)) else {
            return item;
        };
        match self.prune_root(condition) {
            Value::Bool(true) => {
                fields.remove("condition");
            },
            // Kept, since sticky buckets and forced variations skip an
            // experiment's condition, and a rule's prerequisites are checked before it
            Value::Bool(false) => {
                fields.insert(String::from("condition"), json!({"$not": {}}));
                // A force rule that never passes never returns its value. An auto
                // experiment's forced variation still applies to a sticky bucketed user.
                if is_rule && fields.contains_key("force") {
                    fields.insert(String::from("force"), Value::Null);
                }
            },
            pruned => {
                fields.insert(String::from("condition"), pruned);
            },
        }
        item
    }

    fn prune_entries(
        &self,
        condition: &Value,
        visited: &HashSet<String>,
    ) -> Result<Value, Malformed> {
        let mut out = Map::new();
        for (key, value) in entries(condition)? {
            let result = match key.as_str() {
                "$or" | "$nor" | "$and" => self.prune_list(&value, visited, key != "$and")?,
                "$not" => self.prune_entries(&value, visited)?,
                "$savedGroup" => match self.prune_saved_group(&value, visited, false)? {
                    // Keep an undecided reference as is rather than inlining the group
                    decided @ Value::Bool(_) => decided,
                    _ => Value::Null,
                },
                // Null when the attribute is unknown, or the value uses an
                // operator this SDK doesn't support, which the client's SDK may
                _ if self.is_known(&key) => match self.evaluate(&key, &value, visited)? {
                    Some(result) => Value::Bool(result),
                    None => Value::Null,
                },
                _ => Value::Null,
            };
            // $not and $nor pass when what they hold fails
            let result = match (key.as_str(), result) {
                ("$not" | "$nor", Value::Bool(it)) => Value::Bool(!it),
                (_, result) => result,
            };
            // One failing entry fails the whole condition, and a passing one is dropped
            match result {
                Value::Bool(false) => return Ok(Value::Bool(false)),
                Value::Bool(true) => {},
                // An undecided leaf stays as written
                Value::Null => {
                    out.insert(key, value);
                },
                pruned => {
                    out.insert(key, pruned);
                },
            }
        }
        Ok(if out.is_empty() { Value::Bool(true) } else { Value::Object(out) })
    }

    /// Prune an `$or` (or an `$and`) list. Returns the conditions still
    /// undecided, or the list's result once there are none.
    fn prune_list(
        &self,
        conditions: &Value,
        visited: &HashSet<String>,
        or: bool,
    ) -> Result<Value, Malformed> {
        let conditions = list(conditions)?;
        let mut out = vec![];
        for condition in &conditions {
            match self.prune_entries(condition, visited)? {
                // One passing condition decides an $or, and one failing condition an $and
                Value::Bool(true) if or => return Ok(Value::Bool(true)),
                Value::Bool(false) if !or => return Ok(Value::Bool(false)),
                Value::Bool(_) => {},
                pruned => out.push(pruned),
            }
        }
        if !out.is_empty() {
            return Ok(Value::Array(out));
        }
        // Every condition failed an $or, or passed an $and. An empty $or passes.
        Ok(Value::Bool(if or { conditions.is_empty() } else { true }))
    }

    /// Resolve a `$savedGroup` reference: its result once decided, otherwise
    /// the group's condition minus every decided branch, or null when the
    /// group's attribute is unknown or its type is new. `catch` returns a
    /// malformed group condition as written instead of failing the caller.
    fn prune_saved_group(
        &self,
        reference: &Value,
        visited: &HashSet<String>,
        catch: bool,
    ) -> Result<Value, Malformed> {
        let Some(reference) = reference.as_object() else {
            return Ok(Value::Bool(false));
        };
        let Some(id) = reference.get("id").and_then(Value::as_str) else {
            return Ok(Value::Bool(false));
        };
        if visited.contains(id) {
            return Ok(Value::Bool(false));
        }
        // The evaluator's limits fail the whole condition, so a condition that
        // reaches them can't be decided here: it stays as written
        let evaluations = self.group_evaluations.get() + 1;
        if evaluations > MAX_SAVED_GROUP_EVALUATIONS || visited.len() >= MAX_SAVED_GROUP_DEPTH {
            return Err(Malformed);
        }
        self.group_evaluations.set(evaluations);
        let attribute_key = match reference.get("attributeKey") {
            None => None,
            Some(Value::String(key)) => Some(key.as_str()),
            // Falling back to the entry's attribute would check a different population
            Some(_) => return Ok(Value::Bool(false)),
        };
        let Some(entry) = self.saved_groups.get(id).and_then(Value::as_object) else {
            return Ok(Value::Bool(false));
        };
        match entry.get("type").and_then(Value::as_str) {
            Some("list") => {
                let Some(key) = attribute_key.or_else(|| entry.get("attributeKey").and_then(Value::as_str)) else {
                    return Ok(Value::Bool(false));
                };
                if !entry.get("values").is_some_and(Value::is_array) {
                    return Ok(Value::Bool(false));
                }
                if !self.is_known(key) {
                    return Ok(Value::Null);
                }
                let condition = [GrowthBookAttribute::new(
                    String::from("$savedGroup"),
                    GrowthBookAttributeValue::from(json!({"id": id, "attributeKey": key})),
                )];
                Ok(Value::Bool(evaluate(&condition, &self.context()) == Some(true)))
            },
            Some("condition") => {
                let Some(condition) = entry.get("condition").filter(|it| it.is_object()) else {
                    return Ok(Value::Bool(false));
                };
                // A copy, so sibling groups don't see each other's ids
                let mut visited = visited.clone();
                visited.insert(id.to_owned());
                if catch {
                    Ok(self.prune(condition, &visited))
                } else {
                    self.prune_entries(condition, &visited)
                }
            },
            // A group type added after this SDK was built. The client's SDK may support it.
            _ => Ok(Value::Null),
        }
    }

    /// A nested path is unknown, since the server may only have part of the object.
    fn is_known(
        &self,
        path: &str,
    ) -> bool {
        !path.contains('.') && self.attributes.contains_key(path)
    }

    /// Evaluate a known attribute, continuing the current saved-group path.
    fn evaluate(
        &self,
        key: &str,
        value: &Value,
        visited: &HashSet<String>,
    ) -> Result<Option<bool>, Malformed> {
        let condition = [GrowthBookAttribute::new(key.to_owned(), GrowthBookAttributeValue::from(value.clone()))];
        let ctx = ConditionEvalContext::within(&self.attribute_list, &self.typed_groups, visited.clone(), &self.group_evaluations);
        let result = evaluate(&condition, &ctx);
        if ctx.work_limit_exceeded() || ctx.is_malformed() {
            return Err(Malformed);
        }
        Ok(result)
    }

    fn context(&self) -> ConditionEvalContext<'_> {
        ConditionEvalContext::new(&self.attribute_list, &self.typed_groups)
    }
}

/// The entries JS `Object.entries` would return, or `Malformed` for null,
/// where it throws.
fn entries(value: &Value) -> Result<Vec<(String, Value)>, Malformed> {
    Ok(match value {
        Value::Null => return Err(Malformed),
        Value::Object(fields) => fields.iter().map(|(key, value)| (key.clone(), value.clone())).collect(),
        Value::Array(items) => items.iter().enumerate().map(|(index, item)| (index.to_string(), item.clone())).collect(),
        Value::String(text) => text.chars().enumerate().map(|(index, char)| (index.to_string(), Value::String(char.to_string()))).collect(),
        Value::Bool(_) | Value::Number(_) => vec![],
    })
}

/// The items a JS `for...of` loop would visit, or `Malformed` where it throws.
fn list(value: &Value) -> Result<Vec<Value>, Malformed> {
    match value {
        Value::Array(items) => Ok(items.clone()),
        Value::String(text) => Ok(text.chars().map(|char| Value::String(char.to_string())).collect()),
        _ => Err(Malformed),
    }
}

/// JavaScript truthiness.
fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(it) => *it,
        Value::Number(it) => it.as_f64().is_some_and(|it| it != 0.0 && !it.is_nan()),
        Value::String(it) => !it.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// JS `Array.isArray(want) && isIn(actual, want)`: any value of `actual`, or
/// `actual` itself, is in `want`, compared with `===`.
fn has(
    actual: Option<&Value>,
    want: Option<&Vec<Value>>,
) -> bool {
    // A missing value is `undefined`, which no JSON value equals
    let (Some(want), Some(actual)) = (want, actual) else {
        return false;
    };
    let includes = |value: &Value| want.iter().any(|it| strict_eq(it, value));
    match actual {
        Value::Array(items) => items.iter().any(includes),
        other => includes(other),
    }
}

/// JS `===` on JSON values: numbers by value, objects and arrays never equal.
fn strict_eq(
    a: &Value,
    b: &Value,
) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        (Value::Array(_) | Value::Object(_), _) | (_, Value::Array(_) | Value::Object(_)) => false,
        _ => a == b,
    }
}

struct Filters<'a>(&'a ReducePayloadOptions);

impl Filters<'_> {
    /// A rule without projects applies wherever its feature is delivered.
    fn rule_in_projects(
        &self,
        meta: Option<&Value>,
    ) -> bool {
        let include = &self.0.include;
        include.projects.is_none()
            || !meta.is_some_and(is_truthy)
            || !meta.and_then(|it| it.get("projects")).is_some_and(is_truthy)
            || has(meta.and_then(|it| it.get("projects")), include.projects.as_ref())
    }

    /// A feature or experiment has to list one of the projects, or be in all
    /// of them. One without projects is in none, like one without tags.
    fn in_projects(
        &self,
        meta: Option<&Value>,
    ) -> bool {
        let include = &self.0.include;
        let field = |key: &str| meta.and_then(|it| it.get(key));
        include.projects.is_none() || field("allProjects").is_some_and(is_truthy) || has(field("projects"), include.projects.as_ref())
    }

    /// Kept when it matches every `include` filter and no `exclude` filter.
    /// Only listed projects are excluded, so one in all projects stays.
    fn matches(
        &self,
        meta: Option<&Value>,
    ) -> bool {
        let meta = meta.filter(|it| is_truthy(it));
        let (include, exclude) = (&self.0.include, &self.0.exclude);
        let field = |key: &str| meta.and_then(|it| it.get(key));
        let custom = |key: &str| field("customFields").and_then(|it| it.get(key));
        let no_fields = BTreeMap::new();
        self.in_projects(meta)
            && (include.tags.is_none() || has(field("tags"), include.tags.as_ref()))
            && include.custom_fields.as_ref().unwrap_or(&no_fields).iter().all(|(key, want)| has(custom(key), Some(want)))
            && !has(field("projects"), exclude.projects.as_ref())
            && !has(field("tags"), exclude.tags.as_ref())
            && !exclude.custom_fields.as_ref().unwrap_or(&no_fields).iter().any(|(key, want)| has(custom(key), Some(want)))
    }
}

struct Reducer<'a> {
    all_features: &'a Map<String, Value>,
    pruner: &'a Pruner<'a>,
    filters: &'a Filters<'a>,
    prerequisites: Prerequisites,
    added: Map<String, Value>,
}

impl Reducer<'_> {
    /// Add a feature, then the prerequisites the client evaluates for it, even
    /// when the filters drop them. With `Filter`, only the filters decide,
    /// except for holdouts, which aren't app features and are always kept.
    fn add(
        &mut self,
        id: &str,
    ) {
        let Some(feature) = self.all_features.get(id).filter(|it| is_truthy(it)) else {
            return;
        };
        if self.added.contains_key(id) {
            return;
        }
        // Rules scoped to other projects aren't served. A prerequisite from
        // another project keeps its own, as on the server.
        let scoped = self.filters.in_projects(feature.get("metadata"));
        let rules: Option<Vec<Value>> = feature.get("rules").and_then(Value::as_array).map(|rules| {
            rules
                .iter()
                .filter(|rule| !scoped || self.filters.rule_in_projects(rule.get("metadata")))
                .map(|rule| self.pruner.prune_item(rule, true))
                .collect()
        });
        let mut feature = feature.clone();
        if let (Some(rules), Some(fields)) = (&rules, feature.as_object_mut()) {
            fields.insert(String::from("rules"), Value::Array(rules.clone()));
        }
        // Stored before its parents are added, so a prerequisite cycle ends
        self.added.insert(id.to_owned(), feature);
        for rule in rules.unwrap_or_default() {
            self.add_parents(&rule);
        }
    }

    fn add_parents(
        &mut self,
        item: &Value,
    ) {
        let parents = item.get("parentConditions").and_then(Value::as_array).cloned().unwrap_or_default();
        for parent in parents {
            let Some(id) = parent.get("id").and_then(Value::as_str) else {
                continue;
            };
            if self.prerequisites != Prerequisites::Filter || id.starts_with("$holdout:") {
                self.add(id);
            }
        }
    }
}

/// Saved groups that keep their condition as written: those evaluated against
/// something other than the user (each item in `$elemMatch`, or the parent's
/// value in a prerequisite), and the groups those reference.
#[derive(Default)]
struct GroupMarks {
    as_written: HashSet<String>,
    seen: HashSet<String>,
}

impl GroupMarks {
    fn mark(
        &mut self,
        value: &Value,
        saved_groups: &Value,
        user: bool,
    ) {
        match value {
            Value::String(id) => {
                let Some(entry) = saved_groups.get(id.as_str()) else {
                    return;
                };
                if self.as_written.contains(id) || (user && self.seen.contains(id)) {
                    return;
                }
                if user { &mut self.seen } else { &mut self.as_written }.insert(id.clone());
                self.mark(entry, saved_groups, user);
            },
            Value::Array(items) => items.iter().for_each(|item| self.mark(item, saved_groups, user)),
            Value::Object(fields) => {
                for (key, item) in fields {
                    self.mark(item, saved_groups, user && key != "$elemMatch" && key != "parentConditions");
                }
            },
            _ => {},
        }
    }
}

/// The saved group ids anywhere in a value.
fn group_ids<'v>(
    value: &'v Value,
    saved_groups: &Value,
    found: &mut Vec<&'v str>,
) {
    match value {
        Value::String(id) if saved_groups.get(id.as_str()).is_some() => found.push(id),
        Value::Array(items) => items.iter().for_each(|item| group_ids(item, saved_groups, found)),
        Value::Object(fields) => fields.values().for_each(|item| group_ids(item, saved_groups, found)),
        _ => {},
    }
}

/// Whether group `from` references group `to`, directly or through other groups.
fn reaches(
    from: &str,
    to: &str,
    saved_groups: &Value,
    visited: &mut HashSet<String>,
) -> bool {
    let mut found = vec![];
    if let Some(entry) = saved_groups.get(from) {
        group_ids(entry, saved_groups, &mut found);
    }
    found.into_iter().any(|id| id == to || (visited.insert(id.to_owned()) && reaches(id, to, saved_groups, visited)))
}

fn keep_groups(
    value: &Value,
    pruner: &Pruner,
    as_written: &HashSet<String>,
    groups: &mut Map<String, Value>,
) {
    match value {
        Value::String(id) => {
            let Some(entry) = pruner.saved_groups.get(id.as_str()) else {
                return;
            };
            if groups.contains_key(id.as_str()) {
                return;
            }
            // A group on a cycle can't be reduced on its own: its result
            // depends on which group the evaluation entered the cycle from
            let reduce = !as_written.contains(id.as_str()) && !reaches(id, id, pruner.saved_groups, &mut HashSet::new());
            let reference = json!({"id": id});
            let pruned = if reduce { pruner.prune_root_group(&reference) } else { Err(Malformed) };
            let kept = match pruned {
                Ok(condition @ Value::Object(_)) => {
                    let mut entry = entry.clone();
                    if let Some(fields) = entry.as_object_mut() {
                        fields.insert(String::from("condition"), condition);
                    }
                    entry
                },
                _ => entry.clone(),
            };
            groups.insert(id.clone(), kept.clone());
            keep_groups(&kept, pruner, as_written, groups);
        },
        Value::Array(items) => items.iter().for_each(|item| keep_groups(item, pruner, as_written, groups)),
        Value::Object(fields) => fields.values().for_each(|item| keep_groups(item, pruner, as_written, groups)),
        _ => {},
    }
}
