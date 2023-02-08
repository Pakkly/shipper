use std::path::Path;

use crate::{
    defines, fslog, paths,
    remoteinstallerdata::{InstalledFile, StoredInstallData},
};
use log::info;
use pakkly_error::{ferror, FormattedError};
#[cfg(target_os = "macos")]
use walkdir::WalkDir;

#[cfg(target_os = "linux")]
use crate::webview_alert;

#[cfg(target_os = "linux")]
use std::os::unix::fs::PermissionsExt;

#[cfg(target_os = "windows")]
use winreg::{enums::*, RegKey};

#[cfg(target_os = "windows")]
use crate::common;

#[cfg(target_os = "linux")]
use crate::common;

fn add_installer_meta(parameters: &mut StoredInstallData) -> Result<(), FormattedError> {
    let root = paths::get_install_file_pakkly(parameters);
    #[cfg(target_os = "macos")]
    {
        for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
            let dst = entry.path().to_path_buf();
            parameters.installed_files_meta.push(InstalledFile::new(&dst)?);
        }
        parameters.installed_files_meta.push(InstalledFile::new(&root)?);
    }
    #[cfg(not(target_os = "macos"))]
    {
        //other platforms do not have folders as their executable format.
        parameters.installed_files_meta.push(InstalledFile::new(&root)?);
    }
    parameters.installed_files_meta.push(InstalledFile::new(&paths::get_install_dir_pakkly())?);
    parameters.installed_files_meta.push(InstalledFile::new(&paths::get_install_path())?);
    parameters.installed_files_meta.push(InstalledFile::new(&paths::get_install_root())?);
    parameters.installed_files_meta.push(InstalledFile::new(&paths::get_json_path())?);
    Ok(())
}
#[cfg(target_os = "windows")]
fn windows_registry_setup(parameters: &mut StoredInstallData) -> Result<(), FormattedError> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = Path::new(&(*defines::UNINSTALL_REGKEY));
    let (key, _disp) = hkcu.create_subkey(&path)?;
    let mut estimated_size = 0;
    for item in &parameters.installed_files {
        estimated_size += item.size;
    }
    let reg_true: u32 = 1;
    key.set_value("DisplayIcon", &common::path_str(&common::find_executable_path(parameters, None).unwrap().unwrap()))?;
    key.set_value("DisplayName", &parameters.fetched_meta.app_name)?;
    key.set_value("EstimatedSize", &((estimated_size / 1024) as u32))?;
    key.set_value("NoModify", &reg_true)?;
    key.set_value("NoRepair", &reg_true)?;
    if parameters.fetched_meta.company_name.is_some() {
        key.set_value("Publisher", parameters.fetched_meta.company_name.as_ref().unwrap())?;
    }
    let uninstall_batch_file = Path::new(&paths::get_install_dir_pakkly()).join("uninstall.bat");
    key.set_value("UninstallString", &format!("\"{}\"", common::path_str(&uninstall_batch_file)))?;
    key.set_value(
        "QuietUninstallString",
        &format!(
            "\"{}\" {}",
            common::path_str(&paths::get_install_file_pakkly(parameters)),
            defines::PAKKLY_CLI_UNINSTALL_QUIET
        ),
    )?;
    return Ok(());
}

#[cfg(target_os = "windows")]
pub fn windows_registry_teardown() -> Result<(), FormattedError> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = Path::new(&(*defines::UNINSTALL_REGKEY));
    let _ = hkcu.delete_subkey(&path); //ignore errors.
    return Ok(());
}

