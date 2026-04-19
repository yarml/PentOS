use std::{fs, path::Path, sync::LazyLock};

use serde::Deserialize;

use crate::{result::ResultExt, status::Status};

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

    #[serde(rename = "qemu-bin")]
    pub qemu_bin: String,

    #[serde(rename = "qemu-numcores")]
    pub qemu_numcores: String,
    #[serde(rename = "qemu-mem")]
    pub qemu_mem: String,
    #[serde(rename = "qemu-resolution")]
    pub qemu_resolution: String,
}

pub static CONFIG: LazyLock<ChefConfig> = LazyLock::new(|| {
    let raw_base = fs::read_to_string("build.toml").or_fatal("read build.toml");
    let mut base: toml::Value = toml::from_str(&raw_base).or_fatal("parse build.toml");

    if Path::new(".build.toml").exists() {
        let raw_override =
            fs::read_to_string(".build.toml").or_fatal("read .build.toml");
        let ov: toml::Value = toml::from_str(&raw_override).or_fatal("parse .build.toml");

        if let (Some(base_map), Some(ov_map)) = (base.as_table_mut(), ov.as_table()) {
            for (k, v) in ov_map {
                base_map.insert(k.clone(), v.clone());
            }
        }
    }

    base.try_into().or_fatal("parse build.toml")
});

impl ChefConfig {
    pub fn qemu_resolution(&self) -> Resolution {
        Resolution::make(&self.qemu_resolution)
    }
}

pub struct Resolution {
    pub xres: usize,
    pub yres: usize,
    pub vgamem_mb: usize,
}

impl Resolution {
    fn make(raw: &str) -> Self {
        let Some((xres, yres)) = raw.split_once('x') else {
            Status::error("invalid qemu-resolution");
        };

        if xres.chars().any(|c| !c.is_numeric()) || yres.chars().any(|c| !c.is_numeric()) {
            Status::error("invalid qemu-resolution");
        }

        let xres: usize = xres
            .parse()
            .map_err(|_| ())
            .or_fatal("invalid qemu-resolution");
        let yres: usize = yres
            .parse()
            .map_err(|_| ())
            .or_fatal("invalid qemu-resolution");

        let vgamem_mb = (xres * yres * 4).div_ceil(1024 * 1024);

        Self {
            xres,
            yres,
            vgamem_mb,
        }
    }
}
