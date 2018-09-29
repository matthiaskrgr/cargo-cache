#[path = "../src/test_helpers.rs"]
mod test_helpers;

use crate::test_helpers::bin_path;
use std::process::Command;

#[test]
fn no_cargo_home_dir() {
    let cargo_cache = Command::new(bin_path())
        .env("CARGO_HOME", "./xyxyxxxyyyxxyxyxqwertywasd")
        .output();
    // make sure we failed
    let cmd = cargo_cache.unwrap();
    assert!(!cmd.status.success(), "no bad exit status!");

    // no stdout
    assert!(cmd.stdout.is_empty(), "unexpected stdout!");
    // stderr
    let stderr = String::from_utf8_lossy(&cmd.stderr).into_owned();
    assert!(!stderr.is_empty(), "found no stderr!");
    assert!(stderr.starts_with("Error, no cargo home path directory "));
    assert!(stderr.ends_with("./xyxyxxxyyyxxyxyxqwertywasd\' found.\n"));
}
