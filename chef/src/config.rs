use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct ChefConfig {
    #[serde(rename = "ovmf-source")]
    pub ovmf_source: String,
    #[serde(rename = "install-bootloader")]
    pub install_bootloader: String,
    #[serde(rename = "install-kernel")]
    pub install_kernel: String,
}

impl ChefConfig {
    pub fn from(value: &Value) -> Self {
        serde_json::from_value(value.clone()).unwrap()
    }
}
