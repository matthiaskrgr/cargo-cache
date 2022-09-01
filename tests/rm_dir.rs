// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(clippy::assertions_on_result_states)] // not that useful imo

#[path = "../src/test_helpers.rs"]
mod test_helpers;

use crate::test_helpers::bin_path;
use fs_extra::dir;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .map(|e| e.unwrap().path().to_owned())
        .filter(|f| f.exists()) // avoid broken symlinks
        .map(|f| {
            std::fs::metadata(&f)
                .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &f.display()))
                .len()
        })
        .sum::<u64>()
}

#[test]
#[cfg_attr(feature = "offline_tests", ignore)]
fn remove_dirs() {
    // make sure cargo cache --remove-dir works
    // build test crate in new cargo home

    let root_dir = std::env::current_dir();

    println!("cwd: {:?}", root_dir);
    // move into the directory of our dummy crate
    // set a fake CARGO_HOME and build the dummy crate there
    let crate_path = PathBuf::from("tests/all_cargo_home_paths_are_known/testcrate");
    let fchp = "target/remove_dir_cargo_home_orig"; // fake cargo_home path
    println!("FETCHING");
    let mut path = root_dir.unwrap();
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
        let dir = format!("target/rm_dir_cargohomes/{}", param);
        assert!(std::fs::create_dir_all(&dir).is_ok());

        let tmp_cargo_home = tempfile::tempdir_in(&dir).unwrap();
        println!("{:?}", tmp_cargo_home);

        assert!(tmp_cargo_home.path().is_dir());
        // create a new cargo home as temporary directory

        let copy_options = dir::CopyOptions::new();
        let source = cargo_home_src.clone();
        println!("SOURCE: {:?}, DEST: {:?}", source, tmp_cargo_home);
        fs_extra::copy_items(&[source], &tmp_cargo_home, &copy_options).unwrap();
        // run cargo cache and --rm-dir the cache and make sure cargo cache does not crash

        let size_before = dir_size(&PathBuf::from(&dir));

        println!("CARGO HOME: {:?}", tmp_cargo_home.path());

        let cargo_cache = Command::new(bin_path())
            .env("CARGO_HOME", tmp_cargo_home.path())
            .args(["--remove-dir", param])
            .output();
        assert!(cargo_cache.is_ok(), "cargo cache failed to run");
        assert!(
            cargo_cache.unwrap().status.success(),
            "cargo cache exit status not good"
        );
        // run again, this should still succeed (it panicd here previously due to corrupted cache)
        let mut cargo_home_path: PathBuf = tmp_cargo_home.path().into();
        cargo_home_path.push("remove_dir_cargo_home_orig");

        let cargo_cache = Command::new(bin_path())
            .env("CARGO_HOME", &cargo_home_path)
            .args(["--remove-dir", param])
            .output();
        assert!(cargo_cache.is_ok(), "cargo cache failed to run");
        assert!(
            cargo_cache.unwrap().status.success(),
            "cargo cache exit status not good"
        );
        let size_after = dir_size(&PathBuf::from(&dir));

        println!(
            "dir: {:?}, size before: {}, size after: {}",
            &dir, size_before, size_after
        );
        std::mem::drop(tmp_cargo_home);
        // make sure we reduced size!
        assert!(size_before > size_after);
    } // for param in ..
}
