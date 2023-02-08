// build.rs
use duct::cmd;
use pakkly_error::FormattedError;
use std::fs;
use std::path::PathBuf;

fn emit_panic_error(pi: &std::panic::PanicInfo<'_>) {
    let mut error_msg = "".to_string();
    if let Some(msg) = pi.payload().downcast_ref::<&str>() {
        error_msg = format!("Panic Message: {:?}", msg);
    }
    if let Some(location) = pi.location() {
        error_msg =
            format!("{}\nPanic location:  {:?} at line {:?}", error_msg, location.file(), location.line()).to_string();
    } else {
        error_msg += "\nPanic in unknown file";
    }
    println!("{}", error_msg);
}
fn main() {
    std::panic::set_hook(Box::new(|pi| {
        emit_panic_error(&pi);
        std::process::exit(1);
    }));
    let err = body();
    if err.is_err() {
        let err_unwrapped = err.unwrap_err();
        println!("ERROR! {:?}", err_unwrapped);
        std::process::exit(1);
    }
}
fn exec(stdout_path: Option<&PathBuf>, command: &str, args: &[&str]) -> Result<String, FormattedError> {
    let output = match stdout_path {
        Some(p) => cmd(command, args).stdout_path(p.to_str().unwrap()).read(),
        None => cmd(command, args).read(),
    }?;
    println!(
        "Wrote results of {:?} to {}",
        command,
        match stdout_path {
            Some(p) => p.to_str().unwrap(),
            None => "MEMORY",
        }
    );
    return Ok(output);
}
fn body() -> Result<(), FormattedError> {
    let outfolder_env_var = std::env::var("OUT_DIR");
    let out_folder = PathBuf::from(outfolder_env_var.unwrap_or("generated".to_string()).to_owned());

    let script = PathBuf::from("src/licensor/index.js".to_string());

    println!("{:?}", &out_folder);

    fs::remove_dir_all(&out_folder)?;
    fs::create_dir_all(&out_folder)?;

    let out_file = out_folder.join("licenses.txt");
    let node_path;
    #[cfg(target_os = "linux")]
    {
        node_path = exec(None, "/bin/bash", &["-c", "which node"])?;
        println!("Found nodejs at: {}", node_path);
    }
    #[cfg(not(target_os = "linux"))]
    {
        node_path = "node".to_string()
    }
    exec(None, &node_path, &[script.to_str().unwrap(), out_file.to_str().unwrap()])?;
    println!("cargo:rustc-env=COMPILED_LICENSE={}", out_file.to_str().unwrap());
    println!("cargo:rerun-if-changed=../../Cargo.toml");
    return Ok(());
}
