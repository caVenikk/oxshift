use std::env::{self, VarError};

pub fn var(suffix: &str) -> Result<String, VarError> {
    env::var(format!("OXSHIFT_{suffix}"))
}
