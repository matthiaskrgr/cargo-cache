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

    /* let first = all_items[0];
    let last_idx = all_items.len() - 1;
    let last = all_items[last_idx];
    //println!("first {:?}", first);
    //println!("last {:?}", last);
    */
    all_items
}

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

pub(crate) fn trim_cache(
    size_limit: &Option<&str>,
    git_checkouts_cache: &mut git_checkouts::GitCheckoutCache,
    bare_repos_cache: &mut git_bare_repos::GitRepoCache,
    registry_pkg_cache: &mut registry_pkg_cache::RegistryPkgCaches,
    registry_sources_cache: &mut registry_sources::RegistrySourceCaches,
    dry_run: bool,
    size_changed: &mut bool,
) -> Result<(), ()> {
    Ok(())
}

fn parse_size_limit(limit: &Option<&str>) -> usize {
    match limit {
        None => 0,
        Some(limit) => {
            // figure out the unit
            let unit_multiplicator: usize = match limit.chars().last() {
                // limit is empty
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
            return value * unit_multiplicator;
        }
    };
    0
}
