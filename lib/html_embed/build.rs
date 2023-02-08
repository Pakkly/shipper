// build.rs
use duct::cmd;
use pakkly_error::FormattedError;
use pathdiff::diff_paths;
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
    let out_folder = outfolder_env_var.unwrap_or("generated".to_string()).to_owned();
    let html_folder = "src/resources/html";

    {
        let current_dir = std::env::current_dir()?; //
        std::env::set_current_dir("../../build_tools/inline_source/")?;
        let npm_i_status;
        #[cfg(target_os = "windows")]
        {
            npm_i_status = std::process::Command::new("cmd").args(&["/c", "npm", "i"]).status()?;
        }
        #[cfg(not(target_os = "windows"))]
        {
            npm_i_status = std::process::Command::new("npm").arg("i").status()?;
        }
        std::env::set_current_dir(current_dir)?;
        assert!(npm_i_status.success());
    }

    let prefix = "../../build_tools/inline_source/";
    let inline_source_path = PathBuf::from(prefix).canonicalize().unwrap();
    let previous_directory = std::env::current_dir().unwrap();

    fs::remove_dir_all(&out_folder)?;
    fs::create_dir_all(&out_folder)?;

    let npm_path = which::which("npm").unwrap();
    for t in fs::read_dir(html_folder)? {
        let entry = t?;
        let mut embed_root = entry.path();
        if embed_root.is_dir() {
            let root_path_rel = diff_paths(embed_root.canonicalize().unwrap(), &inline_source_path).unwrap();
            let basename = root_path_rel.file_name().unwrap().to_str().unwrap();
            embed_root.push("index.html");
            let index_path = diff_paths(embed_root.canonicalize().unwrap(), &inline_source_path).unwrap();
            let index_path_str = index_path.to_str().unwrap();
            let destination_html_path: PathBuf = [&out_folder, &[basename, ".html"].concat()].iter().collect();
            println!("Writing to: {:?}", &destination_html_path.to_str().unwrap());
            println!("Inlining {:?}...", &basename);
            println!("{}", &index_path_str);
            std::env::set_current_dir(prefix)?;
            exec(
                Some(&destination_html_path),
                &npm_path.to_str().unwrap(),
                &[
                    "run",
                    "--quiet",
                    "--silent",
                    "dev",
                    "--",
                    "--entrypoint",
                    index_path_str,
                    "--root",
                    &root_path_rel.to_str().unwrap(),
                ],
            )?;
            std::env::set_current_dir(&previous_directory)?;
            println!("Target: {:?}", &index_path_str);
            println!("Inlined to {:?}", &destination_html_path);
            //fs::write(&destination_html_path, &contents)?;
            println!("cargo:rustc-env=COMPILED_HTML_{}={}", &basename, &destination_html_path.to_str().unwrap())
        }
    }

    println!("cargo:rustc-cfg=COMPILED_HTML");
    println!("cargo:rerun-if-changed={}", &html_folder);
    return Ok(());
}
