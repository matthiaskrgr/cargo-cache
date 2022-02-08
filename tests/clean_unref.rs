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

use pretty_assertions::assert_eq;
use regex::Regex;

use crate::test_helpers::{bin_path, dir_size};

#[test]
#[cfg_attr(feature = "offline_tests", ignore)]
fn test_clean_unref() {
    // this tests makes cargo create a new CARGO_HOME and tests the --clean-unref features
    const CARGO_HOME: &str = "target/clean_unref_CARGO_HOME/";

    const CACHE_POPULATION_TOML: &str = "tests/clean_unref/crate_to_populate_cache/Cargo.toml";
    const ACTUAL_TOML: &str = "tests/clean_unref/actual_crate/Cargo.toml";

    // download "extra deps" that we are going to remove via clean-nuref
    #[allow(unused)]
    let command = Command::new("cargo")
        .arg("fetch")
        .arg("--manifest-path")
        .arg(CACHE_POPULATION_TOML)
        .env("CARGO_HOME", CARGO_HOME)
        .output();

    // we are just fetching here
    /*
    let status = command.unwrap();
    let stderr = String::from_utf8_lossy(&status.stderr).to_string();
    let stdout = String::from_utf8_lossy(&status.stdout).to_string();
    dbg!(&stderr);
    dbg!(&stdout);
    */

    assert!(
        PathBuf::from(&CARGO_HOME).is_dir(),
        "fake cargo home was not created!"
    );

    // download the normal deps of the crate, most of these we are going to keep
    #[allow(unused)]
    let command = Command::new("cargo")
        .arg("fetch")
        .arg("--manifest-path")
        .arg(ACTUAL_TOML)
        .env("CARGO_HOME", CARGO_HOME)
        .output();

    /*
    let status = command.unwrap();
    let stderr = String::from_utf8_lossy(&status.stderr).to_string();
    let stdout = String::from_utf8_lossy(&status.stdout).to_string();
    dbg!(&stderr);
    dbg!(&stdout);
    */

    // we now have deps in our CARGO_HOME that the "actual crate" does not require
    // cargo-cache clean-unref --dry-run should tell us that we would remove them
    let cargo_cache_command = Command::new(bin_path())
        .arg("clean-unref")
        .arg("--manifest-path")
        .arg(ACTUAL_TOML)
        .arg("--dry-run")
        .env("CARGO_HOME", CARGO_HOME)
        .output();

    let status = cargo_cache_command.unwrap();
    //let stderr = String::from_utf8_lossy(&status.stderr).to_string();
    let stdout = String::from_utf8_lossy(&status.stdout).to_string();
    //dbg!(&stderr);
    //dbg!(&stdout);

    // make sure we would remove something
    assert!(stdout.matches("would remove").count() > 10);

    let cargo_home_size_before_removel = dir_size(&PathBuf::from(CARGO_HOME));
    // now call cargo-cache inside `actual_crate` and clean_unref WITHOUT --dry run
    // (do the actual removing)
    let cargo_cache_command = Command::new(bin_path())
        .arg("clean-unref")
        .arg("--manifest-path")
        .arg(ACTUAL_TOML)
        .env("CARGO_HOME", CARGO_HOME)
        .output();

    let cargo_home_size_after_removel = dir_size(&PathBuf::from(CARGO_HOME));
    // make sure size was reduced
    assert!(
        cargo_home_size_before_removel > cargo_home_size_after_removel,
        "CONDITION NOT MET: {} < {}",
        cargo_home_size_before_removel,
        cargo_home_size_after_removel
    );

    let status = cargo_cache_command.unwrap();
    let stderr = String::from_utf8_lossy(&status.stderr).to_string();
    let stdout = String::from_utf8_lossy(&status.stdout).to_string();
    dbg!(&stderr);
    dbg!(&stdout);
    assert_eq!("", stderr);

    // run with dry-run again, but this time make sure we would remove nothing
    let cargo_cache_command = Command::new(bin_path())
        .arg("clean-unref")
        .arg("--manifest-path")
        .arg(ACTUAL_TOML)
        .arg("--dry-run")
        .env("CARGO_HOME", CARGO_HOME)
        .output();

    dbg!(&cargo_cache_command);
    let status = cargo_cache_command.unwrap();
    let stderr = String::from_utf8_lossy(&status.stderr).to_string();
    // stderr should be empty
    assert_eq!(stderr, "");
    let stdout = String::from_utf8_lossy(&status.stdout).to_string();

    //dbg!(&stderr);
    dbg!(&stdout);

    // make sure we would remove 3 items:
    // git repo checkouts (git/checkouts/clippy_travis_test)
    // registry srcs (registry/src/.../rustc_tool_utils)
    assert_eq!(
        // differences between linux and windows for some reason?
        stdout.matches("would remove").count(),
        2
    );
    assert!(regex::Regex::new("git.checkouts")
        .unwrap()
        .is_match(&stdout));
    // registry/src...
    // we remove the "registry/src" dir as a whole and can't do any more detailed path matching because of that
    assert!(regex::Regex::new("registry.src").unwrap().is_match(&stdout));
    // we should also see that size was reduced during the operation
    // should have something like:
    // Size changed from 59.95 MB to 59.96 MB (+14.95 KB, 0.02%)
    assert!(regex::Regex::new(r#"Size changed from.*to.*.*MB"#)
        .unwrap()
        .is_match(&stdout));

    // run cargo-cache, this should tell us that the cache is almost empty
    let cargo_cache = Command::new(bin_path())
        .env("CARGO_HOME", CARGO_HOME)
        .output();
    assert!(cargo_cache.is_ok(), "cargo cache failed to run");
    let cc_output = String::from_utf8_lossy(&cargo_cache.unwrap().stdout).into_owned();
    // we need to get the actual path to fake cargo home dir and make it an absolute path
    let mut desired_output = String::from("Cargo cache .*clean_unref_CARGO_HOME.*:\n\n");
    desired_output.push_str(
        "Total:                          .* MB
  0 installed binaries:           * 0  B
  Registry:                     .* MB
    Registry index:             .* MB
    1 crate archives:           .* KB
    1 crate source checkouts:   .* KB
  Git db:                       .* KB
    1 bare git repos:           .* KB
    1 git repo checkouts:       .* KB",
    );

    dbg!(&cc_output);
    dbg!(&desired_output);

    let cargo_cache_output_iter = cc_output.lines();
    let wanted_output = desired_output.lines();
    eprintln!("\n\n\n\n");
    cargo_cache_output_iter
        .zip(wanted_output)
        .enumerate()
        .for_each(|(i, (is, wanted))| {
            let regex = Regex::new(wanted);
            assert!(
                regex.clone().unwrap().is_match(&cc_output),
                "\n\nline {}\n\n, regex:\n{:?}
            cc_output:\n{}\n\n",
                i,
                regex.unwrap(),
                is
            );
        });
}
