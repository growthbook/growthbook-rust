use crate::condition::elem_match_comparison::ElemMatchComparison;
use crate::condition::eval_context::ConditionEvalContext;
use crate::condition::operator_condition::OperatorCondition;
use crate::condition::order_comparison::OrderComparison;
use crate::condition::regex_comparison::RegexComparison;
use crate::condition::size_comparison::SizeComparison;
use crate::condition::type_comparison::TypeComparison;
use crate::condition::version_comparison::VersionComparison;
use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue, SavedGroup};

/// A condition result: `Some` once decided, or `None` when it depends on an
/// operator or saved group type this SDK doesn't support. Only `Some(true)`
/// passes, so an unsupported part also fails under `$not` and `$nor`.
pub(crate) type Tri = Option<bool>;

/// Every result must pass: one failure decides it, wherever an unknown one is.
pub(super) fn all_of(results: impl IntoIterator<Item = Tri>) -> Tri {
    let mut result = Some(true);
    for next in results {
        match next {
            Some(false) => return Some(false),
            None => result = None,
            Some(true) => {},
        }
    }
    result
}

/// One result must pass: one pass decides it, wherever an unknown one is.
pub(super) fn any_of(results: impl IntoIterator<Item = Tri>) -> Tri {
    let mut result = Some(false);
    for next in results {
        match next {
            Some(true) => return Some(true),
            None => result = None,
            Some(false) => {},
        }
    }
    result
}

/// Condition keys that aren't attributes, besides `$not`, which is also an
/// operator on a value.
pub(super) const CONDITION_OPERATORS: [&str; 4] = ["$or", "$nor", "$and", "$savedGroup"];

pub trait ConditionsMatchesAttributes {
    fn matches(
        &self,
        ctx: &ConditionEvalContext,
    ) -> bool;
}

impl ConditionsMatchesAttributes for Vec<GrowthBookAttribute> {
    fn matches(
        &self,
        ctx: &ConditionEvalContext,
    ) -> bool {
        !ctx.work_limit_exceeded() && evaluate(self, ctx) == Some(true) && !ctx.work_limit_exceeded() && !ctx.is_malformed()
    }
}

/// Evaluate a condition's entries without collapsing an unknown result.
pub(crate) fn evaluate(
    condition: &[GrowthBookAttribute],
    ctx: &ConditionEvalContext,
) -> Tri {
    all_of(condition.iter().map(|it| verify(None, it, ctx)))
}

fn verify(
    parent_attribute: Option<&GrowthBookAttribute>,
    feature_attribute: &GrowthBookAttribute,
    ctx: &ConditionEvalContext,
) -> Tri {
    if ctx.work_limit_exceeded() || ctx.is_malformed() {
        return Some(false);
    }
    let decided = |result: bool| Some(result);
    match feature_attribute.key.as_str() {
        "$savedGroup" if parent_attribute.is_none() => saved_group(&feature_attribute.value, ctx),
        "$savedGroup" | "$savedGroups" => Some(false),
        "$and" | "$or" | "$nor" if parent_attribute.is_some() => Some(false),
        "$not" => OperatorCondition::not(parent_attribute, feature_attribute, ctx, verify),
        "$ne" => decided(OperatorCondition::ne(parent_attribute, feature_attribute, ctx)),
        "$and" => OperatorCondition::and(parent_attribute, feature_attribute, ctx, verify),
        "$nor" => OperatorCondition::nor(parent_attribute, feature_attribute, ctx, verify),
        "$or" => OperatorCondition::or(parent_attribute, feature_attribute, ctx, verify),
        "$in" => decided(OperatorCondition::is_in(parent_attribute, feature_attribute, ctx, false)),
        "$nin" => decided(OperatorCondition::nin(parent_attribute, feature_attribute, ctx, false)),
        "$gt" => decided(OrderComparison::gt(parent_attribute, feature_attribute, ctx)),
        "$gte" => decided(OrderComparison::gte(parent_attribute, feature_attribute, ctx)),
        "$lt" => decided(OrderComparison::lt(parent_attribute, feature_attribute, ctx)),
        "$lte" => decided(OrderComparison::lte(parent_attribute, feature_attribute, ctx)),
        "$eq" => decided(OperatorCondition::eq(parent_attribute, feature_attribute, ctx)),
        "$exists" => decided(OperatorCondition::exists(parent_attribute, feature_attribute, ctx)),
        "$regex" => decided(RegexComparison::matches(parent_attribute, feature_attribute, ctx)),
        "$type" => decided(TypeComparison::matches(parent_attribute, feature_attribute, ctx)),
        "$size" => SizeComparison::matches(parent_attribute, feature_attribute, ctx),
        "$all" => OperatorCondition::all(parent_attribute, feature_attribute, ctx, false),
        "$vgt" => decided(VersionComparison::vgt(parent_attribute, feature_attribute, ctx)),
        "$vgte" => decided(VersionComparison::vgte(parent_attribute, feature_attribute, ctx)),
        "$vlt" => decided(VersionComparison::vlt(parent_attribute, feature_attribute, ctx)),
        "$vlte" => decided(VersionComparison::vlte(parent_attribute, feature_attribute, ctx)),
        "$veq" => decided(VersionComparison::veq(parent_attribute, feature_attribute, ctx)),
        "$vne" => decided(VersionComparison::vne(parent_attribute, feature_attribute, ctx)),
        "$elemMatch" => ElemMatchComparison::matches(parent_attribute, feature_attribute, ctx),
        "$ini" => decided(OperatorCondition::is_in(parent_attribute, feature_attribute, ctx, true)),
        "$nini" => decided(OperatorCondition::nin(parent_attribute, feature_attribute, ctx, true)),
        "$alli" => OperatorCondition::all(parent_attribute, feature_attribute, ctx, true),
        "$regexi" => decided(RegexComparison::matches_ignore_case(parent_attribute, feature_attribute, ctx)),
        "$notRegex" => decided(RegexComparison::not_matches(parent_attribute, feature_attribute, ctx)),
        "$notRegexi" => decided(RegexComparison::not_matches_ignore_case(parent_attribute, feature_attribute, ctx)),
        "$inGroup" => OperatorCondition::in_group(parent_attribute, feature_attribute, ctx),
        "$notInGroup" => OperatorCondition::not_in_group(parent_attribute, feature_attribute, ctx),
        // An operator this SDK doesn't know. The client's SDK may.
        key if key.starts_with('$') && parent_attribute.is_some() => None,
        _ => non_operator_or_condition(parent_attribute, feature_attribute, ctx),
    }
}

