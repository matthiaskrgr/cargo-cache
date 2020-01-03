// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// note: to make debug prints work:
// cargo test -- --nocapture
#[path = "../src/test_helpers.rs"]
mod test_helpers;

use crate::test_helpers::bin_path;
use std::path::*;
use std::process::Command;

#[test]
fn cargo_new_and_run_local() {
    // first we create a new empty cargo project
    let target_dir = PathBuf::from("target");
    let local_project = target_dir.join("local_project");

    if !local_project.exists() {
        let cargo_new_status = Command::new("cargo")
            .arg("new")
            .arg("local_project")
            .current_dir(&target_dir)
            .output();

        assert!(
            cargo_new_status.is_ok(),
            "creating new project via cargo new did not succeed"
        );
        assert!(
            cargo_new_status.unwrap().status.success(),
            "cargo new failed exit status =! 0"
        );
    }

    assert!(local_project.is_dir());
    assert!(local_project.exists());

    if local_project.is_dir() {
        let cargo_clean = Command::new("cargo")
            .arg("clean")
            .current_dir(&local_project)
            .output();
        assert!(cargo_clean.is_ok(), "cargo clean did not succeed");
        assert!(
            cargo_clean.unwrap().status.success(),
            "cargo clean failed exit status =! 0"
        );
        let local_target_dir = local_project.join("target");
        // target dir must be gone
        assert!(!local_target_dir.exists());
    }

    let local_project = local_project.canonicalize().unwrap();

    // if we are in  target/local_project, the binary is in ../../target/debug/...

    let cc_binary = {
        let mut cwd = std::env::current_dir().expect("Could not get cwd!");
        cwd.push(&bin_path());
        cwd
    };

    let cargo_cache_local = Command::new(&cc_binary)
        .arg("local")
        .current_dir(&local_project)
        .output()
        .unwrap();

    let cc_output = String::from_utf8_lossy(&cargo_cache_local.stdout).into_owned();
    assert!(!cc_output.contains("error"), "cargo cache did not error!");
    // status must be none zero / bad
    assert!(
        !cargo_cache_local.status.success(),
        "cargo cache local did not deliver bad exit status when target dir was missing"
    );

    // ok we errored, now try the same again but with a target dir

    // run cargo check in the local project

    let cargo_check = Command::new("cargo")
        .arg("check")
        .current_dir(&local_project)
        .output();
    assert!(cargo_check.is_ok(), "cargo check did not succeed");
    assert!(
        cargo_check.unwrap().status.success(),
        "cargo check failed exit status =! 0"
    );
    let local_target_dir = local_project.join("target");
    // target dir must be gone
    assert!(local_target_dir.exists());

    // run cargo cache l again
    let cargo_cache_local = Command::new(&cc_binary)
        .arg("l")
        .current_dir(&local_project)
        .output()
        .unwrap();

    let cc_output = String::from_utf8_lossy(&cargo_cache_local.stdout).into_owned();
    assert!(cc_output.contains("Total Size:"), "cargo cache errored!");
    // status must be none zero / bad
    assert!(
        cargo_cache_local.status.success(),
        "cargo cache local did not deliver bad exit when target dir should have been found"
    );
}
