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

use crate::test_helpers::bin_path;
use fs_extra::dir;
use regex::Regex;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;
use walkdir::WalkDir;

#[test]
fn remove_dirs() {
    // make sure cargo cache --remove-dir works
    // build test crate in new cargo home
    //

    let root_dir = std::env::current_dir();

    println!("cwd: {:?}", root_dir);
    // move into the directory of our dummy crate
    // set a fake CARGO_HOME and build the dummy crate there
    let crate_path = PathBuf::from("tests/all_cargo_home_paths_are_known/testcrate");
    let fchp = "target/remove_dir_cargo_home_orig"; // fake cargo_home path
    println!("FETCHING");
    let mut path = root_dir.unwrap().clone();
    path.push("target");
    path.push("remove_dir_cargo_home_orig");
    let status = Command::new("cargo")
        .arg("fetch")
        .current_dir(&crate_path)
        .env("CARGO_HOME", path)
        .output();
    // make sure the build succeeded
    println!("ASSERTING");
    assert!(
        status.is_ok(),
        "fetching deps of dummy crate did not succeed"
    );
    assert!(
        status.unwrap().status.success(),
        "fetch failed of dummy crate, exit status =! 0"
    );
    assert!(
        PathBuf::from(&fchp).is_dir(),
        "fake cargo home was not created!:
        {:?}",
        fchp
    );
    println!("DOING");

    let cargo_home_src = PathBuf::from(fchp);

    for param in &[
        "git-db",
        "git-repos",
        "registry-sources",
        "registry-crate-cache",
        "registry-index",
        "registry",
        "all",
    ] {
        // our CWD is the repo root!!
        //      let x = std::fs::read_dir(".").unwrap().collect::<Vec<_>>();
        //println!("{:?}", x);
        let dir = format!("target/rm_dir_cargohomes/{}", param);
        assert!(std::fs::create_dir_all(&dir).is_ok());

        let tmp_cargo_home = tempfile::tempdir_in(&dir).unwrap();
        println!("{:?}", tmp_cargo_home);

        assert!(tmp_cargo_home.path().is_dir());
        // create a new cargo home as temporary directory

        let copy_options = dir::CopyOptions::new();
        let source = cargo_home_src.clone();
        println!("SOURCE: {:?}, DEST: {:?}", source, tmp_cargo_home);
        fs_extra::copy_items(&vec![source], &tmp_cargo_home, &copy_options).unwrap();
        // run cargo cache and --rm-dir the cache and make sure cargo cache does not crash
        let cargo_cache = Command::new(bin_path())
            .env("CARGO_HOME", &tmp_cargo_home.path())
            .output();
        assert!(cargo_cache.is_ok(), "cargo cache failed to run");
        assert!(
            cargo_cache.unwrap().status.success(),
            "cargo cache exit status not good"
        );
        // run again, this should still succeed
        let cargo_cache = Command::new(bin_path())
            .env("CARGO_HOME", &tmp_cargo_home.path())
            .output();
        assert!(cargo_cache.is_ok(), "cargo cache failed to run");
        assert!(
            cargo_cache.unwrap().status.success(),
            "cargo cache exit status not good"
        );

        // copy cargo home
        // run cargo cache remove dir ..
        // make sure nothing panics
        // make sure size is reduced
        //
    }
}