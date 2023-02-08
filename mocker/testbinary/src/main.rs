#![windows_subsystem = "windows"]
use std::{thread, time};
use web_view::*;
fn main() {
    let l1 = "This is an example application, suitable for testing Pakkly.";
    let l2 = format!("Working Directory: {:?}", std::env::current_dir().unwrap());
    let l3 = format!("Arguments: {:?}", std::env::args());
    let html_content = format!("<body style='background-color:black; color: #00FF00'><div style='text-align:center'>{}<br><br>{}<br>{}</div></body>",l1,l2,l3);

    let mut wv = web_view::builder()
        .title("")
        .content(Content::Html(html_content))
        .size(800, 350)
        .resizable(true)
        .debug(true)
        .user_data(())
        .frameless(false)
        .invoke_handler(|_webview, _arg| Ok(()))
        .build()
        .unwrap();
    wv.set_color(Color { r: 0x2f, g: 0x2f, b: 0x2f, a: 0xff });
    let handle = wv.handle();
    thread::spawn(move || {
        let sleeptime = time::Duration::from_secs(
            u64::from_str_radix(&std::env::var("SHIPPER_TEST_APP_EXIT_AFTER").unwrap_or("50000".to_string()), 10)
                .unwrap(),
        );
        let url = format!("{}/app_ping", std::env::var("SHIPPER_TEST_APP_REMOTE_URL"));

        let _e = ureq::get(&url).call();
        thread::sleep(sleeptime);
        handle
            .dispatch(|x| {
                x.exit();
                Ok(())
            })
            .unwrap();
    });
    wv.run().unwrap();
}
