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
        !ctx.work_limit_exceeded() && self.iter().all(|it| verify(None, it, ctx, false)) && !ctx.work_limit_exceeded()
    }
}

fn verify(
    parent_attribute: Option<&GrowthBookAttribute>,
    feature_attribute: &GrowthBookAttribute,
    ctx: &ConditionEvalContext,
    array_size: bool,
) -> bool {
    if ctx.work_limit_exceeded() {
        return false;
    }
    match feature_attribute.key.as_str() {
        "$savedGroup" if parent_attribute.is_none() => saved_group(&feature_attribute.value, ctx),
        "$savedGroup" | "$savedGroups" => false,
        "$and" | "$or" | "$nor" if parent_attribute.is_some() => false,
        "$not" => OperatorCondition::not(parent_attribute, feature_attribute, ctx, verify),
        "$ne" => OperatorCondition::ne(parent_attribute, feature_attribute, ctx, verify),
        "$and" => OperatorCondition::and(parent_attribute, feature_attribute, ctx, verify),
        "$nor" => OperatorCondition::nor(parent_attribute, feature_attribute, ctx, verify),
        "$or" => OperatorCondition::or(parent_attribute, feature_attribute, ctx, verify),
        "$in" => OperatorCondition::is_in(parent_attribute, feature_attribute, ctx, false, verify),
        "$nin" => OperatorCondition::nin(parent_attribute, feature_attribute, ctx, false, verify),
        "$gt" => OrderComparison::gt(parent_attribute, feature_attribute, ctx, array_size),
        "$gte" => OrderComparison::gte(parent_attribute, feature_attribute, ctx, array_size),
        "$lt" => OrderComparison::lt(parent_attribute, feature_attribute, ctx, array_size),
        "$lte" => OrderComparison::lte(parent_attribute, feature_attribute, ctx, array_size),
        "$eq" => OperatorCondition::eq(parent_attribute, feature_attribute, ctx, verify),
        "$exists" => OperatorCondition::exists(parent_attribute, feature_attribute, ctx, verify),
        "$regex" => RegexComparison::matches(parent_attribute, feature_attribute, ctx),
        "$type" => TypeComparison::matches(parent_attribute, feature_attribute, ctx),
        "$size" => SizeComparison::matches(parent_attribute, feature_attribute, ctx),
        "$all" => OperatorCondition::all(parent_attribute, feature_attribute, ctx, false),
        "$vgt" => VersionComparison::vgt(parent_attribute, feature_attribute, ctx),
        "$vgte" => VersionComparison::vgte(parent_attribute, feature_attribute, ctx),
        "$vlt" => VersionComparison::vlt(parent_attribute, feature_attribute, ctx),
        "$vlte" => VersionComparison::vlte(parent_attribute, feature_attribute, ctx),
        "$veq" => VersionComparison::veq(parent_attribute, feature_attribute, ctx),
        "$vne" => VersionComparison::vne(parent_attribute, feature_attribute, ctx),
        "$elemMatch" => ElemMatchComparison::matches(parent_attribute, feature_attribute, ctx),
        "$ini" => OperatorCondition::is_in(parent_attribute, feature_attribute, ctx, true, verify),
        "$nini" => OperatorCondition::nin(parent_attribute, feature_attribute, ctx, true, verify),
        "$alli" => OperatorCondition::all(parent_attribute, feature_attribute, ctx, true),
        "$regexi" => RegexComparison::matches_ignore_case(parent_attribute, feature_attribute, ctx),
        "$notRegex" => RegexComparison::not_matches(parent_attribute, feature_attribute, ctx),
        "$notRegexi" => RegexComparison::not_matches_ignore_case(parent_attribute, feature_attribute, ctx),
        "$inGroup" => OperatorCondition::in_group(parent_attribute, feature_attribute, ctx),
        "$notInGroup" => OperatorCondition::not_in_group(parent_attribute, feature_attribute, ctx),
        key if key.starts_with('$') && parent_attribute.is_some() => false,
        _ => non_operator_or_condition(parent_attribute, feature_attribute, ctx),
    }
}

