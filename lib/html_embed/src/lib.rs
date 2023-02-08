
macro_rules! include_html{
    ($a:expr) =>{
        include_str!(env!(concat!("COMPILED_HTML_",$a)))
    }
}
pub static CONFIRM:&str= include_html!("confirm");
pub static UPDATER_UI:&str= include_html!("updaterui");