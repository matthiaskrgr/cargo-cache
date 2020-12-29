// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::path::Path;

#[allow(dead_code)] // only used in tests
pub(crate) fn bin_path() -> String {
    let target_dir = cargo_metadata::MetadataCommand::new()
        .exec()
        .unwrap()
        .target_directory;

    // check if we have a release or debug binary to run tests with
    // we need to take into account that linux and windows have different paths to the executable
    let path_release = if cfg!(windows) {
        let mut td = target_dir.clone();
        td.push("release");
        td.push("cargo-cache.exe");
        td
    } else {
        let mut td = target_dir.clone();
        td.push("release");
        td.push("cargo-cache");
        td
    };

    let path_debug = if cfg!(windows) {
        let mut td = target_dir;
        td.push("debug");
        td.push("cargo-cache.exe");
        td
    } else {
        let mut td = target_dir;
        td.push("debug");
        td.push("cargo-cache");
        td
    };

    if path_release.is_file() {
        path_release.display().to_string()
    } else if path_debug.is_file() {
        path_debug.display().to_string()
    } else {
        panic!("No cargo-cache executable found!");
    }
}

#[allow(dead_code)] // only used in tests
pub(crate) fn assert_path_end(path: &Path, wanted_vector: &[&str]) {
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
