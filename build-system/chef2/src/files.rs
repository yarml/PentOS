use {
    crate::{result::ResultExt, status::Status},
    flate2::bufread::GzDecoder,
    std::{
        fs::{self, File},
        io::{self, Read},
        path::Path,
    },
    xz::bufread::XzDecoder,
};

pub fn write<R: ?Sized + Read>(dst: &Path, reader: &mut R) {
    Status::doing("Writing", format!("{dst:?}"));
    if let Some(dst_parent) = dst.parent() {
        fs::create_dir_all(dst_parent).or_fatal("create dir all");
    }
    let mut file = File::create(dst).or_fatal("file open");
    io::copy(reader, &mut file).or_fatal("file write");
}

pub fn copy(src: &Path, dst: &Path) {
    Status::doing("Copying", format!("{src:?} -> {dst:?}"));
    if let Some(dst_parent) = dst.parent() {
        fs::create_dir_all(dst_parent).or_fatal("create dir all");
    }

    fs::copy(src, dst).or_fatal("copy");
}

pub fn download(url: &str) -> Vec<u8> {
    Status::doing("Downloading", url);
    reqwest::blocking::get(url)
        .or_fatal("get")
        .bytes()
        .or_fatal("bytes")
        .to_vec()
}

pub enum Decompressor {
    Xz,
    Gz,
}

pub fn decompress(name: &str, compressed: &[u8]) -> Vec<u8> {
    Status::doing("Decompressing", name);

    let decompressor = if name.ends_with(".xz") {
        Decompressor::Xz
    } else if name.ends_with(".gz") {
        Decompressor::Gz
    } else {
        Status::error(format!("unknown compression for {name}"));
    };

    let mut decompressed = Vec::new();
    match decompressor {
        Decompressor::Xz => {
            let mut decoder = XzDecoder::new(compressed);
            decoder
                .read_to_end(&mut decompressed)
                .or_fatal("decompress");
        }
        Decompressor::Gz => {
            let mut decoder = GzDecoder::new(compressed);
            decoder
                .read_to_end(&mut decompressed)
                .or_fatal("decompress");
        }
    }
    decompressed
}
