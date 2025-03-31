use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct ChefConfig {
    #[serde(rename = "ovmf-source")]
    pub ovmf_source: String,
}

impl ChefConfig {
    pub fn from(value: &Value) -> Self {
        serde_json::from_value(value.clone()).unwrap()
    }
}
