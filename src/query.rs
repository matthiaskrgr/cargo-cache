// Copyright 2017-2019 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::cache::dircache::Cache;
use crate::cache::*;
use crate::library::CargoCachePaths;
use crate::top_items::binaries::*;
use crate::top_items::git_checkouts::*;
use crate::top_items::git_repos_bare::*;
use crate::top_items::registry_cache::*;
use crate::top_items::registry_sources::*;

use clap::ArgMatches;
use regex::Regex;

pub(crate) fn run_query(
    query_config: &ArgMatches<'_>,
    ccd: &CargoCachePaths,
    mut bin_cache: &mut bin::BinaryCache,
    mut checkouts_cache: &mut git_checkouts::GitCheckoutCache,
    mut bare_repos_cache: &mut git_repos_bare::GitRepoCache,
    mut registry_cache: &mut registry_cache::RegistryCache,
    mut registry_sources_cache: &mut registry_sources::RegistrySourceCache,
) {
    println!("Query works!");
    let query = query_config.value_of("QUERY").unwrap_or("" /* default */);

    let mut binary_files = bin_cache
        .files()
        .into_iter()
        .map(|f| f.file_stem().unwrap())
        .collect::<Vec<_>>(); // etc
    binary_files.sort();
    // query by file name etc

    let re = match Regex::new(query) {
        Ok(re) => re,
        Err(e) => {
            eprintln!("Query failed to parse regex '{}': '{}'", query, e);
            std::process::exit(10);
        }
    };

    let matches = binary_files
        .iter()
        .filter(|f| re.is_match(f.to_str().unwrap()))
        .collect::<Vec<_>>();

    println!("Binaries sorted: {:?}", matches);
}

// @TODO: make sure these work:
// cargo cache q
// cargo cache query
// cargo-cache q
// cargo-cache query
