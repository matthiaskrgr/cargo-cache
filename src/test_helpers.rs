// Copyright 2017-2018 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::path::PathBuf;

#[allow(dead_code)]
pub(crate) fn bin_path() -> String {
    let path_release = if cfg!(windows) {
        "target\\release\\cargo-cache.exe"
    } else {
        "target/release/cargo-cache"
    };

    let path_debug = if cfg!(windows) {
        "target\\debug\\cargo-cache.exe"
    } else {
        "target/debug/cargo-cache"
    };

    if PathBuf::from(path_release).is_file() {
        String::from(path_release)
    } else if PathBuf::from(path_debug).is_file() {
        String::from(path_debug)
    } else {
        panic!("No cargo-cache executable found!");
    }
}

#[allow(dead_code)]
pub(crate) fn assert_path_end(path: &PathBuf, wanted_vector: &[&str]) {
    // because windows and linux represent paths differently ( /foo/bar vs C:\\foo\\bar)
    // we need to take this into account when running test on windows/linux

    // the function just splits up paths into their individual components
    // and checks these against a vector string
    let components = path.components().map(|c| c.as_os_str().to_str().unwrap());
    let components_vec: Vec<&str> = components.collect();
    let wanted_len = components_vec.len() - wanted_vector.len();

    let is: &[&str] = &components_vec[wanted_len..];
    let wanted: &[&str] = &wanted_vector[..];

    assert_eq!(is, wanted);
}
