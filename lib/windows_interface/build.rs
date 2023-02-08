// build.rs
use cc;

fn main() {
    cc::Build::new()
        .cpp(true)
        .define("UNICODE", None)
        .define("_UNICODE", None)
        .flag("/std:c++17")
        .file("src/windows_lib/winapi_tools.cpp")
        .compile("winapi_tools");
}
