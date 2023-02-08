use crate::{ipc, remoteinstallerdata::StoredInstallData};
use lazy_static::lazy_static;

/// Shipper will not re-check for new update if it already checked for an updated in the last x seconds
pub static PAKKLY_CACHE_SEC: i64 = 60;

/// How much memory to use to buffer file writes
pub const FS_BUFFER_SIZE: usize = usize::pow(2, 16);

/// These constants allow other programs to specialize a compiled shipper without having to recompile it from source
/// Shipper expects to be edited and have these placeholder strings filled with the actual values, padded with NULL (\00)
#[used]
pub static PAKKLY_ID_DIRTY: &str = "EXAMPLE_PAKKLY_ID_VERYLONGEXAMPLE_PAKKLY_ID_VERYLONGEXAMPLE_0000";
#[used]
pub static SHIPPER_VERSION_DIRTY: &str = "SHIPPER_VERSION_SHIPPER_VERSION_SHIPPER_VERSION_SHIPPER_VERSION_";
#[used]
pub static SHIPPER_CHANNEL_DIRTY: &str = "SHIPPER_CHANNEL_SHIPPER_CHANNEL_SHIPPER_CHANNEL_SHIPPER_CHANNEL_";
#[used]
pub static REMOTE_URL_DIRTY: &str = "REMOTE_URL_REMOTE_URL_REMOTE_URL_REMOTE_URLREMOTE_URL_REMOTE_URL";

pub static PAKKLY_CLI_REPLACE_SHIPPER: &str = "--pakkly_install";
pub static PAKKLY_CLI_INSTALL_QUIET: &str = "--pakkly_install_quiet";
pub static PAKKLY_CLI_UNINSTALL: &str = "--pakkly_uninstall";
pub static PAKKLY_CLI_UNINSTALL_QUIET: &str = "--pakkly_uninstall_quiet";
pub static PAKKLY_CLI_NOROOT: &str = "--pakkly_noroot";
pub static PAKKLY_CLI_VERSION: &str = "--pakkly_version";
pub static PAKKLY_CLI_INSTALLEXACT_SHIPPER: &str = "--pakkly_installexact_shipper";
pub static PAKKLY_CLI_INSTALLEXACT_APP: &str = "--pakkly_installexact_app";
#[cfg(debug_assertions)]
pub static PAKKLY_CLI_DEBUG_PRINTROOT: &str = "--pakkly_debug_printroot";
#[cfg(debug_assertions)]
pub static PAKKLY_CLI_DEBUG_ISDUPLICATE: &str = "--pakkly_debug_isduplicate";
pub static HASH_ALWAYS_REPLACE: &str = "ALWAYS";

pub static HASH_DIRECTORY: &str = "DIRECTORY";
pub static HASH_DEFER: &str = "DEFER";

#[cfg(target_os = "windows")]
pub static OS_NAME: &str = "windows";
#[cfg(target_os = "macos")]
pub static OS_NAME: &str = "macos";
#[cfg(target_os = "linux")]
pub static OS_NAME: &str = "linux";

#[cfg(target_arch = "x86")]
pub static ARCH_NAME: &str = "x86";
#[cfg(target_arch = "x86_64")]
pub static ARCH_NAME: &str = "x86_64";
#[cfg(all(target_arch = "aarch64"))]
pub static ARCH_NAME: &str = "arm_64";

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub static PLATFORM_TYPE: i32 = 0;
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub static PLATFORM_TYPE: i32 = 1;
#[cfg(all(target_os = "windows", target_arch = "aarch64"))]
pub static PLATFORM_TYPE: i32 = 2;
#[cfg(all(target_os = "linux", target_arch = "x86"))]
pub static PLATFORM_TYPE: i32 = 3;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub static PLATFORM_TYPE: i32 = 4;
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub static PLATFORM_TYPE: i32 = 5;
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub static PLATFORM_TYPE: i32 = 6;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub static PLATFORM_TYPE: i32 = 7;

lazy_static! {
    pub static ref IPC_INFO: ipc::IPCInfo = ipc::IPCInfo::new(None).unwrap();
    pub static ref PAKKLY_ID_CLEAN: String = PAKKLY_ID_DIRTY.replace("\0", "");
    pub static ref SHIPPER_VERSION_CLEAN: String = SHIPPER_VERSION_DIRTY.replace("\0", "");
    pub static ref SHIPPER_CHANNEL_CLEAN: String = SHIPPER_CHANNEL_DIRTY.replace("\0", "");
    pub static ref REMOTE_URL_CLEAN: String = REMOTE_URL_DIRTY.replace("\0", "");
    pub static ref PAKKLY_CRASHLOG_URL: String = format!("{}/api/v1/shipper/error", REMOTE_URL_CLEAN.deref());
    pub static ref UNINSTALL_REGKEY: String =
        format!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\pakked_{}", *PAKKLY_ID_CLEAN);
    pub static ref FRESH_INSTALL: bool = StoredInstallData::read_json().is_err();
}
