// Copyright 2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// "cargo cache trim" command

use std::fs;
use std::path::PathBuf;

use crate::cache::caches::*;
use crate::cache::*;
use crate::library::*;
use crate::library::{CargoCachePaths, Error};
use crate::remove::*;
use cargo_metadata::{CargoOpt, MetadataCommand};

use clap::ArgMatches;
use humansize::{file_size_opts, FileSize};
use walkdir::WalkDir;

pub(crate) fn gather_all_cache_items(
    cargo_cache_paths: &CargoCachePaths,
    git_checkouts_cache: &mut git_checkouts::GitCheckoutCache,
    bare_repos_cache: &mut git_bare_repos::GitRepoCache,
    registry_pkg_cache: &mut registry_pkg_cache::RegistryPkgCaches,
    registry_sources_cache: &mut registry_sources::RegistrySourceCaches,
    dry_run: bool,
    size_changed: &mut bool,
) -> () {
    let mut all_items: Vec<&PathBuf> = Vec::new();
    all_items.extend(git_checkouts_cache.items());
    all_items.extend(bare_repos_cache.items());
    all_items.extend(registry_pkg_cache.items());
    all_items.extend(registry_sources_cache.items());

    all_items.sort_by_key(|path| get_last_access_of_file(path));

    let first = all_items[0];
    let last = all_items.len() - 1;
    let last = all_items[last];
    println!("first {:?}", first);
    println!("last {:?}", last);
}

fn get_last_access_of_file(path: &PathBuf) -> std::time::SystemTime {
    if path.is_file() {
        std::fs::metadata(path).unwrap().accessed().unwrap()
    } else {
        // directory, get the latest access of all files of that directory
        // get the max time / the file with the youngest access date / most recently accessed
        WalkDir::new(path.display().to_string())
            .into_iter()
            .map(|e| e.unwrap().path().to_owned())
            .map(|path| std::fs::metadata(path).unwrap().accessed().unwrap())
            .max()
            .unwrap()
    }
}
