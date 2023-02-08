use std::{
    convert::TryFrom,
    path::{Path, PathBuf},
};

use log::error;
use pakkly_error::FormattedError;
use serde::{Deserialize, Serialize};
use std::u8;
use web_view::Color;

use crate::{
    common::{self, is_hash_whitelisted},
    defines, fslog, paths,
    shipperfile::Shipperfile,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StoredInstallData {
    pub background_color: InstallerColor,
    pub installed_files: Vec<InstalledFile>,
    pub installed_files_meta: Vec<InstalledFile>,
    pub fetched_meta: PakklyMetaRemote,
    pub installed_date: i64,
    pub last_launch: i64,
    pub last_ucheck: i64,
    pub installing: bool,
    pub shipperfile: Option<Shipperfile>,
    pub installed_app_info: DownloadParams,
}
impl StoredInstallData {
    pub fn from(o: PakklyMetaRemote) -> Result<StoredInstallData, FormattedError> {
        return Ok(StoredInstallData {
            background_color: InstallerColor::try_from(o.background_color.as_str())?,
            installed_files: [].to_vec(),
            installed_files_meta: [].to_vec(),
            installed_date: 0,
            installed_app_info: o.app.to_owned(),
            fetched_meta: o,
            last_launch: 0,
            last_ucheck: 0,
            installing: false,
            shipperfile: None,
        });
    }
    pub fn write_json(&self) -> Result<(), FormattedError> {
        let bad = defines::HASH_DEFER.to_string();
        let bad_normal = &self.installed_files.iter().find(|a| a.hash == bad);
        if bad_normal.is_some() {
            error!("BAD NORMAL: {}", bad_normal.unwrap().dst_path_human);
            return Err(FormattedError::from_str("Deferred hashes must be fulfilled before writing!".to_string()));
        }
        let bad_meta = &self.installed_files_meta.iter().find(|a| a.hash == bad);
        if bad_meta.is_some() {
            error!("BAD META: {}", bad_meta.unwrap().dst_path_human);
            return Err(FormattedError::from_str("Deferred hashes must be fulfilled before writing!".to_string()));
        }
        let json = serde_json::to_string(&self)?;
        fslog::write(paths::get_json_path(), json)?;
        Ok(())
    }
    pub fn read_json() -> Result<StoredInstallData, FormattedError> {
        let path: PathBuf = PathBuf::from(paths::get_json_path());
        if fslog::exists(&path) {
            let file_data = fslog::read_to_string(path)?;
            let rid: StoredInstallData = serde_json::from_str(&file_data)?;
            if rid.installing {
                return Err(FormattedError::from_str("Install corrupted".to_string()));
            }
            return Ok(rid);
        } else {
            return Err(FormattedError::from_str("File not found!".to_string()));
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PakklyMetaRemote {
    pub background_color: String,
    pub description: Option<String>,
    pub shipper: DownloadParams,
    pub app: DownloadParams,
    pub app_name: String,
    pub company_name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct InstallerColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}
impl From<InstallerColor> for Color {
    fn from(ic: InstallerColor) -> Self {
        Color { r: ic.r, g: ic.g, b: ic.b, a: ic.a }
    }
}
impl TryFrom<&str> for InstallerColor {
    type Error = FormattedError;
    fn try_from(value: &str) -> Result<Self, FormattedError> {
        let trimmed = value.trim_start_matches("#");
        if trimmed.len() == 8 {
            return Ok(InstallerColor {
                r: u8::from_str_radix(&trimmed[0..2], 16)?,
                g: u8::from_str_radix(&trimmed[2..4], 16)?,
                b: u8::from_str_radix(&trimmed[4..6], 16)?,
                a: u8::from_str_radix(&trimmed[6..8], 16)?,
            });
        } else {
            return Err(FormattedError::from_str("Color string malformed!".to_string()));
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstalledFile {
    pub dst_path: PathBuf,
    pub dst_path_human: String,
    pub hash: String,
    pub size: u64,
    pub root: Option<PathBuf>,
}
pub struct FileContentsMeta {
    pub hash: String,
    pub size: u64,
}

impl InstalledFile {
    pub fn rehash(&mut self) -> Result<(), FormattedError> {
        let mut absolute = PathBuf::new();
        if self.root.is_some() {
            absolute.push(&self.root.as_ref().unwrap());
        }
        absolute.push(&self.dst_path);

        let fcm = Self::size_and_hash(&absolute)?;
        self.size = fcm.size;
        self.hash = fcm.hash;
        Ok(())
    }
    fn size_and_hash(absolute: &PathBuf) -> Result<FileContentsMeta, FormattedError> {
        let hash: String;
        let size: u64;
        let meta = std::fs::symlink_metadata(&absolute);
        if meta.is_err() {
            return Err(FormattedError::from_str(format!("Non-existant file added to db: {:?}", absolute)));
        }
        let muw = meta.unwrap();
        if muw.is_symlink() {
            hash = defines::HASH_ALWAYS_REPLACE.to_string();
            size = 0;
        } else {
            if muw.is_dir() {
                hash = defines::HASH_DIRECTORY.to_string();
                size = 0;
            } else {
                if !absolute.exists() {
                    return Err(FormattedError::from_str(format!(
                        "Non-existant file added to db(symlink followed): {:?}",
                        absolute
                    )));
                }
                hash = match is_hash_whitelisted(&absolute) {
                    true => defines::HASH_ALWAYS_REPLACE.to_string(),
                    false => {
                        let (fh_num, fh_bytes) = common::get_file_hash(&absolute)?;
                        if fh_num == 0 {
                            defines::HASH_DEFER.to_string()
                        } else {
                            hex::encode(fh_bytes)
                        }
                    }
                };
                size = muw.len();
            }
        }
        Ok(FileContentsMeta { size, hash })
    }
    pub fn new_rooted_precontent<P: AsRef<Path>>(
        file_path: &P,
        root: Option<&P>,
        content_meta: FileContentsMeta,
    ) -> Result<Self, FormattedError> {
        Ok(Self {
            dst_path: file_path.as_ref().to_path_buf(),
            dst_path_human: file_path.as_ref().as_os_str().to_string_lossy().to_string(),
            hash: content_meta.hash,
            size: content_meta.size,
            root: match root {
                Some(r) => Some(r.as_ref().to_path_buf()),
                None => None,
            },
        })
    }
    pub fn new_rooted<P: AsRef<Path>>(file_path: &P, root: Option<&P>) -> Result<Self, FormattedError> {
        let mut absolute;
        if root.is_some() {
            absolute = root.unwrap().as_ref().to_path_buf();
            absolute.push(file_path);
        } else {
            absolute = file_path.as_ref().to_path_buf();
        }
        return Self::new_rooted_precontent(file_path, root, Self::size_and_hash(&absolute)?);
    }
    pub fn new<P: AsRef<Path>>(file_path: &P) -> Result<Self, FormattedError> {
        return Self::new_rooted(file_path, None);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DownloadParams {
    pub url: String,
    pub version: String,
}
