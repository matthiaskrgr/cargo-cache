// Copyright 2019-2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[path = "../src/test_helpers.rs"]
mod test_helpers;

use std::path::PathBuf;
use std::process::Command;

use regex::Regex;

use crate::test_helpers::bin_path;

#[test]
#[cfg_attr(feature = "offline_tests", ignore)]
fn clean_unref() {
    // this tests makes cargo create a new CARGO_HOME and tests the --clear-unref features
    const CARGO_HOME: &str = "target/clean_unref_CARGO_HOME/";

    const INITIAL_TOML: &str = "tests/clean_unref/crate_to_populate_cache/Cargo.toml";
    const ACTUAL_TOML: &str = "tests/clean_unref/actual_crate/Cargo.toml";

    // download some stuff into the new cargo home
    let command = Command::new("cargo")
        .arg("fetch")
        .arg("--manifest-path")
        .arg(INITIAL_TOML)
        .env("CARGO_HOME", CARGO_HOME)
        .output();

    let status = command.unwrap();

    let stderr = String::from_utf8_lossy(&status.stderr).to_string();
    let stdout = String::from_utf8_lossy(&status.stdout).to_string();

    println!("ERR {:?}", stderr);
    println!("OUT {:?}", stdout);

    assert!(
        PathBuf::from(&CARGO_HOME).is_dir(),
        "fake cargo home was not created!"
    );

    // we populated the cargo_home

    // make download the stuff of the actual crate

    let command = Command::new("cargo")
        .arg("fetch")
        .arg("--manifest-path")
        .arg(ACTUAL_TOML)
        .env("CARGO_HOME", CARGO_HOME)
        .output();

    let status = command.unwrap();

    let stderr = String::from_utf8_lossy(&status.stderr).to_string();
    let stdout = String::from_utf8_lossy(&status.stdout).to_string();

    println!("ERR {:?}", stderr);
    println!("OUT {:?}", stdout);

    // now call cargo-cache inside `actual_crate` and clean_unref with --dry run

    let cargo_cache_command = Command::new(bin_path())
        .arg("clean-unref")
        .arg("--manifest-path")
        .arg(ACTUAL_TOML)
        .arg("--dry-run")
        .env("CARGO_HOME", CARGO_HOME)
        .output();

    let status = cargo_cache_command.unwrap();

    let stderr = String::from_utf8_lossy(&status.stderr).to_string();
    let stdout = String::from_utf8_lossy(&status.stdout).to_string();

    println!("ERR {:?}", stderr);
    println!("OUT {:?}", stdout);

    // make sure we would remove something
    assert!(stdout.matches("would remove").count() > 10);

    // now call cargo-cache inside `actual_crate` and clean_unref WITHOUT --dry run

    let cargo_cache_command = Command::new(bin_path())
        .arg("clean-unref")
        .arg("--manifest-path")
        .arg(ACTUAL_TOML)
        .env("CARGO_HOME", CARGO_HOME)
        .output();

    let status = cargo_cache_command.unwrap();

    let stderr = String::from_utf8_lossy(&status.stderr).to_string();
    let stdout = String::from_utf8_lossy(&status.stdout).to_string();

    println!("ERR {:?}", stderr);
    println!("OUT {:?}", stdout);

    // run with dry-run again, but this time make sure we would remove nothing

    let cargo_cache_command = Command::new(bin_path())
        .arg("clean-unref")
        .arg("--manifest-path")
        .arg(ACTUAL_TOML)
        .arg("--dry-run")
        .env("CARGO_HOME", CARGO_HOME)
        .output();

    let status = cargo_cache_command.unwrap();

    let stderr = String::from_utf8_lossy(&status.stderr).to_string();
    let stdout = String::from_utf8_lossy(&status.stdout).to_string();

    println!("ERR {:?}", stderr);
    println!("OUT {:?}", stdout);

    // make sure we would remove
    // git checkouts,
    // registry srcs
    // clippy_travis_test checkout

    let rm_count = stdout.matches("would remove").count();
    assert!(
        // differences between linux and windows for some reason?
        rm_count == 3
    );

    // run cargo-cache
    let cargo_cache = Command::new(bin_path())
        .env("CARGO_HOME", CARGO_HOME)
        .output();
    assert!(cargo_cache.is_ok(), "cargo cache failed to run");
    let cc_output = String::from_utf8_lossy(&cargo_cache.unwrap().stdout).into_owned();
    // we need to get the actual path to fake cargo home dir and make it an absolute path
    let mut desired_output = String::from("Cargo cache .*clean_unref_CARGO_HOME.*:\n\n");
    desired_output.push_str(
        "Total:                          .* MB
  0 installed binaries:             0  B
  Registry:                     .* MB
    Registry index:             .* MB
    1 crate archives:           .* KB
    1 crate source checkouts:   .* KB
  Git db:                       .* KB
    1 bare git repos:           .* KB
    1 git repo checkouts:       .* KB",
    );

    let regex = Regex::new(&desired_output);

    assert!(
        regex.clone().unwrap().is_match(&cc_output),
        "regex: {:?}, cc_output: {}",
        regex,
        cc_output
    );
}
