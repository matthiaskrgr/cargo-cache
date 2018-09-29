use std::path::PathBuf;

#[allow(dead_code)]
pub(crate) fn bin_path() -> String {
    if PathBuf::from("target/release/cargo-cache").is_file() {
        String::from("target/release/cargo-cache")
    } else if PathBuf::from("target/debug/cargo-cache").is_file() {
        String::from("target/debug/cargo-cache")
    } else {
        panic!("No cargo-cache executable found!");
    }
}
