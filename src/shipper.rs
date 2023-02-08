use crate::remoteinstallerdata::StoredInstallData;
use crate::{common, fslog, paths};
use crate::{installer_tools, unzip};
use log::info;
use pakkly_error::FormattedError;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[cfg(target_os = "windows")]
use crate::common::execute_detached;

#[cfg(target_os = "windows")]
use crate::defines;

#[cfg(target_os = "linux")]
use crate::common::untar;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub fn indirect_update(local_data: &mut StoredInstallData) {
    std::thread::sleep(std::time::Duration::from_secs(1)); //wait for cleanup
    let src_path = Path::new(&paths::get_install_file_pakkly_update(&local_data)).to_path_buf();
    let dest_path = Path::new(&paths::get_install_file_pakkly(&local_data)).to_path_buf();
    let dest_path_folder = dest_path.parent().unwrap();

    if fslog::exists(&dest_path_folder) {
        info!("Deleting the previous install: {}", dest_path_folder.to_str().unwrap());
        fslog::remove_dir_all(&dest_path_folder).unwrap();
    }
    fslog::create_dir_all(&dest_path_folder).unwrap();

    info!("Writing pakkly from: {}", &src_path.to_str().unwrap());
    info!("Writing pakkly to: {}", dest_path.to_str().unwrap());
    if src_path.is_dir() {
        fs::create_dir(&dest_path).unwrap();
        fslog::copy_dir(&src_path, &dest_path).unwrap();
    } else {
        fslog::copy(&src_path, &dest_path).unwrap();
    }
    installer_tools::replace_meta_files(local_data).unwrap();
}
pub fn update_shipper(rdata: &mut StoredInstallData) -> Result<(), FormattedError> {
    #[cfg(target_os = "windows")]
    {
        //windows executables cannot update while running.
        use crate::remoteinstallerdata::InstalledFile;
        let new_app = paths::get_install_file_pakkly_update(&rdata);
        install_shipper(&new_app, rdata)?;

        rdata.installed_files_meta.push(InstalledFile::new(&paths::get_install_file_pakkly_update(&rdata))?);
        rdata.installed_files_meta.push(InstalledFile::new(&paths::get_install_dir_pakkly_update())?);
        rdata.write_json()?;

        let args = vec![defines::PAKKLY_CLI_REPLACE_SHIPPER];
        execute_detached(&new_app, &args).unwrap();
        //this causes a new process to spawn detached and for it to enter indirect_update
        common::exit(0);
    }
    #[cfg(unix)]
    {
        let new_app = paths::get_install_file_pakkly(&rdata);
        install_shipper(&new_app, rdata)?;
        Ok(())
    }
}

pub fn install_shipper(destination_path: &PathBuf, params: &mut StoredInstallData) -> Result<(), FormattedError> {
    info!("Installing shipper...");
    let tmpdir = tempdir()?;
    let mut tmpfilepath = tmpdir.path().to_path_buf();
    tmpfilepath.push("temporary_download_pakkly");
    common::download_file(&params.fetched_meta.shipper.url, &tmpfilepath, |_a, _b| {})?;

    let zip_tmpdir = tempdir()?;
    let unzip_path = zip_tmpdir.path().to_path_buf();
    if unzip::is_file_zip(&tmpfilepath)? {
        info!("Zip detected! Unzipping to {:?}", unzip_path);
        unzip::extract(&tmpfilepath, &unzip_path, |_a, _b| {})?;
    } else {
        #[cfg(target_os = "linux")]
        {
            untar(&tmpfilepath, &unzip_path)?;
        }
        #[cfg(not(target_os = "linux"))]
        {
            let final_dir = zip_tmpdir.path().join("a_file");
            fslog::copy(&tmpfilepath, &final_dir)?;
        }
    }
    {
        let dest_path_with_filename = Path::new(&destination_path).to_path_buf();
        let dest_path_folder = dest_path_with_filename.parent().unwrap().to_path_buf();
        if fslog::exists(&dest_path_folder) {
            info!("Deleting the previous install: {:?}", dest_path_folder);
            fslog::remove_dir_all(&dest_path_folder)?;
        }
        fslog::create_dir_all(&dest_path_folder)?;

        fslog::copy_dir(&unzip_path, &dest_path_folder)?;

        #[cfg(unix)]
        {
            let all_files = common::all_relative_files_in_folder_recursive(&dest_path_folder)?;
            for file in all_files {
                fs::set_permissions(dest_path_folder.join(file), fs::Permissions::from_mode(0o744))?
            }
        }

        let original_filename = common::find_shipper_in_folder(&unzip_path)?.unwrap();
        let src_filename = Path::new(&original_filename).file_name().unwrap().to_str().unwrap();
        let wrong_filename = dest_path_folder.join(src_filename);

        fslog::rename(wrong_filename, dest_path_with_filename)?;
    }

    info!("Done!");
    return Ok(());
}
