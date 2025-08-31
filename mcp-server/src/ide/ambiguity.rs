use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub struct AmbiguityError {
    input: Value,
    alternatives: Vec<Value>,
}

impl AmbiguityError {
    #[expect(dead_code)]
    pub fn new(input: Value, alternatives: Vec<Value>) -> Self {
        Self {
            input,
            alternatives,
        }
    }
}

impl std::fmt::Display for AmbiguityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Ambiguous operation: to clarify meaning, replace `{}` with one of the following:",
            self.input
        )?;
        for alternative in &self.alternatives {
            write!(f, "\n  - {}", alternative)?;
        }
        Ok(())
    }
}
