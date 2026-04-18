use std::{fs, sync::LazyLock};

use serde::Deserialize;

use crate::result::ResultExt;

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

    #[serde(rename = "font-version")]
    pub font_version: String,
    #[serde(rename = "font-source-template")]
    pub font_source_template: String,
    #[serde(rename = "font-path-template")]
    pub font_path_template: String,

    #[serde(rename = "img-disk-size-mb")]
    pub img_disk_size_mb: usize,
    #[serde(rename = "img-part-boot-size-mb")]
    pub img_part_boot_size_mb: usize,
    #[serde(rename = "img-part-system-size-mb")]
    pub img_part_system_size_mb: usize,
}

pub static CONFIG: LazyLock<ChefConfig> = LazyLock::new(|| {
    let raw_config = fs::read_to_string("build.toml").or_fatal("build.toml");
    toml::from_str(&raw_config).or_fatal("build.toml")
});
