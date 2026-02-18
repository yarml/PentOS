use {serde::Deserialize, serde_json::Value};

#[derive(Deserialize)]
pub struct ChefConfig {
    #[serde(rename = "ovmf-version")]
    pub ovmf_version: String,
    #[serde(rename = "ovmf-source-template")]
    pub ovmf_source_template: String,
    #[serde(rename = "ovmf-varsfd-path-template")]
    pub ovmf_varsfd_path_template: String,
    #[serde(rename = "ovmf-codefd-path-template")]
    pub ovmf_codefd_path_template: String,
}

impl ChefConfig {
    pub fn from(value: &Value) -> Self {
        serde_json::from_value(value.clone()).unwrap()
    }
}
