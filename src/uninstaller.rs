use crate::common;
use crate::fslog;
use crate::ipc;
use crate::remoteinstallerdata::StoredInstallData;
use crate::{
    common::is_hash_whitelisted,
    defines,
    remoteinstallerdata::InstalledFile,
    webview_alert::{self, ConfirmParams},
};
use hex;
use log::{info, trace, warn};
use pakkly_error::FormattedError;
use std::fs;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use crate::installer_tools;

struct InstallerInfoPathed {
    pub file: InstalledFile,
    pub path: PathBuf,
}
pub fn uninstall_procedure(local_data: &StoredInstallData, quiet: bool) -> Result<(), FormattedError> {
    if !quiet {
        let confirm_uninstall = webview_alert::confirm(ConfirmParams {
            title: "Confirm Uninstall".into(),
            body: format!("Are you sure you want to uninstall {}?", local_data.fetched_meta.app_name).into(),
            image: webview_alert::ConfirmImage::Question,
            no_str: "No".into(),
            yes_str: "Yes".into(),
        });
        if !confirm_uninstall {
            common::exit(0);
        }
    }
    let mut paths: Vec<InstallerInfoPathed> = vec![];
    info!("Preparing uninstall list from program...");
    for installed_file in &local_data.installed_files {
        let mut pb = PathBuf::new();
        if installed_file.root.is_some() {
            pb.push(&installed_file.root.as_ref().unwrap());
        }
        pb.push(&installed_file.dst_path);
        trace!("Path is: {:?}", pb);
        paths.push(InstallerInfoPathed { path: pb, file: installed_file.to_owned() })
    }
    info!("Preparing uninstall list from meta...");
    for installed_file in &local_data.installed_files_meta {
        let mut pb = PathBuf::new();
        if installed_file.root.is_some() {
            pb.push(&installed_file.root.as_ref().unwrap());
        }
        pb.push(&installed_file.dst_path);
        trace!("Path is: {:?}", pb);
        paths.push(InstallerInfoPathed { path: pb, file: installed_file.to_owned() })
    }
    info!("Erasing dangling IPC items...");
    ipc::erase_all()?;
    info!("Uninstalling {} items...", paths.len());
    paths.sort_by(|a, b| {
        //sort by length so that a/b/c gets deleted before a, used for folder deletion
        b.path.as_os_str().len().cmp(&a.path.as_os_str().len())
    });
    for installed in &paths {
        //first delete all the files
        let meta = fslog::get_simple_fs_meta_symlink(&installed.path);
        if meta.is_none() || meta.unwrap().is_directory {
            continue;
        }
        if !is_hash_whitelisted(&installed.path) && installed.file.hash != defines::HASH_ALWAYS_REPLACE {
            //can compare hashes, check first
            let (_, hash_of_fs) = common::get_file_hash(&installed.path)?;
            let stored_hash = hex::decode(&installed.file.hash)?;
            if hash_of_fs != stored_hash {
                //its been changed since the install, do not delete!
                warn!("Hash mismatch, won't delete {:?}", installed.path);
                warn!("Expected {} but got {}", installed.file.hash, hex::encode(hash_of_fs));
                continue;
            }
        }
        let result = fslog::remove_file(&installed.path);
        if result.is_err() {
            warn!("Could not remove file");
            #[cfg(target_os = "windows")]
            {
                warn!("Scheduling file deletion: {:?}", installed.path);
                if common::arg_flag_set(defines::PAKKLY_CLI_NOROOT) {
                    continue;
                }
                let fd = windows_interface::schedule_file_delete(&installed.path);
                if fd.is_err() {
                    warn!("Scheduling failed! {:?}", fd.unwrap_err());
                }
            }
            continue;
        }
    }
    for installed in &paths {
        //now delete the folders
        let meta = fslog::get_simple_fs_meta_symlink(&installed.path);
        if meta.is_none() || meta.unwrap().is_file {
            continue;
        }
        let result = fs::remove_dir(&installed.path);
        if result.is_err() {
            warn!("Removing path error: path={:?},{}", installed.path, result.err().unwrap());
            #[cfg(target_os = "windows")]
            {
                if common::arg_flag_set(defines::PAKKLY_CLI_NOROOT) {
                    continue;
                }
                warn!("Could not remove folder, will schedule: {:?}", installed.file.dst_path);
                let fd = windows_interface::schedule_file_delete(&installed.path);
                if fd.is_err() {
                    warn!("Scheduling failed! {:?}", fd.unwrap_err());
                }
            }
            continue;
        }
        info!("Removed folder: {:?}", installed.path);
    }
    #[cfg(target_os = "windows")]
    {
        installer_tools::windows_registry_teardown()?;
    }
    common::exit(0);
}
