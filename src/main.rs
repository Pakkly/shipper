#![windows_subsystem = "windows"]
pub mod common;
pub mod defines;
mod fslog;
mod installer;
mod installer_tools;
mod ipc;
mod logger;
pub mod os_spec;
mod paths;
pub mod remoteinstallerdata;
mod shipper;
mod shipperfile;
mod uninstaller;
mod unzip;
mod webview;
mod webview_alert;
use crate::{remoteinstallerdata::StoredInstallData, webview_alert::ConfirmParams};
use chrono::Utc;
use common::CrashState;
use log::{error, info};
use pakkly_error::FormattedError;
use std::thread;
use std::time::Duration;
use std::{
    panic::PanicInfo,
    sync::{Arc, Mutex},
};
fn unwrap_fe<T>(res: Result<T, FormattedError>) -> T
where
    T: std::fmt::Debug,
{
    if res.is_err() {
        let re = res.err().unwrap();
        error!("{:?}", re);
        panic!("Error occurred!");
    }
    return res.unwrap();
}
fn emit_panic_error(pi: &PanicInfo<'_>) {
    let mut error_msg = "".to_string();
    if let Some(msg) = pi.payload().downcast_ref::<&str>() {
        error_msg = format!("Panic Message: {:?}", msg);
    }
    if let Some(location) = pi.location() {
        error_msg =
            format!("{}\nPanic location:  {:?} at line {:?}", error_msg, location.file(), location.line()).to_string();
    } else {
        error_msg += "\nPanic in unknown file";
    }
    error!("{}", error_msg);
}
fn install_quiet_exit_hook(local_data: &mut StoredInstallData) {
    let install_quiet = common::arg_flag_set(defines::PAKKLY_CLI_INSTALL_QUIET);
    if install_quiet {
        unwrap_fe(installer::install(local_data, |_a, _b| {}));
        //common::execute_program_and_terminate(&local_data);

        common::exit(0);
    }
}
fn install_indirect_exit_hook(local_data: &mut StoredInstallData) {
    info!("Checking selfupdate-replace...");
    if common::arg_flag_set(defines::PAKKLY_CLI_REPLACE_SHIPPER) {
        info!("Selfupdate replace function triggered...");
        shipper::indirect_update(local_data)
    }
}
fn selfupdate_hook(local_data: &mut StoredInstallData) {
    let r_version = local_data.fetched_meta.shipper.version.clone();
    info!("Checking selfupdate-fetch...");
    if !*defines::FRESH_INSTALL && r_version != *defines::SHIPPER_VERSION_CLEAN {
        //must install, newer version found
        info!("New Shipper version found, will install...");
        info!("From {} to {}", *defines::SHIPPER_VERSION_CLEAN, r_version);
        let _e = shipper::update_shipper(local_data);
        if _e.is_err() {
            error!("Could not update shipper!");
        }
    }
}
fn repair_shipper_hook(local_data: &mut StoredInstallData) {
    let correct_path = paths::get_install_file_pakkly(local_data);
    if !*defines::FRESH_INSTALL
        && !fslog::exists(&correct_path)
        && !common::arg_flag_set(defines::PAKKLY_CLI_REPLACE_SHIPPER)
    {
        info!("Repair triggered!");
        let _e = shipper::install_shipper(&correct_path, local_data);
        if _e.is_err() {
            error!("Could not fix shipper!");
        }
    }
}
fn should_app_update(local_data: &mut StoredInstallData) -> bool {
    if !*defines::FRESH_INSTALL {
        let cloned = local_data.clone();
        let current_installation_date = cloned.installed_app_info.version;
        let potential_date = cloned.fetched_meta.app.version;
        info!("CD: {}", current_installation_date);
        info!("PD: {}", potential_date);
        if current_installation_date == potential_date {
            info!("Nothing to be done, launching.");
            return false;
        }
    }
    return true;
}
fn uninstall_exit_hook(local_data: Option<&mut StoredInstallData>) {
    let quiet = common::arg_flag_set(defines::PAKKLY_CLI_UNINSTALL_QUIET);
    let regular = common::arg_flag_set(defines::PAKKLY_CLI_UNINSTALL);
    if quiet || regular {
        if local_data.is_none() {
            error!("Uninstall process triggered without installation present.");
            common::exit(1);
        }
        //uninstall procedure triggered
        unwrap_fe(uninstaller::uninstall_procedure(local_data.unwrap(), quiet));
        common::exit(0); //actually is never called since uninstall does that itself. Just a compiler hint
    }
}
fn force_update_hook(local_data: &mut StoredInstallData) {
    let specific_app = common::arg_value_set(defines::PAKKLY_CLI_INSTALLEXACT_APP);
    let specific_shipper = common::arg_value_set(defines::PAKKLY_CLI_INSTALLEXACT_SHIPPER);

    if specific_app.is_some() || specific_shipper.is_some() {
        let new_response_data = common::get_update_info(Some(local_data), specific_app, specific_shipper);
        local_data.fetched_meta = new_response_data.unwrap();
    }
}
fn update_timer_exit_hook(local_data: &mut StoredInstallData) {
    let specific_app = common::arg_value_set(defines::PAKKLY_CLI_INSTALLEXACT_APP);
    let specific_shipper = common::arg_value_set(defines::PAKKLY_CLI_INSTALLEXACT_SHIPPER);
    if specific_app.is_some() || specific_shipper.is_some() {
        return; //forced to update.
    }

    let last_time = local_data.last_launch;
    let systime = Utc::now().timestamp();

    let pakkly_threshold = (last_time + defines::PAKKLY_CACHE_SEC) > systime;
    if pakkly_threshold {
        //timers haven't elapsed yet.
        common::execute_program_and_terminate(local_data.to_owned());
    }
}
fn duplicate_process_hook(local_data: &mut StoredInstallData) {
    let running = ipc::enumerate();

    if running.is_err() {
        error!("Could not enumerate shippers with IPC!");
        error!("{:?}", running.unwrap_err());
        common::execute_program_and_terminate(local_data.to_owned());
    }
    let r_shippers = running.unwrap();
    info!("Found {} running shippers.", r_shippers.len());
    if r_shippers.len() > 0 {
        let quiet = common::arg_flag_set(defines::PAKKLY_CLI_UNINSTALL_QUIET);
        let regular = common::arg_flag_set(defines::PAKKLY_CLI_UNINSTALL);
        if quiet || regular {
            error!("Cannot uninstall while program is running!");
            webview_alert::alert("Program already running.", "Cannot uninstall while program is running!", None);
            common::exit(1);
        }
        info!("Found duplicate shipper of this app_id. Won't check for updates this run.");
        common::execute_program_and_terminate(local_data.to_owned());
    }
}
fn prints_exit_hooks() {
    if common::arg_flag_set(defines::PAKKLY_CLI_VERSION) {
        println!("{}", *defines::SHIPPER_VERSION_CLEAN);
        common::exit(0);
    }
    #[cfg(debug_assertions)]
    {
        if common::arg_flag_set(defines::PAKKLY_CLI_DEBUG_PRINTROOT) {
            println!("{}", paths::get_install_root().to_str().unwrap());
            common::exit(0);
        }
        if common::arg_flag_set(defines::PAKKLY_CLI_DEBUG_ISDUPLICATE) {
            let shippers = ipc::enumerate().unwrap();
            if shippers.len() > 0 {
                println!("DUPLICATE");
            } else {
                println!("NONDUP");
            }
            common::exit(0);
        }
    }
}
fn main() {
    let _ipc = &*defines::IPC_INFO;
    prints_exit_hooks();
    log::set_logger(&*common::LOGGER).map(|()| log::set_max_level(log::LevelFilter::Debug)).unwrap();
    std::panic::set_hook(Box::new(|pi| {
        emit_panic_error(&pi);

        //execute directly as we are guaranteed to be in the main thread:
        common::submit_critical_error();
        common::exit(1);
    }));

    info!("Install mode active: {}", *defines::FRESH_INSTALL);

    let mut local_data: StoredInstallData;

    if *defines::FRESH_INSTALL {
        uninstall_exit_hook(None);
        let mut new_data;
        loop {
            new_data = common::get_update_info(None, None, None);
            if new_data.is_err() {
                let install_quiet = common::arg_flag_set(defines::PAKKLY_CLI_INSTALL_QUIET);
                if install_quiet {
                    error!("Server could not be reached or returned malformed response!");
                    common::exit(1);
                }
                let retry = webview_alert::confirm(ConfirmParams {
                    title: "No Connection".into(),
                    body: "The server could not be reached, please check your internet connection and try again."
                        .into(),
                    image: webview_alert::ConfirmImage::NoInternet,
                    no_str: "Cancel".into(),
                    yes_str: "Retry".into(),
                });
                if !retry {
                    common::exit(1);
                }
            } else {
                break;
            }
        }
        local_data = StoredInstallData::from(new_data.unwrap()).unwrap();
    } else {
        local_data = unwrap_fe(StoredInstallData::read_json());

        duplicate_process_hook(&mut local_data);

        uninstall_exit_hook(Some(&mut local_data));

        install_indirect_exit_hook(&mut local_data);

        update_timer_exit_hook(&mut local_data);

        let new_data = common::get_update_info(Some(&local_data), None, None); //try fetching new update_info
        let systime = Utc::now().timestamp();
        local_data.last_launch = systime;
        if new_data.is_ok() {
            //if successful, replace the fields that are included verbatim from the server.
            //server is OK, continue update logic.

            local_data.fetched_meta = new_data.unwrap();
            common::warn_unwrap(local_data.write_json());
        } else {
            //server not reachable, don't check for updates
            error!("Server wasn't reachable or provided wrong data, falling back to executing program.");
            common::warn_unwrap(local_data.write_json());
            common::execute_program_and_terminate(local_data);
        }
    }
    force_update_hook(&mut local_data);

    if ipc::other_running().unwrap_or(false) {
        info!("Running process is blocking update, launching...");
    } else {
        install_quiet_exit_hook(&mut local_data);
        if should_app_update(&mut local_data) {
            let html_content = html_embed::UPDATER_UI;
            let should_download = Arc::new(Mutex::new(false));
            let thread_should = should_download.clone();
            let local_data_copy = local_data.clone();

            let wv = webview::Webview::new(html_content, &local_data, || {
                *should_download.lock().unwrap() = true;
            });
            let handle = wv.create_handle();
            let handle_mutex_t1 = Arc::new(Mutex::new(handle));
            let handle_mutex_t2 = handle_mutex_t1.clone();
            let handle_mutex_t3 = handle_mutex_t1.clone();
            let handle_mutex_panic = handle_mutex_t1.clone();
            let handle_mutex_netcrash = handle_mutex_t1.clone();
            std::panic::set_hook(Box::new(move |pi| {
                emit_panic_error(&pi);

                let h = handle_mutex_panic.lock().unwrap();
                let attempt = h.dispatch(move |a| {
                    *a.user_data_mut() = CrashState::FatalError;
                    a.exit();
                    return Ok(());
                });
                if attempt.is_err() {
                    //try one last time
                    common::submit_critical_error();
                    common::exit(1);
                }
            }));

            thread::spawn(move || loop {
                if *thread_should.lock().unwrap() {
                    info!("Threaded download starting!");
                    let install_status = installer::install(&mut local_data, move |progress, segment| {
                        webview::Webview::set_download_progress(&handle_mutex_t2.lock().unwrap(), segment, progress);
                    });
                    let mut cs = CrashState::NoError;
                    if install_status.is_err() {
                        let re = install_status.err().unwrap();
                        error!("{:?}", re);
                        if re.is_network_error {
                            cs = CrashState::NetworkError;
                        } else {
                            cs = CrashState::InstallFailed;
                        }
                    }
                    if *defines::FRESH_INSTALL {
                        thread::sleep(Duration::from_millis(300));
                        webview::Webview::set_download_progress(
                            &handle_mutex_t3.lock().unwrap(),
                            common::InstallProgressSegment::Installing,
                            1.0,
                        );
                        thread::sleep(Duration::from_millis(1500));
                    }
                    let h = &handle_mutex_netcrash.lock().unwrap();
                    h.dispatch(move |a| {
                        *a.user_data_mut() = cs;
                        a.exit();
                        return Ok(());
                    })
                    .unwrap();
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            });
            let val = wv.run().unwrap();

            if val == CrashState::UserAbort {
                common::exit(0);
            }
            //the gui has finished and contains the userData we set, representing the crashState
            if val == CrashState::FatalError || val == CrashState::InstallFailed {
                if !(*defines::FRESH_INSTALL) {
                    common::execute_program_and_terminate(local_data_copy);
                } else {
                    common::submit_critical_error();
                    common::exit(1);
                }
            }
            if val == CrashState::NetworkError {
                webview_alert::alert(
                    "Connection Interrupted",
                    "The server could not be reached, please check your internet connection and try again.",
                    None,
                );
                if !(*defines::FRESH_INSTALL) {
                    common::execute_program_and_terminate(local_data_copy);
                }
                common::exit(1);
            }
            if val == CrashState::NoError {
                //workaround for the fact that the shipperfile is not set yet during the first install.
                let freshest_data = unwrap_fe(StoredInstallData::read_json());
                common::execute_program_and_terminate(freshest_data);
            }
            unimplemented!();
        }

        selfupdate_hook(&mut local_data);
        repair_shipper_hook(&mut local_data);
    }
    common::execute_program_and_terminate(local_data);
}
