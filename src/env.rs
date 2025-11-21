use std::env;

pub struct Environment;

impl Environment {
    pub fn string_or_default(
        env_name: &str,
        default: &str,
    ) -> String {
        env::var(env_name).unwrap_or(String::from(default))
    }

    pub fn u64_or_default(
        env_name: &str,
        default: u64,
    ) -> u64 {
        env::var(env_name).ok().map(|env| env.parse::<u64>().unwrap_or(default)).unwrap_or(default)
    }
}
