// note: to make debug prints work:
// cargo test -- --nocapture
#[path = "../src/test_helpers.rs"] mod test_helpers;

use crate::test_helpers::bin_path;
use std::path::*;
use std::process::Command;
use walkdir::WalkDir;

#[test]
fn build_and_check_size_test() {
    // move into the directory of our dummy crate
    // set a fake CARGO_HOME and build the dummy crate there
    let crate_path = PathBuf::from("tests/size_test/");
    let fchp = "target/fake_cargo_home"; // cake cargo_home path
    let status = Command::new("cargo")
        .arg("check")
        .arg("--target-dir")
        .arg("../../target/size_test/")
        .current_dir(&crate_path)
        .env("CARGO_HOME", "../../target/fake_cargo_home")
        .output();
    // make sure the build succeeded
    assert!(status.is_ok(), "build of dummy crate did not succeed");
    assert!(
        PathBuf::from(&fchp).is_dir(),
        "fake cargo home was not created!"
    );
    // make sure the size of the registry matches and we have 4 entries
    let mut registry_cache_path = PathBuf::from(&fchp);
    registry_cache_path.push("registry");
    registry_cache_path.push("cache");
    assert!(registry_cache_path.is_dir(), "no registry cache found");

    let mut filenames = WalkDir::new(registry_cache_path)
        .min_depth(2)
        .into_iter()
        .map(|dir| dir.unwrap().path().file_name().unwrap().to_owned())
        .collect::<Vec<_>>();
    filenames.sort();

    // make sure the filenames all match
    assert!(filenames.len() == 4);

    assert_eq!(
        filenames,
        [
            "cc-1.0.18.crate",
            "libc-0.2.42.crate",
            "pkg-config-0.3.12.crate",
            "unicode-xid-0.0.4.crate"
        ]
    );

    // run it on the fake cargo cache dir
    let cargo_cache = Command::new(bin_path()).env("CARGO_HOME", &fchp).output();
    assert!(cargo_cache.is_ok(), "cargo cache failed to run");
    let cc_output = String::from_utf8_lossy(&cargo_cache.unwrap().stdout).into_owned();
    // we need to get the actual path to fake cargo home dir and make it an absolute path
    let absolute_fchp = PathBuf::from(&fchp).canonicalize().unwrap();
    let mut desired_output = format!("Cargo cache '{}':\n\n", absolute_fchp.display());

    //@TODO this won't work because git repo is constantly growing

    desired_output.push_str(
        "\nTotal size:                   120.39 MB
Size of 0 installed binaries:     0 B
Size of registry:                  120.39 MB
Size of registry crate cache:           407.94 KB
Size of registry source checkouts:      2.04 MB
Size of git db:                    0 B
Size of git repo checkouts:        0 B\n",
    );
    // make sure the sizes match
    // @TODO make this compare texts again, not just lines
    assert_eq!(desired_output.lines().count(), cc_output.lines().count());
}