/// Whether an object represents attribute operators rather than a condition
/// evaluated against the fields of an object. `{}` has no operators, so it passes.
pub(super) fn is_operator_object(fields: &[GrowthBookAttribute]) -> bool {
    fields.iter().all(|field| field.key.starts_with('$'))
}

/// Evaluate an array element or size without resetting group cycle detection.
pub(super) fn matches_value(
    actual: &GrowthBookAttributeValue,
    expected: &GrowthBookAttributeValue,
    ctx: &ConditionEvalContext,
    case_insensitive: bool,
) -> Tri {
    if let GrowthBookAttributeValue::Object(fields) = expected {
        if is_operator_object(fields) {
            let attributes = [GrowthBookAttribute::new(String::from("value"), actual.clone())];
            let nested = ctx.with_attributes(&attributes);
            return all_of(fields.iter().map(|field| verify(Some(&attributes[0]), field, &nested)));
        }
    }
    if case_insensitive {
        if let (GrowthBookAttributeValue::String(actual), GrowthBookAttributeValue::String(expected)) = (actual, expected) {
            return Some(actual.to_lowercase() == expected.to_lowercase());
        }
    }
    Some(actual.json_eq(expected))
}

/// Resolve a condition-level reference. Unknown reference fields are ignored.
fn saved_group(
    reference: &GrowthBookAttributeValue,
    ctx: &ConditionEvalContext,
) -> Tri {
    let GrowthBookAttributeValue::Object(fields) = reference else {
        return Some(false);
    };
    let field = |key: &str| fields.iter().find(|field| field.key == key).map(|field| &field.value);
    let Some(GrowthBookAttributeValue::String(id)) = field("id") else {
        return Some(false);
    };
    let attribute_key = match field("attributeKey") {
        None => None,
        Some(GrowthBookAttributeValue::String(key)) => Some(key),
        _ => return Some(false),
    };
    let Some(next) = ctx.enter_group(id) else {
        return Some(false);
    };
    match ctx.saved_group(id) {
        Some(SavedGroup::List {
            attribute_key: entry_key,
            values: Some(values),
        }) => {
            let Some(key) = attribute_key.or(entry_key.as_ref()) else {
                return Some(false);
            };
            let actual = ctx.find_value(key).unwrap_or(GrowthBookAttributeValue::Empty);
            Some(super::operator_condition::value_in_members(&actual, values, false))
        },
        Some(SavedGroup::Condition(condition)) => evaluate(condition, &next),
        // A group type added after this SDK was built. The client's SDK may support it.
        Some(SavedGroup::Unknown) => None,
        _ => Some(false),
    }
}

/// Evaluate a plain (non-operator) condition value against the attribute at `key`.
pub(super) fn implicit_equals(
    key: &str,
    expected: &GrowthBookAttributeValue,
    ctx: &ConditionEvalContext,
) -> Tri {
    non_operator_or_condition(None, &GrowthBookAttribute::new(key.to_owned(), expected.clone()), ctx)
}

fn non_operator_or_condition(
    parent_attribute: Option<&GrowthBookAttribute>,
    feature_attribute: &GrowthBookAttribute,
    ctx: &ConditionEvalContext,
) -> Tri {
    match &feature_attribute.value {
        GrowthBookAttributeValue::String(_) => Some(string_non_operator(parent_attribute, feature_attribute, ctx)),
        GrowthBookAttributeValue::Array(feature_values) => Some(array(&parent_attribute, &feature_attribute, ctx, feature_values)),
        GrowthBookAttributeValue::Object(it) => object(parent_attribute, feature_attribute, ctx, it),
        GrowthBookAttributeValue::Empty => Some(empty(&parent_attribute, &feature_attribute, ctx)),
        it => Some(fallback(&parent_attribute, feature_attribute, ctx, it)),
    }
}

