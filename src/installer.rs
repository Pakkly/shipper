use crate::common::{self, get_shipperfile, is_hash_whitelisted};
use crate::remoteinstallerdata::{FileContentsMeta, InstalledFile, StoredInstallData};
use crate::{common::InstallProgressSegment, defines};
use crate::{fslog, installer_tools, paths, shipper, unzip};
use chrono::Utc;
use hex;
use log::{info, trace, warn};
use pakkly_error::FormattedError;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use tempfile::tempdir;

#[cfg(unix)]
use log::error;

#[cfg(unix)]
use std::fs;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

struct UpdateFileInfo {
    from_path: String,
    to_path: String,
}

pub fn install<F>(parameters: &mut StoredInstallData, cb: F) -> Result<(), FormattedError>
where
    F: Fn(f32, InstallProgressSegment),
{
    let destination = paths::get_install_path();
    fslog::create_dir_all(&destination)?;

    let tmpdir = tempdir()?;
    let tmpfilepath = tmpdir.path().join("temporary_download_pakkly");

    info!("Downloading to : {:?}", tmpfilepath);

    common::download_file(&parameters.fetched_meta.app.url, &tmpfilepath, &cb)?;
    diff_update(&tmpfilepath, &destination, parameters, &cb)?;

    let now = Utc::now();
    parameters.installed_date = now.timestamp_millis();

    if *defines::FRESH_INSTALL {
        let target = paths::get_install_file_pakkly(&parameters);
        shipper::install_shipper(&target, parameters)?;
    }
    parameters.installed_app_info = parameters.fetched_meta.app.to_owned(); //set the installed version!

    parameters.installing = false;
    installer_tools::replace_meta_files(parameters)?;

    return Ok(());
}

fn check_files<F>(
    update_files: &Vec<PathBuf>,
    unzip_path: &PathBuf,
    target_directory: &PathBuf,
    app_files: &Vec<InstalledFile>,
    progress_cb: F,
) -> Result<(Vec<UpdateFileInfo>, Vec<InstalledFile>), FormattedError>
where
    F: Fn(f32, InstallProgressSegment),
{
    let total_count = update_files.len();
    let mut current_index = 0;
    let mut update_list: Vec<UpdateFileInfo> = vec![];
    let mut updated_file_list: Vec<InstalledFile> = vec![];
    for update_path_relative in update_files {
        let update_path_abs: PathBuf = [unzip_path, &update_path_relative].iter().collect();
        let app_path_abs: PathBuf = [target_directory, &update_path_relative].iter().collect(); //looks wrong but isnt

        let mut installed_file = app_files.iter().find(|x| *update_path_relative == x.dst_path);

        if fslog::exists(&app_path_abs) {
            if app_path_abs.is_dir() && update_path_abs.is_dir() {
                //both source and dest are directories and exist, nothing to do here.
                info!("Found existing directory, skipping: {:?}", update_path_relative);
                updated_file_list.push(InstalledFile::new_rooted(&update_path_relative, Some(&target_directory))?);
                progress_cb(
                    ((current_index as f64) / (total_count as f64) * 0.25 + 0.5) as f32,
                    InstallProgressSegment::Installing,
                );
                current_index += 1;
                continue;
            }
            if app_path_abs.is_dir() != update_path_abs.is_dir() {
                //a file has been chaned to a directory or vice-versa. Sha is incomparable.
                if app_path_abs.is_dir() {
                    warn!("WARNING, dir -> file detected. Directory will be erased: {:?}", update_path_relative);
                    fslog::remove_dir_all(&app_path_abs)?;
                } else {
                    warn!("WARNING, file -> dir detected. File will be erased: {:?}", update_path_relative);
                    fslog::remove_file(&app_path_abs)?;
                }
                installed_file = None;
            }
        }
        let mut new_precontent = FileContentsMeta { size: 0, hash: defines::HASH_DEFER.to_string() };
        if update_path_abs.is_dir() {
            if !fslog::exists(&app_path_abs) {
                fslog::create_dir_all(app_path_abs)?;
            }
            updated_file_list.push(InstalledFile::new_rooted(&update_path_relative, Some(&target_directory))?);

            continue;
        }
        if installed_file.is_some() {
            //file was already installed once, compare and replace if necessary.
            let installed = installed_file.unwrap();
            let mut needs_update = true;
            if fslog::exists(&app_path_abs) && !is_hash_whitelisted(&installed.dst_path) {
                let sha_installed = hex::decode(&installed.hash)?;
                let (_, sha_update) = common::get_file_hash(&update_path_abs)?;
                needs_update = sha_installed != sha_update;
                new_precontent.hash = hex::encode(sha_update);
            }
            if needs_update {
                //file has changed and needs to be updated!
                info!("Found hash difference or missing file, updating: {:?}", update_path_relative);
                fslog::file_open(&update_path_abs)?; //checking that it's readable.
                std::fs::OpenOptions::new().read(true).write(true).truncate(false).create(true).open(&app_path_abs)?; //checking that it's writable
                update_list.push(UpdateFileInfo {
                    from_path: update_path_abs.to_str().unwrap().to_owned(),
                    to_path: app_path_abs.to_str().unwrap().to_owned(),
                })
            } else {
                info!("Found hash match, skipping: {:?}", update_path_relative)
            }
        } else {
            //this file is new, create it!
            info!("New file: {:?} .. from = {:?}", update_path_relative, update_path_abs);
            fslog::file_open(&update_path_abs)?; //checking that the source file is readable.
            fslog::create_dir_all(&app_path_abs.parent().unwrap())?;
            File::create(&app_path_abs)?; //making the file, and ensuring dest is writable

            update_list.push(UpdateFileInfo {
                from_path: update_path_abs.to_str().unwrap().to_owned(),
                to_path: app_path_abs.to_str().unwrap().to_owned(),
            })
        }
        // Get and Set permissions
        updated_file_list.push(InstalledFile::new_rooted_precontent(
            update_path_relative,
            Some(&target_directory),
            new_precontent,
        )?);
        progress_cb(
            ((current_index as f64) / (total_count as f64) * 0.25 + 0.5) as f32,
            InstallProgressSegment::Installing,
        );
        current_index += 1;
    }
    Ok((update_list, updated_file_list))
}

