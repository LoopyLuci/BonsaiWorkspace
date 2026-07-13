use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sdk {
    pub sdk_name: String,
    pub language: String,
    pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SdkTemplate {
    pub template_id: String,
    pub language: String,
}
