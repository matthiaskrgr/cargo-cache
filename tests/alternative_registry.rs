// Copyright 2019 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[path = "../src/test_helpers.rs"]
mod test_helpers;

use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process::Command;

//use path_slash::PathExt;
//use walkdir::WalkDir;

#[allow(non_snake_case)]
#[test]
fn alternative_registry_works() {
    // make sure alternative registries work

    // create a CARGO_HOME with a config file

    let cargo_home = "target/alt_registries_CARGO_HOME/";
    std::fs::create_dir_all(cargo_home).unwrap();
    let cargo_home_path = PathBuf::from(&cargo_home);
    let mut cargo_config_file_path = cargo_home_path.clone(); // target/alt_registries_CARGO_HOME/config
    cargo_config_file_path.push("config");
    println!("cargo config file path: {:?}", cargo_config_file_path);
    // create the config file
    //  std::fs::File::create(&cargo_config_file_path).expect("failed to create cargo_config_file in cargo home");

    // clone the crates io index
    if !PathBuf::from("target/my-index").exists() {
        println!("cloning registry index into target/my-index");
        let git_clone_cmd = Command::new("git")
            .arg("clone")
            .arg("https://github.com/rust-lang/crates.io-index")
            .arg("--depth=1")
            .arg("--quiet")
            .arg("my-index")
            .current_dir("target/")
            .output();
        // located at target/my-index
        let status = git_clone_cmd.unwrap();
        let stderr = String::from_utf8_lossy(&status.stderr).to_string();
        let stdout = String::from_utf8_lossy(&status.stdout).to_string();

        println!("ERR {:?}", stderr);
        println!("OUT {:?}", stdout);
    }

    let my_registry_path = PathBuf::from("target/my-index");
    let my_registry_path_absolute =
        std::fs::canonicalize(&my_registry_path).expect("could not canonicalize path");

    // write the ${CARGO_HOME}/config with info on where to find the alt registry
    let mut config_file = std::fs::File::create(&cargo_config_file_path).unwrap();

    let config_text: &str = &format!(
        "[registries]
my-registry = {{ index = \"{}\" }}\n",
        my_registry_path_absolute.display()
    );

    println!("config text:\n\n{}\n\n", config_text);

    config_file.write_all(config_text.as_bytes()).unwrap();

    let project_path = std::path::PathBuf::from("target/test_crate/");
    println!("creating dummy project dir: {:?}", project_path);
    if !project_path.exists() {
        let cargo_new_cmd = Command::new("cargo")
            .arg("new")
            .arg(project_path.display().to_string())
            .output();

        let status = cargo_new_cmd.unwrap();
        let stderr = String::from_utf8_lossy(&status.stderr).to_string();
        let stdout = String::from_utf8_lossy(&status.stdout).to_string();

        println!("ERR {:?}", stderr);
        println!("OUT {:?}", stdout);
    }
    // next:
    // cargo new  test_project, add regex (from crates io) and something from the custom registry to the index

    let cargo_toml = std::path::PathBuf::from("target/test_crate/Cargo.toml");

    let _ = std::fs::File::open(&cargo_toml).unwrap();
    let mut file = OpenOptions::new().append(true).open(&cargo_toml).unwrap();

    if !std::fs::read_to_string(&cargo_toml)
        .unwrap()
        .contains("regex")
    {
        let additionl_cargo_toml_text = String::from(
            "regex = \"*\"
rayon = { version = \"1\", registry = \"my-index\" }\n",
        );
        for line in additionl_cargo_toml_text.lines() {
            let _ = writeln!(file, "{}", line).unwrap();
        }
    }
    return;
}