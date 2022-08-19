// Copyright 2017-2022 Matthias Krüger. See the COPYRIGHT
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
use std::process::Command;

#[test]
#[cfg_attr(feature = "offline_tests", ignore)]
fn no_rustup_installed_toolchain() {
    // run it on the fake cargo cache dir
    let cargo_cache = Command::new(bin_path())
        .arg("toolchain")
        .env("RUSTUP_HOME", "/tmp/this/dir/does/not/exist")
        .output();
    assert!(cargo_cache.is_ok(), "cargo cache toolchain failed to run");
    let cc_output = &cargo_cache.unwrap();
    let cc_stdout = String::from_utf8_lossy(&cc_output.stdout).into_owned();
    // we need to get the actual path to fake cargo home dir and make it an absolute path
    let cc_stderr = String::from_utf8_lossy(&cc_output.stderr).into_owned();

    assert_eq!(cc_stdout, "");

    assert_eq!(
        cc_stderr,
        "Could not find any toolchains installed via rustup!\n"
    );
}
