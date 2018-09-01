use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

mod version;

fn main() {
    // generate version info from git hashes
    let version = format!("{}", version::VersionInfo::new());

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    File::create(out_dir.join("commit-info.txt"))
        .unwrap()
        .write_all(version.as_bytes())
        .unwrap();
}
