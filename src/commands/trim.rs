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

fn get_last_access_of_item(path: &PathBuf) -> std::time::SystemTime {
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

// get a list of all cache items, sorted by file access time (young to old)
pub(crate) fn gather_all_cache_items<'a>(
    git_checkouts_cache: &'a mut git_checkouts::GitCheckoutCache,
    bare_repos_cache: &'a mut git_bare_repos::GitRepoCache,
    registry_pkg_cache: &'a mut registry_pkg_cache::RegistryPkgCaches,
    registry_sources_cache: &'a mut registry_sources::RegistrySourceCaches,
    dry_run: bool,
    size_changed: &mut bool,
) -> Vec<&'a PathBuf> {
    let mut all_items: Vec<&PathBuf> = Vec::new();
    all_items.extend(git_checkouts_cache.items());
    all_items.extend(bare_repos_cache.items());
    all_items.extend(registry_pkg_cache.items());
    all_items.extend(registry_sources_cache.items());

    all_items.sort_by_key(|path| get_last_access_of_item(path));

    all_items
}

/// figure how big the cache should remain after trimming
/// 0 = no limit, don't delete anything
fn parse_size_limit(limit: &Option<&str>) -> usize {
    match limit {
        None => 0,
        Some(limit) => {
            // figure out the unit
            let unit_multiplicator: usize = match limit.chars().last() {
                // we have no limit
                None => 0,
                // we expect a unit such as B, K, M, G, T...
                Some(c) => {
                    if c.is_alphabetic() {
                        match c {
                            'B' => 1,
                            'K' => 1024,
                            'M' => 1024 * 1024,
                            'G' => 1024 * 1024 * 1024,
                            'T' => 1024 * 1024 * 1024 * 1024,
                            _ => panic!("failed to parse unit, please use one of B K M G or T"),
                        }
                    } else {
                        panic!("failed to parse")
                    }
                }
            };
            let value: usize = limit[0..=limit.len()].parse().unwrap();
            if value == 0 {
                return 0;
            }
            value * unit_multiplicator
        }
    }
}

pub(crate) fn trim_cache(
    size_limit: &Option<&str>,
    git_checkouts_cache: &mut git_checkouts::GitCheckoutCache,
    bare_repos_cache: &mut git_bare_repos::GitRepoCache,
    registry_pkg_cache: &mut registry_pkg_cache::RegistryPkgCaches,
    registry_sources_cache: &mut registry_sources::RegistrySourceCaches,
    dry_run: bool,
    size_changed: &mut bool,
) -> Result<(), ()> {
    // parse the size limit
    let size_limit = parse_size_limit(size_limit);
    // get all the items of the cache
    let all_cache_items = gather_all_cache_items(
        git_checkouts_cache,
        bare_repos_cache,
        registry_pkg_cache,
        registry_sources_cache,
        dry_run,
        size_changed,
    );

    let mut cache_size = 0;

    // delete everything that is unneeded
    all_cache_items.iter().for_each(|_| ());

    unimplemented!();
}
