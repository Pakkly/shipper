use lazy_static::lazy_static;
use web_view::*;

lazy_static! {
    static ref HTML_CONTENT_CONFIRM: &'static str = html_embed::CONFIRM;
}
pub struct ConfirmParams {
    pub title: String,
    pub body: String,
    pub image: ConfirmImage,
    pub yes_str: String,
    pub no_str: String,
}
#[derive(Copy, Clone)]
pub enum ConfirmImage {
    NoInternet = 0,
    Error = 1,
    Question = 2,
}
pub fn alert(title: &str, body: &str, img: Option<ConfirmImage>) -> bool {
    return confirm(ConfirmParams {
        title: title.to_owned(),
        body: body.to_owned(),
        image: img.unwrap_or(ConfirmImage::Error),
        yes_str: "Ok".to_string(),
        no_str: "".to_string(),
    });
}
pub fn confirm(params: ConfirmParams) -> bool {
    let mut wv = web_view::builder()
        .title("")
        .content(Content::Html(HTML_CONTENT_CONFIRM.to_string()))
        .size(280, 350)
        .resizable(false)
        .frameless(cfg!(windows))
        .user_data(false)
        .invoke_handler(move |_webview, arg| {
            match arg {
                "init" => {
                    _webview
                        .eval(&format!(
                            "showAlert({:?},{:?},{},{:?},{:?})",
                            params.title, params.body, params.image as i32, params.yes_str, params.no_str
                        ))
                        .unwrap();
                    return Ok(());
                }
                "yes" => {
                    *_webview.user_data_mut() = true;
                    _webview
                        .handle()
                        .dispatch(move |exit_wv| {
                            exit_wv.exit();
                            return Ok(());
                        })
                        .unwrap();
                    return Ok(());
                }
                "no" => {
                    *_webview.user_data_mut() = false;
                    _webview
                        .handle()
                        .dispatch(move |exit_wv| {
                            exit_wv.exit();
                            return Ok(());
                        })
                        .unwrap();
                    return Ok(());
                }
                _ => unimplemented!(),
            };
        })
        .build()
        .unwrap();
    wv.set_color(Color { r: 0x2f, g: 0x2f, b: 0x2f, a: 0xff });
    return wv.run().unwrap();
}
