use crate::condition::elem_match_comparison::ElemMatchComparison;
use crate::condition::eval_context::ConditionEvalContext;
use crate::condition::operator_condition::OperatorCondition;
use crate::condition::order_comparison::OrderComparison;
use crate::condition::regex_comparison::RegexComparison;
use crate::condition::size_comparison::SizeComparison;
use crate::condition::type_comparison::TypeComparison;
use crate::condition::version_comparison::VersionComparison;
use crate::extensions::FindGrowthBookAttribute;
use crate::model_public::{GrowthBookAttribute, GrowthBookAttributeValue};

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
        self.iter().all(|it| verify(None, it, ctx, false))
    }
}

fn verify(
    parent_attribute: Option<&GrowthBookAttribute>,
    feature_attribute: &GrowthBookAttribute,
    ctx: &ConditionEvalContext,
    array_size: bool,
) -> bool {
    match feature_attribute.key.as_str() {
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
        "$size" => SizeComparison::matches(parent_attribute, feature_attribute, ctx, verify),
        "$all" => OperatorCondition::all(parent_attribute, feature_attribute, ctx, false, verify),
        "$vgt" => VersionComparison::vgt(parent_attribute, feature_attribute, ctx),
        "$vgte" => VersionComparison::vgte(parent_attribute, feature_attribute, ctx),
        "$vlt" => VersionComparison::vlt(parent_attribute, feature_attribute, ctx),
        "$vlte" => VersionComparison::vlte(parent_attribute, feature_attribute, ctx),
        "$veq" => VersionComparison::veq(parent_attribute, feature_attribute, ctx),
        "$vne" => VersionComparison::vne(parent_attribute, feature_attribute, ctx),
        "$elemMatch" => ElemMatchComparison::matches(parent_attribute, feature_attribute, ctx, array_size, verify),
        "$ini" => OperatorCondition::is_in(parent_attribute, feature_attribute, ctx, true, verify),
        "$nini" => OperatorCondition::nin(parent_attribute, feature_attribute, ctx, true, verify),
        "$alli" => OperatorCondition::all(parent_attribute, feature_attribute, ctx, true, verify),
        "$regexi" => RegexComparison::matches_ignore_case(parent_attribute, feature_attribute, ctx),
        "$notRegex" => RegexComparison::not_matches(parent_attribute, feature_attribute, ctx),
        "$notRegexi" => RegexComparison::not_matches_ignore_case(parent_attribute, feature_attribute, ctx),
        "$inGroup" => OperatorCondition::in_group(parent_attribute, feature_attribute, ctx),
        "$notInGroup" => OperatorCondition::not_in_group(parent_attribute, feature_attribute, ctx),
        _ => non_operator_or_condition(parent_attribute, feature_attribute, ctx),
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
    if feature_attribute.key.starts_with('$') {
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
        ctx.find_value(&parent_attribute.unwrap_or(feature_attribute).key).is_none()
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
    use std::fs;

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
            let contents = fs::read_to_string("./tests/all_cases.json").expect("Should have been able to read the file");

            serde_json::from_str(&contents).expect("Failed to create cases")
        }
    }
}
