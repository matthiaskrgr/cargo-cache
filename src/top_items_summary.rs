// Copyright 2017-2018 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::library::CargoCachePaths;
use crate::top_items::binaries::*;
use crate::top_items::git_checkouts::*;
use crate::top_items::git_repos_bare::*;
use crate::top_items::registry_cache::*;
use crate::top_items::registry_sources::*;

pub(crate) fn get_top_crates(limit: u32, ccd: &CargoCachePaths) -> String {
    // run the functions in parallel for a tiny speedup
    let (binaries, other) = rayon::join(
        || binary_stats(&ccd.bin_dir, limit),
        || {
            rayon::join(
                || {
                    rayon::join(
                        || registry_source_stats(&ccd.registry_sources, limit),
                        || registry_cache_stats(&ccd.registry_cache, limit),
                    )
                },
                || {
                    rayon::join(
                        || git_repos_bare_stats(&ccd.git_repos_bare, limit),
                        || git_checkouts_stats(&ccd.git_checkouts, limit),
                    )
                },
            )
        },
    );
    // destruct all the tupels
    let (reg_src_and_cache /*tup*/, git_bare_repos_and_checkouts /*tup*/) = other;
    // split up tupels into single variables
    let (reg_src, reg_cache) = reg_src_and_cache;
    let (bare_repos, repo_checkouts) = git_bare_repos_and_checkouts;

    // concat the strings in the order we want them
    let mut output = String::with_capacity(
        binaries.len() + reg_src.len() + reg_cache.len() + bare_repos.len() + repo_checkouts.len(),
    );
    output.push_str(&binaries);
    output.push_str(&reg_src);
    output.push_str(&reg_cache);
    output.push_str(&bare_repos);
    output.push_str(&repo_checkouts);
    // strip newlines at the end and the beginning
    output.trim().to_string()
}
