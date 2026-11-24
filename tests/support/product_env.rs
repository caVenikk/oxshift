use std::env::{self, VarError};

pub fn var(suffix: &str) -> Result<String, VarError> {
    match env::var(format!("OXSHIFT_{suffix}")) {
        Err(VarError::NotPresent) => env::var(format!("PYOXIDE_{suffix}")),
        value => value,
    }
}