fn place_icons_and_shortcuts(parameters: &mut StoredInstallData, fresh: bool) -> Result<(), FormattedError> {
    if parameters.shipperfile.is_none() {
        return Err(ferror!("place_icons_and_shortcuts called without shipperfile"));
    }
    let pakkly_installed = &paths::get_install_file_pakkly(parameters);
    let x = paths::get_install_dir_pakkly();
    let shipper_dir = Path::new(&x);
    let licenses_file = shipper_dir.join("OSS_LICENSES.txt");
    fslog::write(&licenses_file, licensor::LICENSES)?;
    info!("Placing shortcuts and meta files...");
    parameters.installed_files_meta.clear();
    parameters.installed_files_meta.push(InstalledFile::new(&licenses_file)?);
    #[cfg(target_os = "macos")]
    {
        let mut lnkfile = std::path::PathBuf::new();
        lnkfile.push("/Applications");
        lnkfile.push(format!("{}.app", parameters.fetched_meta.app_name));
        let ln_meta = std::fs::symlink_metadata(&lnkfile);
        if fresh && ln_meta.is_err() {
            //err means it doesn't exist
            std::process::Command::new("ln")
                .arg("-s")
                .arg(&pakkly_installed.as_os_str())
                .arg("/Applications")
                .status()?;
        }
        parameters.installed_files_meta.push(InstalledFile::new(&lnkfile)?);
    }

    #[cfg(target_os = "linux")]
    {
        let shipperfile = parameters.shipperfile.as_ref().unwrap();
        if !common::arg_flag_set(defines::PAKKLY_CLI_NOROOT) {
            use crate::defines::PAKKLY_ID_CLEAN;
            use crate::os_spec::linux;
            use base64::{
                alphabet,
                engine::{self, general_purpose},
                Engine as _,
            };

            const PREFIX: &str = "data:image/png;base64,";
            if !shipperfile._generated.icon.starts_with(PREFIX) {
                return Err(FormattedError::from_str(format!("Missing data type for icon. Expected: {}", PREFIX)));
            }
            let mut linsudo = linux::LinuxSudo::new();
            let png_data =
                &general_purpose::STANDARD.decode(&shipperfile._generated.icon.trim_start_matches(PREFIX))?;
            let png_file = shipper_dir.join("icon.png");
            let png_symlink = format!("/usr/share/icons/hicolor/256x256/apps/pak_{}.png", *PAKKLY_ID_CLEAN);

            fslog::write(&png_file, png_data)?;
            if fresh {
                linsudo.command("rm".to_string(), ["-f".to_string(), png_symlink.clone()].to_vec());
                linsudo.command(
                    "ln".to_string(),
                    ["-s".to_string(), png_file.to_str().unwrap().to_string(), png_symlink.clone()].to_vec(),
                );
            }

            let desktop_data = format!(
                "[Desktop Entry]
Encoding=UTF-8
Type=Application
Name={}
Comment={}
Exec={}
Icon={}
Terminal=false
Categories=GNOME;Application;
StartupNotify=true",
                parameters.fetched_meta.app_name,
                parameters.fetched_meta.description.clone().unwrap_or("".to_string()),
                common::path_str(&pakkly_installed),
                png_symlink
            );
            let desktop_file = shipper_dir.join("linux.desktop");
            let desktop_symlink = format!("/usr/share/applications/pak_{}.desktop", *PAKKLY_ID_CLEAN);

            fslog::write(&desktop_file, desktop_data)?;
            if fresh {
                linsudo.command("rm".to_string(), ["-f".to_string(), desktop_symlink.clone()].to_vec());
                linsudo.command(
                    "ln".to_string(),
                    ["-s".to_string(), desktop_file.to_str().unwrap().to_string(), desktop_symlink.clone()].to_vec(),
                );
            }

            //linsudo.command("ln".to_string(), ["-s".to_string(),pakkly_installed.to_string(),"/usr/bin".to_string()].to_vec());
            if fresh {
                let e = linsudo.flush();
                if e.is_err() {
                    let err = e.unwrap_err();
                    if err.is_missing_sudo {
                        webview_alert::alert(
                            "Not root.",
                            "This program needs to be run as root to complete the initial installation.",
                            None,
                        );
                        common::exit(1);
                    }
                }
            }
            let uninstall_bash_file = Path::new(&paths::get_install_dir_pakkly()).join("uninstall.sh");
            fslog::write(
                &uninstall_bash_file,
                format!("sudo {} {}", common::path_str(&pakkly_installed), defines::PAKKLY_CLI_UNINSTALL),
            )?;
            std::fs::set_permissions(&uninstall_bash_file, std::fs::Permissions::from_mode(0o744))?;

            /*let mut lnkfile = std::path::PathBuf::new();
            lnkfile.push("/usr/bin");
            lnkfile.push(&parameters.fetched_meta.app_name);*/

            parameters.installed_files_meta.push(InstalledFile::new(&uninstall_bash_file)?);
            parameters.installed_files_meta.push(InstalledFile::new(&png_symlink)?);
            parameters.installed_files_meta.push(InstalledFile::new(&desktop_symlink)?);
            parameters.installed_files_meta.push(InstalledFile::new(&png_file)?);
            parameters.installed_files_meta.push(InstalledFile::new(&desktop_file)?);
            //parameters.installed_files_meta.push(InstalledFile::new(&lnkfile)?);
        }
    }
    #[cfg(target_os = "windows")]
    {
        use directories::UserDirs;
        use windows_interface::make_lnk;
        let user_dirs = UserDirs::new().unwrap();

        let desktop_folder = user_dirs.desktop_dir().unwrap().join(format!("{}.lnk", parameters.fetched_meta.app_name));

        make_lnk(
            &Path::new(pakkly_installed),
            &desktop_folder,
            &Path::new(&common::find_executable_path(&parameters, None).unwrap().unwrap()),
            "",
            false,
        )?;
        let base_dirs = directories::BaseDirs::new().unwrap();
        let appdata = base_dirs
            .data_dir()
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join(format!("{}.lnk", parameters.fetched_meta.app_name));
        make_lnk(
            &Path::new(pakkly_installed),
            &appdata,
            &Path::new(&common::find_executable_path(&parameters, None).unwrap().unwrap()),
            "",
            false,
        )?;

        let uninstall_path = Path::new(&paths::get_install_dir_pakkly())
            .join(format!("Uninstall {}.lnk", parameters.fetched_meta.app_name));
        make_lnk(
            &Path::new(pakkly_installed),
            &uninstall_path,
            &Path::new(&common::find_executable_path(&parameters, None).unwrap().unwrap()),
            defines::PAKKLY_CLI_UNINSTALL,
            true,
        )?;

        let uninstall_batch_file = Path::new(&paths::get_install_dir_pakkly()).join("uninstall.bat");
        fslog::write(
            &uninstall_batch_file,
            format!("@echo off\r\nstart \"\" \"{}\"", uninstall_path.to_str().unwrap()),
        )?;

        parameters.installed_files_meta.push(InstalledFile::new(&uninstall_batch_file)?);
        parameters.installed_files_meta.push(InstalledFile::new(&desktop_folder)?);
        parameters.installed_files_meta.push(InstalledFile::new(&uninstall_path)?);
        parameters.installed_files_meta.push(InstalledFile::new(&appdata)?);
    }

    info!("Done!");
    Ok(())
}

pub fn replace_meta_files(parameters: &mut StoredInstallData) -> Result<(), FormattedError> {
    parameters.write_json()?;
    place_icons_and_shortcuts(parameters, *defines::FRESH_INSTALL)?;
    add_installer_meta(parameters)?;
    #[cfg(target_os = "windows")]
    {
        windows_registry_teardown()?;
        windows_registry_setup(parameters)?;
    }
    parameters.write_json()?;

    Ok(())
}
