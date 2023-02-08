use crate::{
    common, defines, ipc,
    logger::SimpleLogger,
    paths,
    remoteinstallerdata::{PakklyMetaRemote, StoredInstallData},
    shipperfile::InstanceMode,
    webview_alert::{self, ConfirmParams},
};
use crate::{defines::FRESH_INSTALL, fslog, shipperfile::Shipperfile};
use lazy_static::{__Deref, lazy_static};
use log::{info, warn};
use pakkly_error::FormattedError;
use serde::Serialize;
use std::{
    convert::From,
    fs::File,
    hash::Hasher,
    io::{BufWriter, Write},
};
use std::{fs, io::Read};
use std::{fs::DirEntry, process::Command};
use std::{
    io::BufReader,
    path::{Path, PathBuf},
    time::Duration,
};
use wyhash::WyHash;

lazy_static! {
    pub static ref LOGGER: SimpleLogger = SimpleLogger::new(1024 * 1024);
}
#[derive(Serialize, Clone, Debug)]
struct ErrorStruct {
    data: String,
    app_id: String,
    r#type: String,
    os: String,
    architecture: String,
    shipper_version: String,
}
pub fn submit_basic_crash(err: &str) {
    let es = ErrorStruct {
        app_id: (*defines::PAKKLY_ID_CLEAN).to_string(),
        r#type: "basic".to_string(),
        os: defines::OS_NAME.to_string(),
        architecture: defines::ARCH_NAME.to_string(),
        shipper_version: defines::SHIPPER_VERSION_CLEAN.to_string(),
        data: err.to_string(),
    };
    let js = serde_json::to_string(&es).unwrap_or("{\"data\":\"Cannot be serialized!\"}".to_string());
    info!("REQ POST {}", &defines::PAKKLY_CRASHLOG_URL.as_str());
    warn_unwrap(
        ureq::post(&defines::PAKKLY_CRASHLOG_URL)
            .timeout(Duration::from_secs(5))
            .set("Content-Type", "application/json")
            .send_string(&js),
    );
}
pub fn warn_unwrap<K, J>(x: Result<K, J>)
where
    J: std::fmt::Debug,
{
    if x.is_err() {
        let err = x.err().unwrap();
        warn!("{:?}", err);
    }
}
pub fn submit_critical_error() {
    let submit_error = webview_alert::confirm(ConfirmParams{
        title: "Critical Error".into(), 
        body:"Something has gone terribly wrong.\n\n Would you like to send a report to the developers so they can fix such issues?".into(),
        image: webview_alert::ConfirmImage::Error,
        no_str:"No".into(),
        yes_str:"Yes".into()
    });
    if submit_error {
        info!("VERSION: {}", defines::SHIPPER_VERSION_CLEAN.to_string(),);
        let rdata = &LOGGER.ring_display_and_close();
        let es = ErrorStruct {
            app_id: (*defines::PAKKLY_ID_CLEAN).to_string(),
            r#type: "dump".to_string(),
            os: defines::OS_NAME.to_string(),
            architecture: defines::ARCH_NAME.to_string(),
            shipper_version: defines::SHIPPER_VERSION_CLEAN.to_string(),
            data: hex::encode(rdata),
        };
        let js = serde_json::to_string(&es).unwrap_or("{\"data\":\"Cannot be serialized!\"}".to_string());
        info!("REQ POST {}", &defines::PAKKLY_CRASHLOG_URL.as_str());
        warn_unwrap(
            ureq::post(&defines::PAKKLY_CRASHLOG_URL)
                .timeout(Duration::from_secs(30))
                .set("Content-Type", "application/json")
                .send_string(&js),
        );
    }
}
pub fn get_file_hash(filepath: &PathBuf) -> Result<(u64, Vec<u8>), FormattedError> {
    let mut file = BufReader::new(fslog::file_open(filepath)?);
    let mut hasher = WyHash::with_seed(0);
    let mut file_buf: [u8; defines::FS_BUFFER_SIZE] = [0; defines::FS_BUFFER_SIZE];
    hasher.write("HASH_START".as_bytes());
    loop {
        let len = file.read(&mut file_buf)?;
        if len == 0 {
            break;
        }
        hasher.write(&file_buf[..len]);
    }
    let res = hasher.finish();
    drop(file);
    Ok((res, res.to_le_bytes().to_vec()))
}
pub fn get_standard_timeout() -> Duration {
    if *FRESH_INSTALL {
        return Duration::from_secs(60);
    } else {
        return Duration::from_secs(3);
    }
}
pub fn arg_flag_set(arg_query: &str) -> bool {
    let args = std::env::args();
    for arg in args {
        if arg == arg_query {
            #[cfg(debug_assertions)]
            {
                info!("flag_lookup {} = {}", arg_query, true);
            }
            return true;
        }
    }
    #[cfg(debug_assertions)]
    {
        info!("flag_lookup {} = {}", arg_query, false);
    }
    return false;
}
pub fn arg_value_set(arg_query: &str) -> Option<String> {
    let args = std::env::args();
    let mut arg_found = false;
    for arg in args {
        if arg_found {
            return Some(arg.to_string());
        }
        if arg == arg_query {
            arg_found = true; //next arg is value
        }
    }
    if arg_found {
        //wrong!!
        panic!("{} CLI needs a value!", arg_query);
    }
    return None;
}
/// The shipper is assumed to be the file with the shortest name in the folder.
pub fn find_shipper_in_folder(search_folder: &PathBuf) -> std::io::Result<Option<PathBuf>> {
    let mut ok_files: Vec<DirEntry> = fs::read_dir(search_folder)?.filter(|x| x.is_ok()).map(|x| x.unwrap()).collect();
    if ok_files.len() == 0 {
        return Ok(None);
    }
    ok_files.sort_unstable_by_key(|x| x.file_name().len());

    return Ok(Some(ok_files.first().unwrap().path()));
}
pub fn find_file_joined(gb: &str, search_folder: &PathBuf) -> Option<PathBuf> {
    let complete_path = search_folder.join(gb);
    if complete_path.exists() && complete_path.is_file() {
        return Some(complete_path);
    }
    warn!("Found nothing for path: {}", complete_path.to_str().unwrap());
    return None;
}
pub fn find_executable_path(
    params: &StoredInstallData,
    in_folder: Option<&PathBuf>,
) -> Result<Option<PathBuf>, FormattedError> {
    let def = paths::get_install_path();
    let search_folder = match in_folder.is_some() {
        true => in_folder.unwrap(),
        false => &def,
    };
    let pf = match &params.shipperfile {
        Some(pf_unwrapped) => Ok(pf_unwrapped),
        None => Err(FormattedError::from_str("find_executable_path called without pakklyfile!".to_string())),
    }?;
    return Ok(find_file_joined(&pf.program_path_to_binary, &search_folder));
}
pub fn all_absolute_files_in_folder_recursive(root: &PathBuf) -> Result<Vec<PathBuf>, FormattedError> {
    let entries = fs::read_dir(root)?;
    let mut ret: Vec<PathBuf> = vec![];
    for entry in entries {
        let entry_uw = entry?;
        let entry_path = entry_uw.path();
        if entry_path.is_file() {
            ret.push(entry_path);
        } else {
            ret.append(&mut (all_absolute_files_in_folder_recursive(&entry_path)?));
        }
    }
    return Ok(ret);
}
pub fn all_relative_files_in_folder_recursive(root: &PathBuf) -> Result<Vec<PathBuf>, FormattedError> {
    let relative: Vec<PathBuf> = all_absolute_files_in_folder_recursive(root)?
        .iter()
        .map(|x| x.strip_prefix(root).unwrap().to_path_buf())
        .collect();
    return Ok(relative);
}