fn string_non_operator(
    parent_attribute: Option<&GrowthBookAttribute>,
    feature_attribute: &GrowthBookAttribute,
    ctx: &ConditionEvalContext,
) -> bool {
    if feature_attribute.key.starts_with('$') && parent_attribute.is_some() {
        false
    } else {
        OperatorCondition::eq(parent_attribute, feature_attribute, ctx)
    }
}

fn array(
    parent_attribute: &Option<&GrowthBookAttribute>,
    feature_attribute: &&GrowthBookAttribute,
    ctx: &ConditionEvalContext,
    feature_values: &[GrowthBookAttributeValue],
) -> bool {
    if let Some(GrowthBookAttributeValue::Array(user_values)) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
        if feature_values.len() == user_values.len() {
            feature_values.iter().zip(&user_values).all(|(value, user_value)| value.json_eq(user_value))
        } else {
            false
        }
    } else {
        false
    }
}

fn object(
    parent_attribute: Option<&GrowthBookAttribute>,
    feature_attribute: &GrowthBookAttribute,
    ctx: &ConditionEvalContext,
    it: &[GrowthBookAttribute],
) -> Tri {
    if is_operator_object(it) {
        let parent = feature_attribute.aggregate_key(parent_attribute);
        all_of(it.iter().map(|next| verify(Some(&parent), next, ctx)))
    } else {
        // A non-operator object condition is a deep-equality check, so it
        // matches only an object attribute with the same contents.
        match ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
            Some(user_value @ GrowthBookAttributeValue::Object(_)) => Some(user_value.json_eq(&feature_attribute.value)),
            _ => Some(false),
        }
    }
}

fn empty(
    parent_attribute: &Option<&GrowthBookAttribute>,
    feature_attribute: &&GrowthBookAttribute,
    ctx: &ConditionEvalContext,
) -> bool {
    if let Some(it) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
        it == GrowthBookAttributeValue::Empty
    } else {
        true
    }
}

fn fallback(
    parent_attribute: &Option<&GrowthBookAttribute>,
    feature_attribute: &GrowthBookAttribute,
    ctx: &ConditionEvalContext,
    it: &GrowthBookAttributeValue,
) -> bool {
    // Numbers and booleans: JS `===`, so no coercion from other types.
    if let Some(user_value) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
        it.strict_eq(&user_value)
    } else {
        false
    }
}

impl GrowthBookAttribute {
    fn aggregate_key(
        &self,
        parent_attribute: Option<&GrowthBookAttribute>,
    ) -> Self {
        let key = parent_attribute.map(|parent| format!("{}.{}", parent.key, self.key)).unwrap_or(self.key.clone());
        GrowthBookAttribute { key, value: self.value.clone() }
    }
}

#[cfg(test)]
mod test {
    use serde::Deserialize;
    use serde_json::Value;

    use crate::condition::eval_context::{saved_groups_from_value, ConditionEvalContext};
    use crate::condition::use_case::ConditionsMatchesAttributes;
    use crate::model_public::GrowthBookAttribute;

    #[tokio::test]
    async fn evaluate_conditions() -> Result<(), Box<dyn std::error::Error>> {
        let cases = Cases::new();

        for value in cases.eval_condition {
            let eval_condition = EvalCondition::new(value);

            let vec_condition = &GrowthBookAttribute::from(eval_condition.condition).expect("Failed to create attributes");
            let vec_attributes = GrowthBookAttribute::from(eval_condition.attribute).expect("Failed to create attributes");
            let saved_groups = saved_groups_from_value(eval_condition.saved_groups.as_ref());
            let enabled = vec_condition.matches(&ConditionEvalContext::new(&vec_attributes, &saved_groups));
            if enabled != eval_condition.result {
                panic!("EvalCondition failed: {}", eval_condition.name)
            }
        }

        Ok(())
    }

    #[derive(Deserialize, Clone)]
    #[serde(rename_all = "camelCase")]
    struct Cases {
        eval_condition: Vec<Value>,
    }

    pub struct EvalCondition {
        name: String,
        condition: Value,
        attribute: Value,
        result: bool,
        saved_groups: Option<Value>,
    }

    impl EvalCondition {
        fn new(value: Value) -> Self {
            let array = value.as_array().expect("Failed to convert to array");
            Self {
                name: array[0].as_str().expect("Failed to convert do str").to_string(),
                condition: array[1].clone(),
                attribute: array[2].clone(),
                result: array[3].as_bool().expect("Failed to convert to bool"),
                // Optional 5th element: the savedGroups map for $inGroup/$notInGroup.
                saved_groups: array.get(4).cloned(),
            }
        }
    }

    impl Cases {
        pub fn new() -> Self {
            serde_json::from_value(crate::corpus::active()).expect("Failed to create cases")
        }
    }
}
