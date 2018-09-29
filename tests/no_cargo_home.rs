
use std::process::Command;

#[test]
fn no_cargo_home_dir() {
    // make sure cargo cache is built
    let cargo_build = Command::new("cargo").arg("build").output();
    assert!(cargo_build.is_ok(), "could not build cargo cache");
    let cargo_cache = Command::new("target/debug/cargo-cache")
        .env("CARGO_HOME", "./xyxyxxxyyyxxyxyxqwertywasd")
        .output();
    // make sure we failed
    let cmd = cargo_cache.unwrap();
    assert!(! cmd.status.success(), "no bad exit status!");

    // no stdout
    assert!(cmd.stdout.is_empty(), "unexpected stdout!");
    // stderr
    let stderr = String::from_utf8_lossy(&cmd.stderr).into_owned();
    assert!(!stderr.is_empty(),"found no stderr!");
    assert!(stderr.starts_with("Error, no cargo home path directory "));
    assert!(stderr.ends_with("./xyxyxxxyyyxxyxyxqwertywasd\' found.\n"));
}