pub fn execute_detached(executable: &PathBuf, args_param: &Vec<&str>) -> Result<(), FormattedError> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(&executable).arg("--args").args(args_param).spawn().unwrap();
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("CMD").arg("/C").arg("START").arg("/b").arg(executable).args(args_param).spawn().unwrap();
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("nohup").arg(executable).args(args_param).arg("&").spawn().unwrap();
    }
    return Ok(());
}
pub fn execute_program_and_terminate(local_data: StoredInstallData) -> ! {
    let pf = &local_data.shipperfile;
    let pf_unwrapped;
    if pf.is_none() {
        panic!("execute_program_and_terminate called without pakklyfile!");
    } else {
        pf_unwrapped = pf.as_ref().unwrap();
    }
    let ipc_entries = ipc::enumerate();
    let instance_mode = pf_unwrapped.instance_mode.as_ref().unwrap_or(&InstanceMode::multi_instance);
    if ipc_entries.is_err() {
        warn!("{:?}", ipc_entries.err().unwrap());
        if *instance_mode == InstanceMode::single_instance {
            webview_alert::alert(
                "App already running",
                format!("{} is already running!", local_data.fetched_meta.app_name).as_str(),
                None,
            );
            common::exit(0);
        }
    } else {
        let ipc_uw = ipc_entries.unwrap();
        if *instance_mode != InstanceMode::multi_instance && ipc_uw.len() > 0 {
            let focus_result = ipc::focus(&ipc_uw[0]);
            if focus_result.is_err() {
                if *instance_mode == InstanceMode::single_instance {
                    warn!("{:?}", focus_result.as_ref().err().unwrap());
                    webview_alert::alert(
                        "App already running",
                        format!("{} is already running!", local_data.fetched_meta.app_name).as_str(),
                        None,
                    );
                }
            }
            common::exit(0);
        }
    }
    let executable_path = find_executable_path(&local_data, None).unwrap().unwrap();
    //let executable_path_str = executable_path.to_string_lossy().to_string();
    let default_working_dir_str = executable_path.parent().unwrap().to_string_lossy().to_string();
    let default_args = Vec::new();
    let working_dir = pf_unwrapped.program_working_subdirectory.clone().unwrap_or(default_working_dir_str);
    let args = &pf_unwrapped.program_arguments.as_ref().unwrap_or(&default_args);
    info!("Launching: {}", executable_path.to_string_lossy());
    let mut client_program =
        Command::new(&executable_path).args(*args).current_dir(working_dir).envs(std::env::vars()).spawn().unwrap();

    warn_unwrap(defines::IPC_INFO.clear());
    let mark_ipc = ipc::IPCInfo::new(Some(client_program.id().to_string()));
    let res = client_program.wait();

    if mark_ipc.is_ok() {
        warn_unwrap(mark_ipc.unwrap().clear());
    }
    res.unwrap();
    exit(0);
}
pub fn get_update_info(
    current_data: Option<&StoredInstallData>,
    new_app: Option<String>,
    new_shipper: Option<String>,
) -> Result<PakklyMetaRemote, FormattedError> {
    let url = format!("{}/api/v1/shipper/info", defines::REMOTE_URL_CLEAN.deref());
    log::info!("Getting update info: {url}");
    let mut raw_req = ureq::request("GET", url.as_str())
        .timeout(get_standard_timeout())
        .query("app_id", &*defines::PAKKLY_ID_CLEAN)
        .query("platform_type", defines::PLATFORM_TYPE.to_string().as_str())
        .query("shipper_version", &*defines::SHIPPER_VERSION_CLEAN)
        .query("channel", &*defines::SHIPPER_CHANNEL_CLEAN);
    if current_data.is_some() {
        //add the current version
        raw_req = raw_req.query("app_version", &current_data.unwrap().installed_app_info.version)
    }
    if new_app.is_some() {
        //requesting specific app version
        raw_req = raw_req.query("new_app_version", new_app.unwrap().as_str())
    }
    if new_shipper.is_some() {
        //requesting specific shipper version
        raw_req = raw_req.query("new_shipper_version", new_shipper.unwrap().as_str())
    }
    let update_info_request = raw_req.call()?;
    let update_info_response = update_info_request.into_string()?;
    let update_info_parsed: PakklyMetaRemote = serde_json::from_str(&update_info_response)?;

    return Ok(update_info_parsed);
}
pub fn get_shipperfile(root: &PathBuf) -> Result<Shipperfile, FormattedError> {
    let mut path: PathBuf = PathBuf::from(root);
    path.push("shipperfile.json");
    if fslog::exists(&path) {
        let contents = fslog::read_to_string(path)?;
        let shipperfile: Shipperfile = serde_json::from_str(&contents)?;
        return Ok(shipperfile);
    } else {
        std::thread::sleep(Duration::from_secs(10000000));
        return Err(FormattedError::from_str("Missing shipperfile.json!".to_string()));
    }
}
pub fn is_hash_whitelisted(path: &PathBuf) -> bool {
    if path.to_string_lossy().contains("node_modules") {
        return true;
    }
    if path.starts_with(paths::get_install_dir_pakkly()) {
        return true;
    }
    return false;
}
pub fn random_hash() -> String {
    hex::encode(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
            .to_string()
            .as_bytes()
            .to_vec(),
    )
}

