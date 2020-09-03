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
        // if we have a file, simply get the accesss time
        std::fs::metadata(path).unwrap().accessed().unwrap()
    } else {
        // if we have a directory, get the latest access of all files of that directory
        // get the max time / the file with the youngest access date / most recently accessed
        WalkDir::new(path)
            .into_iter()
            .map(|e| e.unwrap().path().to_owned())
            .map(|path| std::fs::metadata(path).unwrap().accessed().unwrap()) //@TODO make this an reusable function/method to simplify code
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

    // use caching, calculating the last access for each path ever time is not cheap
    // sort from youngest to oldest
    all_items.sort_by_cached_key(|path| get_last_access_of_item(path));
    // reverse the vec so that youngest access dates come first
    // [2020, 2019, 2018, ....]
    all_items.reverse();

    all_items
}

/// figure how big the cache should remain after trimming
/// 0 = no limit, don't delete anything
fn parse_size_limit_to_bytes(limit: &Option<&str>) -> usize {
    match limit {
        None => 0, //@TODO throw error here?
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
                        panic!("failed to parse, no unit supplied") // @TODO return error here
                    }
                }
            };
            let value: usize = limit[0..(limit.len() - 1)].parse().unwrap();
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
    // the cache should not exceed this limit
    let size_limit = parse_size_limit_to_bytes(size_limit);
    //FIXME
    let size_limit: u64 = 1000 * 1024 * 1024; // 1 GB
                                              // get all the items of the cache
    let all_cache_items: Vec<&PathBuf> = gather_all_cache_items(
        git_checkouts_cache,
        bare_repos_cache,
        registry_pkg_cache,
        registry_sources_cache,
        dry_run,
        size_changed,
    );

    // delete everything that is unneeded
    let mut cache_size = 0;

    // walk the items and collect items until we have reached the size limit
    all_cache_items
        // walk through the files, youngest item comes first, oldest item comes last
        .iter()
        .filter(|path| {
            let item_size = cumulative_dir_size(&path).dir_size;
            // add the item size to the cache size
            cache_size += item_size;
            // keep all items (for deletion) once we have exceeded the cache size
            cache_size > size_limit as u64
        })
        .for_each(|path| println!("{}", path.display().to_string()));
    // for debugging: the smaller the size limit is, the more items we keep for deletion
    Ok(())
}

#[cfg(test)]
mod parse_size_limit {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn size_limit() {
        // shorter function name
        fn p(limit: Option<&str>) -> usize {
            parse_size_limit_to_bytes(&limit)
        }
        assert_eq!(p(None), 0);

        assert_eq!(p(Some("1B")), 1);

        assert_eq!(p(Some("1K")), 1024);

        assert_eq!(p(Some("1M")), 1_048_576);

        assert_eq!(p(Some("1G")), 1_073_741_824);

        assert_eq!(p(Some("1T")), 1_099_511_627_776);

        assert_eq!(p(Some("4M")), 4_194_304);
        assert_eq!(p(Some("42M")), 44_040_192);
        assert_eq!(p(Some("1337M")), 1_401_946_112);
    }
}
