use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IAMPolicy {
    pub version: String,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statement {
    pub sid: Option<String>,
    pub effect: String,
    pub actions: Vec<String>,
    pub resources: Vec<String>,
    pub conditions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketPolicy {
    pub version: String,
    pub statements: Vec<Statement>,
}

impl Default for IAMPolicy {
    fn default() -> Self {
        Self {
            version: "2012-10-17".to_string(),
            statements: Vec::new(),
        }
    }
}