pub fn download_file<F>(url: &str, destination: &PathBuf, cb: F) -> Result<(), FormattedError>
where
    F: Fn(f32, InstallProgressSegment),
{
    info!("Downloading: {}", url);
    info!("REQ GET {}", url);
    let resp_raw = ureq::get(&url).call();
    if let Ok(resp) = resp_raw {
        let content_length_header = resp.header("Content-Length");
        let content_length: i64 = match content_length_header {
            Some(cl) => cl.parse().unwrap_or(0),
            None => 0,
        };
        let mut handle = resp.into_reader();
        if content_length == 0 {
            cb(-1.0, InstallProgressSegment::Downloading);
        } else {
            cb(0.0, InstallProgressSegment::Downloading);
        }
        {
            let mut buffer: Vec<u8> = Vec::new();
            buffer.resize(1024 * 1024 * 20, 0x00);
            let mut file = BufWriter::new(File::create(&destination)?);
            let mut written_bytes = 0;
            loop {
                let length = handle.read(buffer.as_mut_slice())?;
                if length == 0 {
                    break;
                }
                let wlen = file.write(&buffer.as_slice()[0..length])?;
                if wlen == 0 {
                    return Err(FormattedError::from(std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "Wrote 0 length!",
                    )));
                }
                written_bytes += wlen;
                if content_length != 0 {
                    cb(((written_bytes as f64) / (content_length as f64)) as f32, InstallProgressSegment::Downloading);
                }
            }

            file.flush()?;
        }
    } else {
        return Err(FormattedError::from_ureq(resp_raw.unwrap_err()));
    }
    return Ok(());
}

pub fn path_str<P: AsRef<Path>>(path: &P) -> String {
    return path.as_ref().to_string_lossy().to_string();
}
pub fn exit(code: i32) -> ! {
    warn_unwrap(defines::IPC_INFO.clear());
    std::process::exit(code);
}
#[cfg(target_os = "linux")]
pub fn untar(tar_file: &PathBuf, destination_path: &PathBuf) -> Result<(), FormattedError> {
    use tar::Archive;
    let mut ar = Archive::new(File::open(tar_file)?);
    ar.set_preserve_permissions(true);
    ar.unpack(destination_path)?;
    Ok(())
}
pub enum SceneID {
    InstallPrompt = 0,
    InstallProgress = 1,
}
pub enum InstallProgressSegment {
    Downloading = 0,
    Installing = 1,
}
#[derive(PartialEq)]
pub enum CrashState {
    UserAbort = 0,
    NoError = 1,
    FatalError = 2,
    NetworkError = 3,
    InstallFailed = 4,
}
