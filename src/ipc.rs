use std::{fs, path::PathBuf, process};

use pakkly_error::FormattedError;
use serde::{Deserialize, Serialize};
#[cfg(target_os = "windows")]
use windows_interface;

use crate::paths;
pub fn erase_all() -> Result<(), FormattedError> {
    let p = paths::get_ipc_dir();
    fs::remove_dir_all(p)?;
    Ok(())
}
pub fn other_running() -> Result<bool, FormattedError> {
    return Ok(enumerate()?.len() > 0);
}
pub fn focus(target: &ExternalIPCInfo) -> Result<(), FormattedError> {
    #[cfg(target_os = "macos")]
    {
        use crate::os_spec::mac;
        return mac::focus_app_by_pid(i64::from_str_radix(&target.pid, 10)?);
    }
    #[cfg(target_os = "windows")]
    {
        return windows_interface::focus_app_by_pid(u32::from_str_radix(&target.pid, 10)?);
    }
    #[cfg(target_os = "linux")]
    {
        return Err(pakkly_error::ferror!("No focus for OS {}", crate::defines::OS_NAME));
    }
}
fn running_map(pids: &Vec<i64>) -> Vec<bool> {
    #[cfg(not(target_os = "macos"))]
    {
        use sysinfo::{PidExt, ProcessRefreshKind, RefreshKind, System, SystemExt};
        let mut ret = Vec::new();
        ret.resize(pids.len(), false);
        let prk = ProcessRefreshKind::new();
        let rk = RefreshKind::new().with_processes(prk);
        let mut sys = System::new_with_specifics(rk);
        sys.refresh_processes_specifics(prk);
        for (pid, _process) in sys.processes() {
            let pid_64: i64 = pid.as_u32().into();
            let found_pid = pids.iter().position(|x| *x == pid_64);
            if found_pid.is_some() {
                ret[found_pid.unwrap()] = true;
            }
        }
        return ret;
    }

    #[cfg(target_os = "macos")]
    {
        use crate::os_spec::mac;
        let running_pids = mac::get_running_pids();
        return pids.iter().map(|x| running_pids.contains(x)).collect();
    }
}
///Returns a list of all running shippers with this Pakkly ID, excluding this one.
pub fn enumerate() -> Result<Vec<ExternalIPCInfo>, FormattedError> {
    let ipc_dir = paths::get_ipc_dir();
    if !ipc_dir.exists() {
        return Ok(vec![]);
    }
    let my_pid = process::id().to_string();
    let dir_entries = fs::read_dir(ipc_dir)?;
    let mut found_ipcs: Vec<i64> = vec![];
    for dirent in dir_entries {
        if dirent.is_err() {
            return Err(dirent.unwrap_err().into());
        }
        let dirent_uw = dirent.unwrap();
        let is_json = dirent_uw.file_name().to_string_lossy().ends_with(".json");
        if is_json {
            let file_content = fs::read(dirent_uw.path())?;
            let ipc_info: ExternalIPCInfo = serde_json::from_str(&String::from_utf8(file_content)?)?;
            if ipc_info.pid == my_pid {
                continue;
            }
            found_ipcs.push(i64::from_str_radix(&ipc_info.pid, 10)?);
        }
    }
    let only_running_mask = running_map(&found_ipcs);
    return Ok(found_ipcs
        .iter()
        .enumerate()
        .filter(|(i, _x)| only_running_mask[*i])
        .map(|(_i, x)| ExternalIPCInfo { pid: x.to_string() })
        .collect());
}
fn init() -> Result<(), FormattedError> {
    let ipc_dir = paths::get_ipc_dir();
    if ipc_dir.is_file() {
        fs::remove_file(&ipc_dir)?;
    }
    if !ipc_dir.exists() {
        fs::create_dir_all(&ipc_dir)?;
    }
    Ok(())
}
fn pid_to_file(pid: &str) -> PathBuf {
    return paths::get_ipc_dir().join(format!("{pid}.json"));
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ExternalIPCInfo {
    pid: String,
}
pub struct IPCInfo {
    ipc_data: ExternalIPCInfo,
}

impl IPCInfo {
    pub fn new(pid: Option<String>) -> Result<Self, FormattedError> {
        init()?;
        let pid_str = match pid {
            Some(x) => x,
            None => std::process::id().to_string(),
        };
        let pidfile = pid_to_file(&pid_str);
        let ipc_pid = Self { ipc_data: ExternalIPCInfo { pid: pid_str } };
        fs::write(pidfile, serde_json::to_string(&ipc_pid.ipc_data)?)?;
        return Ok(ipc_pid);
    }
    pub fn clear(&self) -> Result<(), FormattedError> {
        let target_file = pid_to_file(&self.ipc_data.pid);
        fs::remove_file(target_file)?;
        Ok(())
    }
}
