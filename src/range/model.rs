pub struct Range {
    pub start: f32,
    pub end: f32,
}

impl Range {
    pub fn get_range(range: Option<Vec<f32>>) -> Option<Range> {
        if let Some(range) = range {
            if range.len() == 2 {
                Some(Range { start: range[0], end: range[1] })
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_bucket_range(
        variations: i64,
        coverage: &Option<f32>,
        weights: Option<Vec<f32>>,
    ) -> Vec<Self> {
        let clamped_coverage = clamped_coverage(*coverage);
        let adjusted_weights = adjusted_weights(variations, weights.clone());

        let mut acc = 0.0;

        adjusted_weights
            .iter()
            .map(|weight| {
                let start = acc;
                acc += weight;
                let end = format!("{:.4}", start + clamped_coverage * weight).parse::<f32>().expect("Failed to parse to formated f32");
                Self { start, end }
            })
            .collect()
    }

    pub fn in_range(
        &self,
        value: &f32,
    ) -> bool {
        value >= &self.start && value < &self.end
    }
}

fn clamped_coverage(coverage: Option<f32>) -> f32 {
    match coverage {
        None => 1.0,
        Some(value) => value.clamp(0.0, 1.0),
    }
}

fn adjusted_weights(
    variations: i64,
    weights: Option<Vec<f32>>,
) -> Vec<f32> {
    if let Some(found_weights) = weights {
        if variations == found_weights.len() as i64 {
            let weights_sum: f32 = found_weights.iter().sum();
            if (0.99..=1.01).contains(&weights_sum) {
                return found_weights.clone();
            }
        }
    }

    get_equal_weights(variations)
}

fn get_equal_weights(variations: i64) -> Vec<f32> {
    let weight = 1.0 / variations as f32;
    let mut vec = vec![];
    for _ in 0..variations {
        vec.push(weight);
    }
    vec
}

#[cfg(test)]
mod test {
    use std::fs;

    use serde::Deserialize;
    use serde_json::Value;

    use crate::range::model::{get_equal_weights, Range};

    #[tokio::test]
    async fn evaluate_get_bucket_range() -> Result<(), Box<dyn std::error::Error>> {
        let cases = Cases::new();

        for value in cases.get_bucket_range {
            let eval_get_bucket_range = EvalGetBucketRange::new(value);
            let result = Range::get_bucket_range(eval_get_bucket_range.variations, &eval_get_bucket_range.coverage, eval_get_bucket_range.weights);
            for (index, range) in result.iter().enumerate() {
                let (expected_start, expected_end) = eval_get_bucket_range.result[index];
                if expected_start != range.start || expected_end != range.end {
                    panic!("EvalGetBucketRange failed: name='{}' start={} end={}", eval_get_bucket_range.name, range.start, range.end)
                }
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn evaluate_get_equal_weights() -> Result<(), Box<dyn std::error::Error>> {
        let cases = Cases::new();

        for value in cases.get_equal_weights {
            let eval_get_equal_weights = EvalGetEqualWeights::new(value);
            let result = get_equal_weights(eval_get_equal_weights.variations);
            for (index, weight) in result.iter().enumerate() {
                let expected_weight = eval_get_equal_weights.weights[index];
                if expected_weight.ne(weight) {
                    panic!("EvalGetEqualWeights failed: expected_weight={expected_weight} weight={weight}")
                }
            }
        }
        Ok(())
    }

    #[derive(Deserialize, Clone)]
    #[serde(rename_all = "camelCase")]
    struct Cases {
        get_bucket_range: Vec<Value>,
        get_equal_weights: Vec<Value>,
    }

    pub struct EvalGetBucketRange {
        name: String,
        variations: i64,
        coverage: Option<f32>,
        weights: Option<Vec<f32>>,
        result: Vec<(f32, f32)>,
    }

    impl EvalGetBucketRange {
        fn new(value: Value) -> Self {
            let array = value.as_array().expect("Failed to convert to array");
            Self {
                name: array[0].as_str().expect("Failed to convert do str").to_string(),
                variations: array[1].as_array().expect("Failed to convert to array")[0].as_i64().expect("Failed to convert to i64"),
                coverage: array[1].as_array().expect("Failed to convert to array")[1].as_f64().map(|it| it as f32),
                weights: array[1].as_array().expect("Failed to convert to array")[2]
                    .as_array()
                    .map(|array| array.iter().map(|it| it.as_f64().expect("Failed to convert to f64") as f32).collect()),
                result: array[2]
                    .as_array()
                    .expect("Failed to convert to array")
                    .iter()
                    .map(|it| {
                        let array = it.as_array().expect("Failed to convert to array [2]");
                        (
                            array[0].as_f64().expect("Failed to converto to f64") as f32,
                            array[1].as_f64().expect("Failed to converto to f64 [2]") as f32,
                        )
                    })
                    .collect(),
            }
        }
    }

    pub struct EvalGetEqualWeights {
        variations: i64,
        weights: Vec<f32>,
    }

    impl EvalGetEqualWeights {
        fn new(value: Value) -> Self {
            let array = value.as_array().expect("Failed to convert to array");
            Self {
                variations: array[0].as_i64().expect("Failed to convert to i64"),
                weights: array[1]
                    .as_array()
                    .expect("Failed to convert to array")
                    .iter()
                    .map(|it| it.as_f64().expect("Failed to convert to f64") as f32)
                    .collect(),
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
