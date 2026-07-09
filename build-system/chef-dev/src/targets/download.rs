use {
    crate::paths, chef_core::{
        config::CONFIG,
        files,
        result::ResultExt,
        status::Status,
        target::{
            Target,
            run_policy::{FilesNotExist, RunPolicy},
        },
    }, std::{collections::HashMap, path::PathBuf, str::FromStr}, tar::Archive
};

pub fn ovmf() -> DownloadArchiveTarget {
    let version = &CONFIG.ovmf_version;
    let url = CONFIG.ovmf_source_template.replace("$$", version);
    let varsfd = CONFIG.ovmf_varsfd_path_template.replace("$$", version);
    let codefd = CONFIG.ovmf_codefd_path_template.replace("$$", version);

    let mut install = HashMap::new();
    install.insert(varsfd, paths::varsfd().to_str().unwrap().to_string());
    install.insert(codefd, paths::codefd().to_str().unwrap().to_string());

    DownloadArchiveTarget::new(url, install)
}

pub fn font() -> DownloadArchiveTarget {
    let version = &CONFIG.font_version;
    let url = CONFIG.font_source_template.replace("$$", version);
    let font = CONFIG.font_path_template.replace("$$", version);

    let mut install = HashMap::new();
    install.insert(font, paths::font().to_str().unwrap().to_string());

    DownloadArchiveTarget::new(url, install)
}

pub struct DownloadArchiveTarget {
    install_locations: HashMap<String, String>,
    url: String,
}

impl DownloadArchiveTarget {
    pub fn new(url: String, install_locations: HashMap<String, String>) -> Self {
        Self {
            install_locations,
            url,
        }
    }
}

impl Target for DownloadArchiveTarget {
    fn spec(&self) -> bool {
        let mut installed_files = 0;

        let name = self.url.split('/').next_back().unwrap();
        let raw_archive = files::download(&self.url);
        let decompressed = files::decompress(name, &raw_archive);
        let mut archive = Archive::new(decompressed.as_slice());

        for entry in archive.entries().or_fatal("archive entries") {
            let mut entry = entry.or_fatal("archive entry");
            let path = entry.path().unwrap().to_str().unwrap().to_string();
            if let Some(install_location) = self.install_locations.get(&path) {
                files::write(&PathBuf::from_str(install_location).unwrap(), &mut entry);
                installed_files += 1;
            }
        }

        if installed_files < self.install_locations.len() {
            Status::warning("Installed less files than expected");
        }

        true
    }

    fn run_policy(&self) -> Box<dyn RunPolicy> {
        let mut file_locations = Vec::new();
        for location in self.install_locations.values() {
            file_locations.push(PathBuf::from_str(location).unwrap());
        }
        Box::new(FilesNotExist(file_locations))
    }

    fn dependencies(&self) -> Vec<std::rc::Rc<dyn Target>> {
        vec![]
    }
}
