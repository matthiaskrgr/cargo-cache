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

use std::io::prelude::*;
use std::path::PathBuf;
use std::process::Command;

use crate::test_helpers::bin_path;
use regex::Regex;

#[allow(non_snake_case)]
#[test]
#[cfg_attr(feature = "offline_tests", ignore)]
fn alternative_registry_works() {
    // make sure alternative registries work

    // first create a $CARGO_HOME with a config file
    let cargo_home_path = {
        let mut path = PathBuf::from("target");
        path.push("alt_reg_cloudsmith_CARGO_HOME");
        path
    };

    std::fs::create_dir_all(&cargo_home_path)
        .expect("Failed to create 'alt_reg_cloudsmith_CARGO_HOME' dir");

    // get the path to the config file inside the $CARGO_HOME: target/alt_registries_CARGO_HOME/config
    let cargo_config_file_path: PathBuf = {
        let mut path = cargo_home_path.clone();
        path.push("config");
        path
    };

    println!(
        "DEBUG: cargo config file path: '{:?}'",
        cargo_config_file_path
    );

    // next we need to set up the alternative registry inside the ${CARGO_HOME}/config

    let mut cfg_file_handle = std::fs::File::create(&cargo_config_file_path)
        .expect("failed to create cargo home config file!");

    // this is the content of the cargo home config
    let config_text = String::from(
        r#"[registries]
cloudsmith = { index = "https://dl.cloudsmith.io/public/matthias-kruger/ccart/cargo/index.git" }
"#,
    );

    println!("DEBUG: config text:\n{}\n", config_text);
    // write the content into the config file
    cfg_file_handle
        .write_all(config_text.as_bytes())
        .expect("failed to fill cargo home config file");

    // the path where we try to build a test crate
    let project_path = {
        let mut path = PathBuf::from("tests");
        path.push("cloudsmith_registry_test");
        path.push("use_cloudsmith_dep");
        path
    };

    // @CONTINUE

    // get the absolute path to our cargo_home
    // target/alt_reg_cloudsmith_CARGO_HOME
    let cargo_home_path_absolute: PathBuf = {
        let mut path: PathBuf = std::env::current_dir().unwrap();
        path.push(cargo_home_path);
        path
    };

    // run the build command to force cargo to use the alternative registry
    // and fill the cargo_home with the alternative registry
    let fetch_cmd = Command::new("cargo")
        .arg("fetch")
        .current_dir(&project_path)
        .env("CARGO_HOME", cargo_home_path_absolute.display().to_string())
        .output()
        .unwrap();

    let status = fetch_cmd.status;
    let stderr = String::from_utf8_lossy(&fetch_cmd.stderr).to_string();
    let stdout = String::from_utf8_lossy(&fetch_cmd.stdout).to_string();

    // @TODO handle all  command::new() calls that way!
    if !fetch_cmd.status.success() {
        println!("error while cargo building test crate");
        println!("stderr:\n{:?}", stderr);
        println!("stdout:\n{:?}", stdout);
        println!("status: {:?}", status);
        panic!("error while building test crate");
    }

    println!("ERR {:?}", stderr);
    println!("OUT {:?}", stdout);

    // run cargo cache on the new cargo_home
    let cargo_cache_cmd = Command::new(bin_path())
        .env("CARGO_HOME", cargo_home_path_absolute.display().to_string())
        .env("RUST_BACKTRACE", "1")
        .output()
        .unwrap();

    if !cargo_cache_cmd.status.success() {
        println!("error running cargo-cache on alt reg $CARGO_HOME");
        println!("stderr:\n{:?}", stderr);
        println!("stdout:\n{:?}", stdout);
        println!("status: {:?}", status);
        panic!("error while running cargo-home with alt regs");
    }

    let stdout = String::from_utf8_lossy(&cargo_cache_cmd.stdout).to_string();

    println!("DEBUG: cargo-cache output:\n\n{}", stdout);
    // check if the output is what we expect

    let mut desired_output =
        String::from("Cargo cache .*target.*alt_reg_cloudsmith_CARGO_HOME.*\n\n");

    desired_output.push_str(
        "Total:                        .* MB
  0 installed binaries:      .* 0  B
  Registry:                    .* MB
    2 registry indices:        .* MB
    2 crate archives:          .* KB
    2 crate source checkouts:  .* KB
  Git db:                    .* 0  B
    0 bare git repos:        .* 0  B
    0 git repo checkouts:    .* 0  B",
    );

    let regex = Regex::new(&desired_output).unwrap();

    assert!(
        regex.is_match(&stdout),
        "ERROR: regex did not match!\n\nregex:\n{:?}\n\ncc_output:\n{:?}",
        regex,
        stdout
    );

    // test "cargo cache registry" output

    /*
    Cargo cache '/home/matthias/vcs/github/cargo-cache/target/alt_reg_cloudsmith_CARGO_HOME':

    Total:                          80.41 MB
      0 installed binaries:             0  B
      Registry: dl.cloudsmith.io     5.52 KB
        Registry index:              3.21 KB
        1 crate archives:             971  B
        1 crate source checkouts:    1.34 KB
      Registry: github.com          80.40 MB
        Registry index:             80.39 MB
        1 crate archives:            2.79 KB
        1 crate source checkouts:    7.76 KB
      Git db:                           0  B
        0 bare git repos:               0  B
        0 git repo checkouts:           0  B
    */

    // run cargo cache on the new cargo_home
    let cargo_cache_registry_cmd = Command::new(bin_path())
        .arg("registry")
        .env("CARGO_HOME", cargo_home_path_absolute.display().to_string())
        .output()
        .unwrap();

    if !cargo_cache_registry_cmd.status.success() {
        println!("error running cargo-cache on alt reg $CARGO_HOME");
        println!("stderr:\n{:?}", stderr);
        println!("stdout:\n{:?}", stdout);
        println!("status: {:?}", status);
        panic!("error while running cargo-home with alt regs");
    }

    let stdout = String::from_utf8_lossy(&cargo_cache_registry_cmd.stdout).to_string();

    println!("DEBUG: cargo-cache output:\n\n{}", stdout);
    // check if the output is what we expect

    let mut desired_output =
        String::from("Cargo cache .*target.*alt_reg_cloudsmith_CARGO_HOME.*\n\n");

    desired_output.push_str(
        "Total:                 .* MB
  0 installed binaries:      .*  0  B
  Registry: dl.cloudsmith.io    .* KB
    Registry index:             .* KB
    1 crate archives:           .*  B
    1 crate source checkouts:   .* KB
  Registry: github.com          .* MB
    Registry index:             .* MB
    1 crate archives:           .* KB
    1 crate source checkouts:   .* KB
  Git db:                    .*  0  B
    0 bare git repos:        .*  0  B
    0 git repo checkouts:    .*  0  B",
    );

    let regex = Regex::new(&desired_output).unwrap();

    assert!(
        regex.is_match(&stdout),
        "ERROR: regex did not match!\n\nregex:\n{:?}\n\ncc_output:\n{:?}",
        regex,
        stdout
    );
}
