use crate::hash::{HashCode, HashCodeVersion};
use crate::model_public::GrowthBookAttributeValue;
use crate::range::model::Range;

pub struct Namespace;

impl Namespace {
    pub fn is_in(
        user_value: &GrowthBookAttributeValue,
        namespace: &str,
        range: &Range,
    ) -> bool {
        let user_weight = HashCode::hash_code(&format!("{}__", user_value), namespace, HashCodeVersion::from(1)).unwrap_or(-1.0);
        range.in_range(&user_weight)
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use serde::Deserialize;
    use serde_json::Value;

    use crate::model_public::GrowthBookAttributeValue;
    use crate::namespace::use_case::Namespace;
    use crate::range::model::Range;

    #[tokio::test]
    async fn evaluate_get_bucket_range() -> Result<(), Box<dyn std::error::Error>> {
        let cases = Cases::new();

        for value in cases.in_namespace {
            let eval_in_namespace = EvalInNamespace::new(value);
            let range = Range {
                start: eval_in_namespace.range_start,
                end: eval_in_namespace.range_end,
            };
            let result = Namespace::is_in(&GrowthBookAttributeValue::String(eval_in_namespace.user_value.clone()), &eval_in_namespace.namespace_name, &range);
            if result != eval_in_namespace.result {
                panic!(
                    "EvalInNamespace failed: name='{}' expected_result={} result={result}",
                    eval_in_namespace.name, eval_in_namespace.user_value
                )
            }
        }

        Ok(())
    }

    #[derive(Deserialize, Clone)]
    #[serde(rename_all = "camelCase")]
    struct Cases {
        in_namespace: Vec<Value>,
    }

    pub struct EvalInNamespace {
        name: String,
        user_value: String,
        namespace_name: String,
        range_start: f32,
        range_end: f32,
        result: bool,
    }

    impl EvalInNamespace {
        fn new(value: Value) -> Self {
            let array = value.as_array().expect("Failed to convert to array");
            Self {
                name: array[0].as_str().expect("Failed to convert do str").to_string(),
                user_value: array[1].as_str().expect("Failed to convert do str").to_string(),
                namespace_name: array[2].as_array().expect("Failed to convert to array")[0].as_str().expect("Failed to convert to str").to_string(),
                range_start: array[2].as_array().expect("Failed to convert to array")[1].as_f64().expect("Failed to convert to str") as f32,
                range_end: array[2].as_array().expect("Failed to convert to array")[2].as_f64().expect("Failed to convert to str") as f32,
                result: array[3].as_bool().expect("Failed to convert do bool"),
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
