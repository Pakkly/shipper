use pakkly_error::{ferror, FormattedError};
use std::ffi::CString;
use std::os::raw::c_char;
use std::os::raw::c_int;
use std::os::raw::c_ulong;
extern "C" {
    pub fn CreateLink(
        target: *const c_char,
        link_file_path: *const c_char,
        icon_path: *const c_char,
        args: *const c_char,
        description: *const c_char,
        admin: c_char,
    ) -> c_int;
    pub fn FocusPID(pid: c_ulong) -> c_int;
    pub fn ScheduleFileDelete(target: *const c_char) -> c_int;
}
pub fn make_lnk(
    link_to: &std::path::Path,
    write_file: &std::path::Path,
    link_icon_file: &std::path::Path,
    args: &str,
    admin: bool,
) -> Result<bool, FormattedError> {
    let c_target = CString::new(link_to.to_str().unwrap()).unwrap();
    let c_writefile = CString::new(write_file.to_str().unwrap()).unwrap();
    let c_description = CString::new("").unwrap();
    let c_icon_path = CString::new(link_icon_file.to_str().unwrap()).unwrap();
    let args = CString::new(args).unwrap();
    unsafe {
        return Ok(CreateLink(
            c_target.as_ptr(),
            c_writefile.as_ptr(),
            c_icon_path.as_ptr(),
            args.as_ptr(),
            c_description.as_ptr(),
            admin as i8,
        ) == 0);
    }
}
pub fn schedule_file_delete(target: &std::path::Path) -> Result<(), FormattedError> {
    let c_target = CString::new(target.to_str().unwrap()).unwrap();
    let ret;
    unsafe {
        ret = ScheduleFileDelete(c_target.as_ptr());
    }
    if ret == 0 {
        //success
        return Ok(());
    } else {
        return Err(FormattedError::from_str(format!("MoveFileExW returned {}", ret)));
    }
}
pub fn focus_app_by_pid(pid: u32) -> Result<(), FormattedError> {
    let ret;
    unsafe {
        ret = FocusPID(pid);
    }
    return match ret {
        0 => Ok(()),
        1 => Err(ferror!("NOT FOUND(1)")),
        2 => Err(ferror!("NO FOCUS(2)")),
        _ => Err(ferror!("NO FOCUS(?)")),
    };
}
