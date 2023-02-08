use std::{
    sync::Mutex,
    time::{Duration, SystemTime},
};

use crate::common::SceneID;
use crate::remoteinstallerdata::StoredInstallData;
use crate::{
    common::{self, InstallProgressSegment},
    defines,
};
use common::CrashState;
use lazy_static::lazy_static;
use web_view::*;
pub struct Webview<'a> {
    webview: WebView<'a, CrashState>,
}
lazy_static! {
    static ref LAST_UPDATED_PROGRESS: Mutex<SystemTime> = Mutex::new(SystemTime::now());
}
impl<'a> Webview<'a> {
    pub fn new<F: 'a>(html_content: &str, remote_data: &StoredInstallData, start_download: F) -> Webview<'a>
    where
        F: Fn(),
    {
        let rdata_copy = remote_data.clone();
        let mut wv = web_view::builder()
            .title("")
            .content(Content::Html(html_content))
            .size(230, 350)
            .resizable(false)
            .frameless(cfg!(windows))
            .user_data(CrashState::UserAbort)
            .invoke_handler(move |_webview, arg| {
                match arg {
                    "download" => {
                        set_download_progress_direct(_webview, InstallProgressSegment::Downloading, 0.0).unwrap();
                        set_scene_direct(
                            _webview,
                            SceneID::InstallProgress,
                            (&rdata_copy.fetched_meta.app_name).to_string(),
                        )
                        .unwrap();
                        start_download();
                        return Ok(());
                    }
                    "init" => {
                        _webview.inject_css(&"").unwrap();
                        if *defines::FRESH_INSTALL {
                            set_scene_direct(
                                _webview,
                                SceneID::InstallPrompt,
                                (&rdata_copy.fetched_meta.app_name).to_string(),
                            )
                            .unwrap();
                        } else {
                            set_scene_direct(
                                _webview,
                                SceneID::InstallProgress,
                                (&rdata_copy.fetched_meta.app_name).to_string(),
                            )
                            .unwrap();
                            start_download();
                        }
                        return Ok(());
                    }
                    _ => unimplemented!(),
                };
            })
            .build()
            .unwrap();
        wv.set_color(remote_data.background_color);
        return Webview { webview: wv };
    }
    pub fn create_handle(&self) -> Handle<CrashState> {
        return self.webview.handle();
    }

    pub fn run(self) -> WVResult<CrashState> {
        self.webview.run()
    }
    /*pub fn set_scene(handle: &Handle<i32>,scene_id: SceneID){
        handle.dispatch(move |webview| {
            set_scene_direct(webview,scene_id)
        }).unwrap();
    }*/
    pub fn set_download_progress(handle: &Handle<CrashState>, segment: InstallProgressSegment, progress: f32) {
        let elapsed = LAST_UPDATED_PROGRESS.lock().unwrap().elapsed();
        if progress != 1.0 && elapsed.is_ok() && elapsed.unwrap() < Duration::from_millis(200) {
            return;
        } else {
            *LAST_UPDATED_PROGRESS.lock().unwrap() = std::time::SystemTime::now();
        }
        handle.dispatch(move |webview| set_download_progress_direct(webview, segment, progress)).unwrap();
    }
}
fn set_download_progress_direct(
    webview: &mut WebView<CrashState>,
    segment: InstallProgressSegment,
    progress: f32,
) -> WVResult {
    webview.eval(&format!("setDownloadProgress({},{})", segment as i32, progress))
}
fn set_scene_direct(webview: &mut WebView<CrashState>, scene_id: SceneID, app_name: String) -> WVResult {
    webview.eval(&format!("setSceneID({},\"{}\")", scene_id as i32, app_name.replace("\"", "\\\"")))
}
