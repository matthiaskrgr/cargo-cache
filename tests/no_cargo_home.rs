// Copyright 2017-2018 Matthias Krüger. See the COPYRIGHT
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
fn no_cargo_home_dir() {
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
        Regex::new(r"Error, no cargo home path directory .*./xyxyxxxyyyxxyxyxqwertywasd' found.\n")
            .unwrap();
    assert!(re.is_match(&stderr));
}
