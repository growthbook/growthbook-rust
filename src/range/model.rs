pub struct Range {
    pub start: f64,
    pub end: f64,
}

impl Range {
    pub fn get_range(range: Option<Vec<f64>>) -> Option<Range> {
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
        coverage: &Option<f64>,
        weights: Option<Vec<f64>>,
    ) -> Vec<Self> {
        let clamped_coverage = clamped_coverage(*coverage);
        let adjusted_weights = adjusted_weights(variations, weights.clone());

        let mut acc = 0.0;

        adjusted_weights
            .iter()
            .map(|weight| {
                let start = acc;
                acc += weight;
                let end = start + clamped_coverage * weight;
                Self { start, end }
            })
            .collect()
    }

    pub fn in_range(
        &self,
        value: &f64,
    ) -> bool {
        value >= &self.start && value < &self.end
    }
}

fn clamped_coverage(coverage: Option<f64>) -> f64 {
    match coverage {
        None => 1.0,
        Some(value) => value.clamp(0.0, 1.0),
    }
}

fn adjusted_weights(
    variations: i64,
    weights: Option<Vec<f64>>,
) -> Vec<f64> {
    if let Some(found_weights) = weights {
        if variations == found_weights.len() as i64 {
            let weights_sum: f64 = found_weights.iter().sum();
            if (0.99..=1.01).contains(&weights_sum) {
                return found_weights.clone();
            }
        }
    }

    get_equal_weights(variations)
}

fn get_equal_weights(variations: i64) -> Vec<f64> {
    let weight = 1.0 / variations as f64;
    let mut vec = vec![];
    for _ in 0..variations {
        vec.push(weight);
    }
    vec
}

#[cfg(test)]
mod test {
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
                // The corpus stores rounded ranges; JS compares both sides floored to 8 decimals.
                if round(expected_start) != round(range.start) || round(expected_end) != round(range.end) {
                    panic!("EvalGetBucketRange failed: name='{}' start={} end={}", eval_get_bucket_range.name, range.start, range.end)
                }
            }
        }
        Ok(())
    }

    #[test]
    fn get_bucket_range_matches_js_exactly() {
        // Expected ends are JS `String(n)` values: serde_json's default float parser can be off by one ULP for 17-digit decimals.
        let cases: Value = serde_json::from_str(include_str!("../../tests/fixtures/js_bucket_ranges.json")).expect("Failed to parse JS ranges");
        for case in cases.as_array().expect("Failed to convert to array") {
            let weights = case["weights"].as_array().map(|array| array.iter().map(|it| it.as_f64().expect("Failed to convert to f64")).collect());
            let result = Range::get_bucket_range(case["numVariations"].as_i64().expect("Failed to convert to i64"), &case["coverage"].as_f64(), weights);
            let expected = case["ranges"].as_array().expect("Failed to convert to array");
            assert_eq!(result.len(), expected.len(), "{case}");
            for (range, expected) in result.iter().zip(expected) {
                assert_eq!((range.start, range.end), (parse(&expected[0]), parse(&expected[1])), "{case}");
            }
        }
    }

    fn parse(value: &Value) -> f64 {
        value.as_str().expect("Failed to convert to str").parse().expect("Failed to parse f64")
    }

    fn round(value: f64) -> f64 {
        (value * 1e8).floor() / 1e8
    }

    #[tokio::test]
    async fn evaluate_get_equal_weights() -> Result<(), Box<dyn std::error::Error>> {
        let cases = Cases::new();

        for value in cases.get_equal_weights {
            let eval_get_equal_weights = EvalGetEqualWeights::new(value);
            let result = get_equal_weights(eval_get_equal_weights.variations);
            for (index, weight) in result.iter().enumerate() {
                let expected_weight = eval_get_equal_weights.weights[index];
                if round(expected_weight).ne(&round(*weight)) {
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
        coverage: Option<f64>,
        weights: Option<Vec<f64>>,
        result: Vec<(f64, f64)>,
    }

    impl EvalGetBucketRange {
        fn new(value: Value) -> Self {
            let array = value.as_array().expect("Failed to convert to array");
            Self {
                name: array[0].as_str().expect("Failed to convert do str").to_string(),
                variations: array[1].as_array().expect("Failed to convert to array")[0].as_i64().expect("Failed to convert to i64"),
                coverage: array[1].as_array().expect("Failed to convert to array")[1].as_f64(),
                weights: array[1].as_array().expect("Failed to convert to array")[2]
                    .as_array()
                    .map(|array| array.iter().map(|it| it.as_f64().expect("Failed to convert to f64")).collect()),
                result: array[2]
                    .as_array()
                    .expect("Failed to convert to array")
                    .iter()
                    .map(|it| {
                        let array = it.as_array().expect("Failed to convert to array [2]");
                        (array[0].as_f64().expect("Failed to converto to f64"), array[1].as_f64().expect("Failed to converto to f64 [2]"))
                    })
                    .collect(),
            }
        }
    }

    pub struct EvalGetEqualWeights {
        variations: i64,
        weights: Vec<f64>,
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
                    .map(|it| it.as_f64().expect("Failed to convert to f64"))
                    .collect(),
            }
        }
    }

    impl Cases {
        pub fn new() -> Self {
            serde_json::from_value(crate::corpus::active()).expect("Failed to create cases")
        }
    }
}
