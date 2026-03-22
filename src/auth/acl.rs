use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlPolicy {
    pub owner: CanonicalUser,
    pub grants: Vec<Grant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalUser {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grant {
    pub grantee: Grantee,
    pub permission: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grantee {
    #[serde(rename = "type")]
    pub grantee_type: String,
    pub uri: Option<String>,
    pub id: Option<String>,
}
