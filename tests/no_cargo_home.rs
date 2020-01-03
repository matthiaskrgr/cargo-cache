// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
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
use regex::Regex;
use std::process::Command;

#[test]
fn run_tests() {
    // we need this fake harness to make sure the two tests don't modify CARGO_HOME at the same time
    // which would be a race condition
    CARGO_HOME_is_nonexisting_dir();
    CARGO_HOME_is_empty();
}

#[allow(non_snake_case)]
fn CARGO_HOME_is_nonexisting_dir() {
    // CARGO_HOME points to a directory that does not exist
    let cargo_cache = Command::new(bin_path())
        .env("CARGO_HOME", "./xyxyxxxyyyxxyxyxqwertywasd")
        .output();
    // make sure we failed
    let cmd = cargo_cache.unwrap();
    assert!(!cmd.status.success(), "no bad exit status!");

    // no stdout
    assert!(cmd.stdout.is_empty(), "unexpected stdout!");
    // stderr
    let stderr = String::from_utf8_lossy(&cmd.stderr).into_owned();
    assert!(!stderr.is_empty(), "found no stderr!");
    let re =
        Regex::new(r"CARGO_HOME .*./xyxyxxxyyyxxyxyxqwertywasd. is not an existing directory!\n")
            .unwrap();
    eprintln!("REGEX:\n{}", &re);
    eprintln!("OUTPUT:\n{}", &stderr);
    assert!(re.is_match(&stderr));
}
#[allow(non_snake_case)]
fn CARGO_HOME_is_empty() {
    // CARGO_HOME is empty
    // we will fall back to default "~/.cargo"
    let cargo_cache = Command::new(bin_path()).env("CARGO_HOME", "").output();
    // make sure we failed
    let cmd = cargo_cache.unwrap();
    assert!(cmd.status.success(), "bad exit status!");

    // no stdout
    assert!(!cmd.stdout.is_empty(), "unexpected stdout!");
    // stderr
    let stderr = String::from_utf8_lossy(&cmd.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&cmd.stdout).into_owned();
    assert!(stderr.is_empty());
    let re = Regex::new(r"Cargo cache.*\.cargo.*:").unwrap();
    assert!(re.is_match(&stdout));
}
