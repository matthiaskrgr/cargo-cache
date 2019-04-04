// Copyright 2017-2019 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[path = "../src/test_helpers.rs"]
mod test_helpers;

use std::fs::{create_dir_all, File};
use std::path::*;
use std::process::Command;

use crate::test_helpers::bin_path;

use regex::Regex;
use walkdir::WalkDir;

#[test]
fn build_and_check_size_test() {
    // move into the directory of our dummy crate
    // set a fake CARGO_HOME and build the dummy crate there
    let crate_path = PathBuf::from("tests/size_test/");
    let fchp = "target/spurious_files_test"; // fake cargo_home path
    let status = Command::new("cargo")
        .arg("check")
        .arg("--target-dir")
        .arg("../../target/spurious_files_test/")
        .current_dir(&crate_path)
        .env("CARGO_HOME", "../../target/spurious_files_test")
        .output();
    // make sure the build succeeded
    assert!(status.is_ok(), "build of dummy crate did not succeed");
    assert!(
        PathBuf::from(&fchp).is_dir(),
        "fake cargo home was not created!"
    );
    // add some files here and there
    File::create("target/spurious_files_test/file1.txt").unwrap();
    File::create("target/spurious_files_test/file2").unwrap();
    File::create("target/spurious_files_test/registry/file3.ogg").unwrap();

    File::create("target/spurious_files_test/registry/cache/file4").unwrap();
    File::create("target/spurious_files_test/registry/index/file5").unwrap();
    File::create("target/spurious_files_test/registry/src/file6").unwrap();

    File::create("target/spurious_files_test/registry/cache/github.com-1ecc6299db9ec823/file7")
        .unwrap();
    File::create("target/spurious_files_test/registry/index/github.com-1ecc6299db9ec823/file8")
        .unwrap();
    File::create("target/spurious_files_test/registry/src/github.com-1ecc6299db9ec823/file9")
        .unwrap();
    create_dir_all("target/spurious_files_test/registry/cache/github.com-1ecc6299db9ec823/foo_dir")
        .unwrap();

    // make sure the size of the registry matches and we have 4 entries
    let mut registry_pkg_cache_path = PathBuf::from(&fchp);
    registry_pkg_cache_path.push("registry");
    registry_pkg_cache_path.push("cache");
    assert!(registry_pkg_cache_path.is_dir(), "no registry cache found");

    let mut filenames = WalkDir::new(registry_pkg_cache_path)
        .min_depth(2)
        .into_iter()
        .map(|dir| dir.unwrap().path().file_name().unwrap().to_owned())
        .collect::<Vec<_>>();
    filenames.sort();

    // make sure the filenames all match
    assert_eq!(filenames.len(), 6);

    assert_eq!(
        filenames,
        [
            "cc-1.0.18.crate",
            "file7",   // new
            "foo_dir", // new
            "libc-0.2.42.crate",
            "pkg-config-0.3.12.crate",
            "unicode-xid-0.0.4.crate"
        ]
    );

    // run it on the fake cargo cache dir
    let cargo_cache = Command::new(bin_path()).env("CARGO_HOME", &fchp).output();
    assert!(cargo_cache.is_ok(), "cargo cache failed to run");
    let cmd_out = &cargo_cache.unwrap();
    let cc_output = String::from_utf8_lossy(&cmd_out.stdout).into_owned();
    // we need to get the actual path to fake cargo home dir and make it an absolute path
    let mut desired_output = String::from("Cargo cache .*spurious_files_test.*:\n\n");

    /*
    Cargo cache '...cargo-cache/target/fake_cargo_home/':

    Total size:                             21.96 MB
    Size of 0 installed binaries:             0 B
    Size of registry:                         21.96 MB
    Size of 4 crate archives:                   407.74 KB
    Size of 4 crate source checkouts:           2.04 MB
    Size of git db:                           0 B
    Size of 0 bare git repos:                   0 B
    Size of 0 git repo checkouts:               0 B
    */

    desired_output.push_str(
        "Total size:          .*MB
Size of .* installed binaries:  .*B
Size of registry:           .*MB
Size of registry index:       .*MB
Size of .* crate archives:       .*KB
Size of .* crate source checkouts:  .*MB
Size of git db:              .*B
Size of .* bare git repos:   .*B
Size of .* git repo checkouts:  .*B",
    );

    let regex = Regex::new(&desired_output);

    assert!(
        regex.clone().unwrap().is_match(&cc_output),
        "regex: {:?}, cc_output: {}",
        desired_output,
        cc_output
    );
}
