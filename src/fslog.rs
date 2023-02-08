use std::{io::Result, path::Path};

pub struct SimpleFSMeta {
    pub is_file: bool,
    pub is_directory: bool,
    pub is_symlink: bool,
}
#[allow(unused_variables)]
fn log_access<P: AsRef<Path>>(path: P, operation: &str) {
    #[cfg(debug_assertions)]
    {
        use log::info;
        info!("[FS_LOG] Operation={} on file {}", operation, path.as_ref().display());
    }
}
#[allow(unused_variables)]
fn log_copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q, operation: &str) {
    #[cfg(debug_assertions)]
    {
        use log::info;
        info!("[FS_LOG] Operation={} from {} to {}", operation, from.as_ref().display(), to.as_ref().display());
    }
}
pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<()> {
    log_access(&path, "write");
    return std::fs::write(path, contents);
}
pub fn remove_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
    log_access(&path, "remove_dir_all");
    return std::fs::remove_dir_all(path);
}
pub fn create_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
    log_access(&path, "create_dir_all");
    return std::fs::create_dir_all(path);
}
pub fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String> {
    log_access(&path, "read_to_string");
    return std::fs::read_to_string(path);
}
pub fn remove_file<P: AsRef<Path>>(path: P) -> Result<()> {
    log_access(&path, "remove_file");
    return std::fs::remove_file(path);
}
pub fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<u64> {
    log_copy(&from, &to, "copy");
    return std::fs::copy(from, to);
}
pub fn copy_dir<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> std::result::Result<u64, fs_extra::error::Error> {
    log_copy(&from, &to, "copy_dir");
    let options =
        fs_extra::dir::CopyOptions { overwrite: true, copy_inside: true, content_only: true, ..Default::default() };
    return fs_extra::dir::copy(from, to, &options);
}
pub fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<()> {
    log_copy(&from, &to, "rename");
    return std::fs::rename(from, to);
}
pub fn exists<P: AsRef<Path>>(path: P) -> bool {
    log_access(&path, "exists");
    return path.as_ref().exists();
}
pub fn file_open<P: AsRef<Path>>(path: P) -> std::io::Result<std::fs::File> {
    log_access(&path, "open");
    return std::fs::File::open(path);
}
pub fn get_simple_fs_meta_symlink<P: AsRef<Path>>(file: P) -> Option<SimpleFSMeta> {
    let data = std::fs::symlink_metadata(file);
    if data.is_err() {
        return None;
    } else {
        let data_uw = data.unwrap();
        return Some(SimpleFSMeta {
            is_directory: data_uw.is_dir(),
            is_file: data_uw.is_file(),
            is_symlink: data_uw.is_symlink(),
        });
    }
}
