use backtrace::Backtrace;
use std::boxed::Box;
use std::error::Error;
use std::fmt::{Debug, Formatter};

pub struct FormattedError {
    err: Box<dyn Error>,
    msg: String,
    stacktrace: String,
    pub is_network_error: bool,
    pub is_missing_sudo: bool,
}
impl Debug for FormattedError {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        if self.msg != "" {
            write!(fmt, "ERROR: {:?} \n\n\nSTACK TRACE: \n{}", &self.msg, &self.stacktrace)
        } else {
            write!(fmt, "ERROR: {:?} \n\n\nSTACK TRACE: \n{}", &self.err, &self.stacktrace)
        }
    }
}
impl Default for FormattedError {
    fn default() -> Self {
        FormattedError {
            err: Box::from(std::io::Error::new(std::io::ErrorKind::Other, "")),
            msg: "".to_string(),
            stacktrace: format!("{:?}", Backtrace::new()),
            is_network_error: false,
            is_missing_sudo: false,
        }
    }
}
impl<T> From<T> for FormattedError
where
    T: Error + 'static,
{
    fn from(err: T) -> Self {
        FormattedError { err: Box::from(err), ..Default::default() }
    }
}
impl FormattedError {
    pub fn from_str(val: String) -> Self {
        FormattedError { msg: val, ..Default::default() }
    }
    pub fn from_ureq(err: ureq::Error) -> Self {
        Self { err: Box::from(err), is_network_error: true, ..Default::default() }
    }
    pub fn from_missing_sudo(err: String) -> Self {
        FormattedError { is_missing_sudo: true, msg: err, ..Default::default() }
    }
}
#[macro_export(local_inner_macros)]
macro_rules! ferror {
    ($($arg:tt)+) => {
        FormattedError::from_str(std::format!($($arg)+).to_string())
    }

}
