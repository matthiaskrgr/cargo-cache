use std::path::PathBuf;
use std::process::Command;

#[test]
fn no_cargo_home_dir() {
    let debug_build = PathBuf::from("target/debug/").is_dir();

    // make sure cargo cache is built
    let cargo_build = if debug_build {
        Command::new("cargo").arg("build").output()
    } else {
        Command::new("cargo").arg("build").arg("--release").output()
    };
    assert!(cargo_build.is_ok(), "could not build cargo cache");
    let cargo_cache = Command::new(if debug_build {
        "target/debug/cargo-cache"
    } else {
        "target/release/cargo-cache"
    })
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
