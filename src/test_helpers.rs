pub(crate) fn bin_path() -> String {
    let string = if cfg!(release) {
        String::from("target/release/cargo-cache")
    } else {
        String::from("target/debug/cargo-cache")
    };

    if !std::path::PathBuf::from(&string).is_file() {
        panic!("executable '{}' not found!", string);
    }
    string
}