/// Whether an object represents attribute operators rather than a condition
/// evaluated against the fields of an object.
pub(super) fn is_operator_object(fields: &[GrowthBookAttribute]) -> bool {
    !fields.is_empty() && fields.iter().all(|field| field.key.starts_with('$'))
}

/// Evaluate an array element or size without resetting group cycle detection.
pub(super) fn matches_value(
    actual: &GrowthBookAttributeValue,
    expected: &GrowthBookAttributeValue,
    ctx: &ConditionEvalContext,
    case_insensitive: bool,
) -> bool {
    if let GrowthBookAttributeValue::Object(fields) = expected {
        if is_operator_object(fields) {
            let attributes = [GrowthBookAttribute::new(String::from("value"), actual.clone())];
            let nested = ctx.with_attributes(&attributes);
            return fields.iter().all(|field| verify(Some(&attributes[0]), field, &nested, false));
        }
    }
    if case_insensitive {
        if let (GrowthBookAttributeValue::String(actual), GrowthBookAttributeValue::String(expected)) = (actual, expected) {
            return actual.to_lowercase() == expected.to_lowercase();
        }
    }
    actual.strict_eq(expected)
}

/// Resolve a condition-level reference. Unknown reference fields are ignored.
fn saved_group(
    reference: &GrowthBookAttributeValue,
    ctx: &ConditionEvalContext,
) -> bool {
    let GrowthBookAttributeValue::Object(fields) = reference else {
        return false;
    };
    let field = |key: &str| fields.iter().find(|field| field.key == key).map(|field| &field.value);
    let Some(GrowthBookAttributeValue::String(id)) = field("id") else {
        return false;
    };
    let attribute_key = match field("attributeKey") {
        None => None,
        Some(GrowthBookAttributeValue::String(key)) => Some(key),
        _ => return false,
    };
    let Some(next) = ctx.enter_group(id) else {
        return false;
    };
    match ctx.saved_group(id) {
        Some(SavedGroup::List {
            attribute_key: entry_key,
            values: Some(values),
        }) => {
            let Some(key) = attribute_key.or(entry_key.as_ref()) else {
                return false;
            };
            let actual = ctx.find_value(key).unwrap_or(GrowthBookAttributeValue::Empty);
            super::operator_condition::value_in_members(&actual, values, false)
        },
        Some(SavedGroup::Condition(condition)) => condition.matches(&next),
        _ => false,
    }
}

fn non_operator_or_condition(
    parent_attribute: Option<&GrowthBookAttribute>,
    feature_attribute: &GrowthBookAttribute,
    ctx: &ConditionEvalContext,
) -> bool {
    match &feature_attribute.value {
        GrowthBookAttributeValue::String(_) => string_non_operator(parent_attribute, feature_attribute, ctx),
        GrowthBookAttributeValue::Array(feature_values) => array(&parent_attribute, &feature_attribute, ctx, feature_values),
        GrowthBookAttributeValue::Object(it) => object(parent_attribute, feature_attribute, ctx, it),
        GrowthBookAttributeValue::Empty => empty(&parent_attribute, &feature_attribute, ctx),
        it => fallback(&parent_attribute, feature_attribute, ctx, it),
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
        OperatorCondition::eq(parent_attribute, feature_attribute, ctx, verify)
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
            feature_values.iter().enumerate().all(|(index, value)| value == &user_values[index])
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
) -> bool {
    if it.is_empty() {
        // A non-operator condition `{key: {}}` is a deep-equality check, so it
        // matches only when the user's value at `key` is itself an empty object
        // `{}`. A missing attribute (or any non-object / non-empty value) does
        // not match.
        matches!(
            ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key),
            Some(GrowthBookAttributeValue::Object(user_object)) if user_object.is_empty()
        )
    } else {
        it.iter().all(|next| {
            let parent = feature_attribute.aggregate_key(parent_attribute);
            verify(Some(&parent), next, ctx, false)
        })
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
    if let Some(user_value) = ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key) {
        it == &user_value
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
