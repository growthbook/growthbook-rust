use std::hash::Hasher;

use hashers::fnv::FNV1aHasher32;

pub enum HashCodeVersion {
    V1,
    V2,
    Invalid,
}

impl From<i64> for HashCodeVersion {
    fn from(value: i64) -> Self {
        match value {
            1 => Self::V1,
            2 => Self::V2,
            _ => Self::Invalid,
        }
    }
}

impl From<Option<i64>> for HashCodeVersion {
    fn from(option: Option<i64>) -> Self {
        match option {
            None => Self::V1,
            Some(value) => Self::from(value),
        }
    }
}

pub struct HashCode;

impl HashCode {
    pub fn hash_code(
        input: &str,
        seed: &str,
        version: HashCodeVersion,
    ) -> Option<f32> {
        match version {
            HashCodeVersion::V1 => Some(Self::hash_v1(input, seed)),
            HashCodeVersion::V2 => Some(Self::hash_v2(input, seed)),
            _ => None,
        }
    }

    fn hash_v1(
        input: &str,
        seed: &str,
    ) -> f32 {
        let concatenated = format!("{}{}", input, seed);
        let hash_value = Self::fnv1a_32(concatenated.as_bytes());
        let remainder = hash_value % 1000;
        remainder as f32 / 1000.0
    }

    fn hash_v2(
        input: &str,
        seed: &str,
    ) -> f32 {
        let concatenated = format!("{}{}", seed, input);
        let first = Self::fnv1a_32(concatenated.as_bytes());

        let first_as_string = format!("{}", first);
        let second = Self::fnv1a_32(first_as_string.as_bytes());

        let remainder = (second as u64) % 10000;

        remainder as f32 / 10000.0
    }

    fn fnv1a_32(data: &[u8]) -> u32 {
        let mut hasher = FNV1aHasher32::default();
        hasher.write(data);
        hasher.finish() as u32
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use serde::Deserialize;
    use serde_json::Value;

    use crate::hash::{HashCode, HashCodeVersion};

    #[tokio::test]
    async fn evaluate_hashes() -> Result<(), Box<dyn std::error::Error>> {
        let cases = Cases::new();

        for value in cases.hash {
            let eval_hash = EvalHash::new(value);
            let result = HashCode::hash_code(&eval_hash.value, &eval_hash.seed, HashCodeVersion::from(eval_hash.version));
            if result != eval_hash.result {
                panic!(
                    "EvalHash failed: value={} seed={} version={} expected={:?} result={:?}",
                    eval_hash.value, eval_hash.seed, eval_hash.version, eval_hash.result, result
                )
            }
        }

        Ok(())
    }

    #[derive(Deserialize, Clone)]
    #[serde(rename_all = "camelCase")]
    struct Cases {
        hash: Vec<Value>,
    }

    #[derive(Deserialize, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct EvalHash {
        seed: String,
        value: String,
        version: i64,
        result: Option<f32>,
    }

    impl EvalHash {
        fn new(value: Value) -> Self {
            let array = value.as_array().expect("Failed to convert to array");
            Self {
                seed: array[0].as_str().expect("Failed to convert do str").to_string(),
                value: array[1].as_str().expect("Failed to convert do str").to_string(),
                version: array[2].as_i64().expect("Failed to convert to i64"),
                result: array[3].as_f64().map(|it| it as f32),
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
