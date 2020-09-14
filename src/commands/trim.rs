// Copyright 2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// "cargo cache trim" command
// trim the size of the cargo cache down to a certain limit.
// note that this does not take account the registry indices and the installed binaries in calculations

use std::fmt;
use std::path::PathBuf;

use crate::cache::caches::*;
use crate::cache::*;
use crate::library::*;
use crate::remove::*;

use walkdir::WalkDir;

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum TrimError<'a> {
    // failed to parse the unit of a `cargo cache trim --limit 123G` argument
    TrimLimitUnitParseFailure(&'a str),
}

impl fmt::Display for TrimError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::TrimLimitUnitParseFailure(limit) => {
                write!(f, "Failed to parse limit: \"{}\". Should be of the form 123X where X is one of B,K,M,G or T.", limit)
            }
        }
    }
}

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
) -> Vec<&'a PathBuf> {
    let mut all_items: Vec<&PathBuf> = Vec::new();
    all_items.extend(git_checkouts_cache.items());
    all_items.extend(bare_repos_cache.items());
    all_items.extend(registry_pkg_cache.items());
    all_items.extend(registry_sources_cache.items());

    // calculating the last access for each path ever time is not cheap, so use caching
    // sort from youngest to oldest
    all_items.sort_by_cached_key(|path| get_last_access_of_item(path));
    // reverse the vec so that youngest access dates come first
    // [2020, 2019, 2018, ....]
    all_items.reverse();

    all_items
}

/// figure out how big the cache should remain after trimming
fn parse_size_limit_to_bytes<'a>(limit: &Option<&'a str>) -> Result<u64, TrimError<'a>> {
    match limit {
        None => unreachable!("No trim --limit was supplied altough clap should enforce that!"),
        Some(limit) => {
            // figure out the unit
            let unit_multiplicator: Result<u64, TrimError<'a>> = match limit.chars().last() {
                // we have no limit
                None => Ok(0),
                // we expect a unit such as B, K, M, G, T...
                Some(c) => {
                    if c.is_alphabetic() {
                        match c {
                            'b' | 'B' => Ok(1),
                            'k' | 'K' => Ok(1024),
                            'm' | 'M' => Ok(1024 * 1024),
                            'g' | 'G' => Ok(1024 * 1024 * 1024),
                            't' | 'T' => Ok(1024 * 1024 * 1024 * 1024),
                            _ => Err(TrimError::TrimLimitUnitParseFailure(limit)),
                        }
                    } else {
                        Err(TrimError::TrimLimitUnitParseFailure(limit))
                    }
                }
            };
            let value: f64 = limit[0..(limit.len() - 1)].parse().unwrap();
            if value == 0.0 {
                return Ok(0);
            }
            // we may truncate the value here but that's ok
            #[allow(clippy::cast_lossless)]
            #[allow(clippy::cast_sign_loss)]
            #[allow(clippy::cast_possible_truncation)]
            #[allow(clippy::cast_precision_loss)]
            Ok((value * unit_multiplicator? as f64) as u64)
        }
    }
}

// this is the function that trim sthe cache to a given limit
pub(crate) fn trim_cache<'a>(
    unparsed_size_limit: &Option<&'a str>,
    git_checkouts_cache: &mut git_checkouts::GitCheckoutCache,
    bare_repos_cache: &mut git_bare_repos::GitRepoCache,
    registry_pkg_cache: &mut registry_pkg_cache::RegistryPkgCaches,
    registry_sources_cache: &mut registry_sources::RegistrySourceCaches,
    dry_run: bool,
    size_changed: &mut bool,
) -> Result<(), TrimError<'a>> {
    // the cache should not exceed this limit
    let size_limit = parse_size_limit_to_bytes(unparsed_size_limit)?;

    // fast path:
    // if the  limit is bigger than the cache size, we can return
    // because we know we won't have to delete anything
    let total_cache_size: u64 = git_checkouts_cache.total_size()
        + bare_repos_cache.total_size()
        + registry_pkg_cache.total_size()
        + registry_sources_cache.total_size();

    if size_limit > total_cache_size {
        //println!("trim: limit exceeds cache-limit, doing nothing");
        return Ok(());
    }

    // get all the items of the cache
    let all_cache_items: Vec<&PathBuf> = gather_all_cache_items(
        git_checkouts_cache,
        bare_repos_cache,
        registry_pkg_cache,
        registry_sources_cache,
    );

    // delete everything that is unneeded
    let mut cache_size = 0;
    // walk the items and collect items until we have reached the size limit
    all_cache_items
        // walk through the files, youngest item comes first, oldest item comes last
        .iter()
        .filter(|path| {
            let item_size = size_of_path(path);
            // add the item size to the cache size
            cache_size += item_size;
            // keep all items (for deletion) once we have exceeded the cache size
            cache_size > size_limit
        })
        // .for_each(|path| println!("{}", path.display().to_string()));
        // for debugging: the smaller the size limit is, the more items we keep for deletion
        .for_each(|path| {
            remove_file(
                path,
                dry_run,
                size_changed,
                None,
                &DryRunMessage::Default,
                None,
            )
        });
    Ok(())
}

#[cfg(test)]
mod parse_size_limit {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn size_limit() {
        // shorter function name
        fn p<'a>(limit: &Option<&'a str>) -> Result<u64, TrimError<'a>> {
            parse_size_limit_to_bytes(limit)
        }

        assert_eq!(p(&Some("1b")), Ok(1));
        assert_eq!(p(&Some("1B")), Ok(1));

        assert_eq!(p(&Some("1k")), Ok(1_024));
        assert_eq!(p(&Some("1K")), Ok(1_024));

        assert_eq!(p(&Some("1m")), Ok(1_048_576));
        assert_eq!(p(&Some("1M")), Ok(1_048_576));

        assert_eq!(p(&Some("1g")), Ok(1_073_741_824));
        assert_eq!(p(&Some("1G")), Ok(1_073_741_824));

        assert_eq!(p(&Some("1t")), Ok(1_099_511_627_776));
        assert_eq!(p(&Some("1T")), Ok(1_099_511_627_776));

        assert_eq!(p(&Some("4M")), Ok(4_194_304));
        assert_eq!(p(&Some("42M")), Ok(44_040_192));
        assert_eq!(p(&Some("1337M")), Ok(1_401_946_112));

        assert_eq!(p(&Some("1.5k")), Ok(1_536));

        assert_eq!(
            p(&Some("1_")),
            Err(TrimError::TrimLimitUnitParseFailure("1_"))
        );
    }

    // make sure Size limit None panicss
    #[test]
    #[should_panic(expected = "No trim --limit was supplied altough clap should enforce that!")]
    fn size_limit_none_panics() {
        let _ = parse_size_limit_to_bytes(&None);
    }
}
