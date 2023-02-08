// build.rs
fn main() {
    #[cfg(target_os = "windows")]
    {
        //
        embed_resource::compile("data.rc"); //
    }
}
