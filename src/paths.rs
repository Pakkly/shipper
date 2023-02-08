use std::path::{Path, PathBuf};

use directories::BaseDirs;

use crate::{defines::PAKKLY_ID_CLEAN, remoteinstallerdata::StoredInstallData};

pub fn get_json_path() -> PathBuf {
    return get_install_dir_pakkly().join("pakkly.installer.json");
}
pub fn get_install_dir_pakkly_update() -> PathBuf {
    return get_install_subdir("runner_update");
}
pub fn get_install_file_pakkly_update(parameters: &StoredInstallData) -> PathBuf {
    let path = get_install_dir_pakkly_update();
    let dest = Path::new(&path);

    let extension = {
        #[cfg(target_os = "windows")]
        {
            ".exe"
        }
        #[cfg(target_os = "macos")]
        {
            ".app"
        }
        #[cfg(target_os = "linux")]
        {
            ""
        }
    };
    return dest.join(format!("{}{}", parameters.fetched_meta.app_name, extension));
}
pub fn get_ipc_dir() -> PathBuf {
    return get_install_subdir("runner").join("ipc");
}
pub fn get_install_path() -> PathBuf {
    return get_install_subdir("program");
}
pub fn get_install_dir_pakkly() -> PathBuf {
    return get_install_subdir("runner");
}
pub fn get_install_file_pakkly(parameters: &StoredInstallData) -> PathBuf {
    let path = get_install_dir_pakkly();
    let dest = Path::new(&path);

    let extension = {
        #[cfg(target_os = "windows")]
        {
            ".exe"
        }
        #[cfg(target_os = "macos")]
        {
            ".app"
        }
        #[cfg(target_os = "linux")]
        {
            ""
        }
    };
    let app_name = {
        #[cfg(target_os = "linux")]
        {
            &parameters.fetched_meta.app_name.replace(" ", "_")
        }
        #[cfg(not(target_os = "linux"))]
        {
            &parameters.fetched_meta.app_name
        }
    };
    return dest.join(format!("{}{}", app_name, extension));
}
pub fn get_install_root() -> PathBuf {
    let dest = BaseDirs::new().unwrap();
    let destination = dest.data_local_dir().join(&["pakked_", (*PAKKLY_ID_CLEAN).as_str()].concat());
    return destination;
}
fn get_install_subdir(subpath: &str) -> PathBuf {
    return get_install_root().join(subpath);
}
