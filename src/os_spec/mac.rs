use objc::{class, msg_send, runtime::Object, sel, sel_impl};
use pakkly_error::{ferror, FormattedError};

static NSAPPLICATION_ACTIVATE_ALL_WINDOWS: u64 = 1 << 0;
static NSAPPLICATION_ACTIVATE_IGNORING_OTHER_APPS: u64 = 1 << 1;
pub fn get_running_pids() -> Vec<i64> {
    let mut pids = vec![];
    unsafe {
        let cls = class!(NSWorkspace);
        let my_workspace: *const Object = msg_send![cls, sharedWorkspace];
        let running_processes: *const Object = msg_send![my_workspace, runningApplications];
        let app_count: usize = msg_send![running_processes, count];
        for i in 0..app_count {
            let current_app: *const Object = msg_send![running_processes, objectAtIndex: i];
            let pid: i64 = msg_send![current_app, processIdentifier];
            pids.push(pid);
        }
    }
    return pids;
}
pub fn focus_app_by_pid(pid: i64) -> Result<(), FormattedError> {
    unsafe {
        let cls = class!(NSWorkspace);
        let my_workspace: *const Object = msg_send![cls, sharedWorkspace];
        let running_processes: *const Object = msg_send![my_workspace, runningApplications];
        let app_count: usize = msg_send![running_processes, count];
        for i in 0..app_count {
            let current_app: *const Object = msg_send![running_processes, objectAtIndex: i];
            let current_pid: i64 = msg_send![current_app, processIdentifier];
            if pid == current_pid {
                //this is the app!
                let is_active: bool = msg_send![current_app, isActive];
                if is_active {
                    return Ok(());
                }
                let active_success: bool = msg_send![
                    current_app,
                    activateWithOptions: NSAPPLICATION_ACTIVATE_ALL_WINDOWS
                        | NSAPPLICATION_ACTIVATE_IGNORING_OTHER_APPS
                ];
                return match active_success {
                    true => Ok(()),
                    false => Err(ferror!("NSRunningApplication.activateWithOptions FAILED!")),
                };
            }
        }
    }
    return Err(ferror!("Focus target not found!"));
}
