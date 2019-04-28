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

use std::path::PathBuf;
use std::process::Command;

use walkdir::WalkDir;

#[allow(non_snake_case)]
#[test]
fn CARGO_HOME_subdirs_are_known() {
    // this tests makes cargo create a new CARGO_HOME and makes sure that the paths that are found
    // are known by cargo cache
    let cargo_home = "target/cargo_home_subdirs_known_CARGO_HOME";

    // in the fake CARGO_HOME, install cargo-cache via git
    let _command = Command::new("cargo")
        .arg("install")
        .arg("--git")
        .arg("https://github.com/matthiaskrgr/cargo-cache")
        .arg("--debug")
        //        .current_dir(&crate_path)
        .env(
            "CARGO_TARGET_DIR",
            "../../target/cargo_home_dirs_are_known_target_dir/",
        )
        .env(
            "CARGO_HOME",
            "../../target/cargo_home_subdirs_known_CARGO_HOME/",
        )
        .output();
    // note: it does not matter if the build succeeds or not, we only need
    // cargo to inizialize the CARGO_HOME

    /*
    let status = command.unwrap();


    let stderr = String::from_utf8_lossy(&status.stderr).to_string();
    let stdout = String::from_utf8_lossy(&status.stdout).to_string();

        println!("ERR {:?}", stderr);
        println!("OUT {:?}", stdout);
*/
    assert!(
        PathBuf::from(&cargo_home).is_dir(),
        "fake cargo home was not created!"
    );

    let wd = WalkDir::new(cargo_home).max_depth(3);

    /*
    Ok(DirEntry("target/cargo_home_subdirs_known_CARGO_HOME"))
    Ok(DirEntry("target/cargo_home_subdirs_known_CARGO_HOME/git"))
    Ok(DirEntry("target/cargo_home_subdirs_known_CARGO_HOME/git/.cargo-lock-git"))
    Ok(DirEntry("target/cargo_home_subdirs_known_CARGO_HOME/git/db"))
    Ok(DirEntry("target/cargo_home_subdirs_known_CARGO_HOME/git/db/cargo-cache-16826c8e13331adc"))
    Ok(DirEntry("target/cargo_home_subdirs_known_CARGO_HOME/git/checkouts"))
    Ok(DirEntry("target/cargo_home_subdirs_known_CARGO_HOME/git/checkouts/cargo-cache-16826c8e13331adc"))
    Ok(DirEntry("target/cargo_home_subdirs_known_CARGO_HOME/registry"))
    Ok(DirEntry("target/cargo_home_subdirs_known_CARGO_HOME/registry/index"))
    Ok(DirEntry("target/cargo_home_subdirs_known_CARGO_HOME/registry/index/github.com-1ecc6299db9ec823"))
    */

    let mut wd_iter = wd.into_iter();
    assert!(wd_iter.next().unwrap().unwrap().path().display().to_string().starts_with("target/cargo_home_subdirs_known_CARGO_HOME"));
    assert!(wd_iter
        .next()
        .unwrap()
        .unwrap()
        .path()
        .display()
        .to_string()
        .starts_with("target/cargo_home_subdirs_known_CARGO_HOME/git"));
    assert!(wd_iter
        .next()
        .unwrap()
        .unwrap()
        .path()
        .display()
        .to_string()
        .starts_with("target/cargo_home_subdirs_known_CARGO_HOME/git/.cargo-lock-git"));
    assert!(wd_iter
        .next()
        .unwrap()
        .unwrap()
        .path()
        .display()
        .to_string()
        .starts_with("target/cargo_home_subdirs_known_CARGO_HOME/git/db"));
    assert!(wd_iter
        .next()
        .unwrap()
        .unwrap()
        .path()
        .display()
        .to_string()
        .starts_with("target/cargo_home_subdirs_known_CARGO_HOME/git/db/cargo-cache-"));
    assert!(wd_iter
        .next()
        .unwrap()
        .unwrap()
        .path()
        .display()
        .to_string()
        .starts_with("target/cargo_home_subdirs_known_CARGO_HOME/git/checkouts"));
    assert!(wd_iter
        .next()
        .unwrap()
        .unwrap()
        .path()
        .display()
        .to_string()
        .starts_with("target/cargo_home_subdirs_known_CARGO_HOME/git/checkouts/cargo-cache-"));
    assert!(wd_iter
        .next()
        .unwrap()
        .unwrap()
        .path()
        .display()
        .to_string()
        .starts_with("target/cargo_home_subdirs_known_CARGO_HOME/registry"));
    assert!(wd_iter
        .next()
        .unwrap()
        .unwrap()
        .path()
        .display()
        .to_string()
        .starts_with("target/cargo_home_subdirs_known_CARGO_HOME/registry/index"));
    assert!(wd_iter
        .next()
        .unwrap()
        .unwrap()
        .path()
        .display()
        .to_string()
        .starts_with("target/cargo_home_subdirs_known_CARGO_HOME/registry/index/github.com"));

    /*   for i in wd {
        println!("{:?}", i);
    }*/
}