fn diff_update<F>(
    downloaded_file: &PathBuf,
    target_directory: &PathBuf,
    params: &mut StoredInstallData,
    progress_cb: F,
) -> Result<(), FormattedError>
where
    F: Fn(f32, InstallProgressSegment),
{
    /*
        The most important function in this project. Takes a source file, target directory, and install params
        This function determines if the unzips the source file to a temp folder

        Then the diffing algorithm is run over the contents of 'temp folder' and 'target directory', once for every item in 'temp folder'.
        It starts by opening each equivalent file in 'target directory' for writing. If it is successful the file gets added to the list of copy-targets.
        The file is then closed to respect ulimit.

        If the 'temp file' hash matches the hash of the file with the same name in the 'target directory' (stored in 'install params'),
        it is not added to the list of copy-targets.

        Once all the files in 'target directory' have been evaluated to be writable, they are truncated and written to with the contents of their equivalent
        file in 'temp folder'.

        Then, it cleans up the old files that are no longer needed.

        Returns the list of relative paths that constituted this update, including ones whos hash matched, for easier processing.
    */
    let fresh_install = params.installed_files.len() == 0;
    info!("DIFFUPDATE procedure started:");
    info!("fresh_install={}", fresh_install);
    info!("Updating...");

    let zip_tmpdir = tempdir()?;
    let unzip_path = zip_tmpdir.path().to_path_buf();

    let mut empty = [].to_vec();
    unzip::extract(&downloaded_file, &unzip_path, &progress_cb)?;

    let shipperfile = get_shipperfile(&unzip_path)?;
    params.shipperfile = Some(shipperfile);

    if common::find_executable_path(params, Some(&unzip_path)).unwrap().is_none() {
        //cannot find by glob in update directory! Install cannot continue.
        common::submit_basic_crash("Cannot find executable in new update");
        return Err(FormattedError::from_str("Cannot find executable in new update.".to_string()));
    }
    let app_files = match fresh_install {
        false => &mut params.installed_files,
        true => &mut empty,
    };

    let update_files: Vec<PathBuf> = common::all_relative_files_in_folder_recursive(&unzip_path)?;
    log::info!("Update files: {:?}", update_files);
    //open all files for writing BEFORE writing to them to ensure proper permissions.
    let (update_list, mut updated_file_list) =
        check_files(&update_files, &unzip_path, target_directory, &app_files, &progress_cb)?;
    info!("All writes checked and hashes calculated...");

    if !fresh_install {
        //mark as dirty until updating completes
        let mut params_old = StoredInstallData::read_json()?;
        params_old.installing = true;
        params_old.write_json()?;
    }

    //all handles are openable, if we got here it means we can safely update. This next section must never error! If it does the install is corrupted.
    info!("Starting critical section...");
    {
        let total_count = update_list.len();
        let mut current_index = 0;
        let mut buffer: Vec<u8> = Vec::new();
        buffer.resize(1024 * 1024 * 20, 0x00);
        for handle in update_list {
            let mut h_from = fslog::file_open(&handle.from_path).unwrap();
            let mut h_to = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .truncate(false)
                .create(true)
                .open(&handle.to_path)?;
            buffer.fill(0x00);
            h_to.set_len(0).unwrap(); //this error must never fire!
            loop {
                let length = h_from.read(buffer.as_mut_slice()).unwrap();
                if length == 0 {
                    break;
                }
                let wlen = h_to.write(&buffer.as_slice()[0..length]).unwrap();
                if wlen == 0 {
                    panic!("Wrote 0 length!");
                }
            }
            #[cfg(unix)]
            {
                let metadata = h_from.metadata().unwrap();
                fs::set_permissions(&handle.to_path, metadata.permissions()).unwrap();
            }
            drop(h_to);
            drop(h_from);
            progress_cb(
                ((current_index as f64) / (total_count as f64) * 0.23 + 0.75) as f32,
                InstallProgressSegment::Installing,
            );
            current_index += 1;
        }
    }

    let bad_hash = defines::HASH_DEFER.to_string();
    for file in &mut updated_file_list {
        if file.hash == bad_hash {
            file.rehash()?;
        }
    }

    if !fresh_install {
        //mark as clean, we've passed the critical section
        params.installing = false;
        params.write_json()?;
    }

    progress_cb(0.99, InstallProgressSegment::Installing);
    //clean and remove old files
    let cleanup = cleanup_obsolete_files(&params.installed_files, &updated_file_list, target_directory);
    if cleanup.is_err() {
        warn!("Warning: Cleanup of obsolete files failed!");
    }
    #[cfg(unix)]
    {
        let exe_path = common::find_executable_path(&params, None).unwrap();
        if let Some(path_unwrapped) = exe_path {
            fs::set_permissions(&path_unwrapped, fs::Permissions::from_mode(0o744))?;
        } else {
            let e = "Exe path not found for permission set.";
            error!("{}", e);
            common::submit_basic_crash(e);
            return Err(FormattedError::from_str(e.to_string()));
        }
    }
    progress_cb(1.0, InstallProgressSegment::Installing);
    info!("DIFFUPDATE complete");
    params.installed_files = updated_file_list;
    Ok(())
}
fn cleanup_obsolete_files(
    installed_old: &Vec<InstalledFile>,
    installed_new: &Vec<InstalledFile>,
    target_directory: &PathBuf,
) -> Result<(), FormattedError> {
    let mut cleanup_dirs: Vec<PathBuf> = vec![];
    let mut cleanup_files: Vec<PathBuf> = vec![];
    info!("Cleanup started...");
    for file in installed_old {
        let file_kept = installed_new.iter().find(|x| x.dst_path.cmp(&file.dst_path) == std::cmp::Ordering::Equal);
        if file_kept.is_none() {
            let mut p: PathBuf = PathBuf::from(target_directory);
            p.push(&file.dst_path);
            if p.is_dir() {
                cleanup_dirs.push(p);
            } else {
                cleanup_files.push(p);
            }
        }
    }
    for file in cleanup_files {
        trace!("Removing obsolete file: {:?}", file);
        fslog::remove_file(file)?;
    }
    cleanup_dirs.sort_by(|a, b| {
        //sort by length so that a/b/c gets deleted before a
        b.as_os_str().len().cmp(&a.as_os_str().len())
    });
    for dir in cleanup_dirs {
        trace!("Removing obsolete dir: {:?}", dir);
        fslog::remove_dir_all(dir)?;
    }
    Ok(())
}
