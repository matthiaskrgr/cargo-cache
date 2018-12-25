// Copyright 2017-2018 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::cache::dircache::DirCache;
use crate::library::CargoCachePaths;
use crate::top_items::binaries::*;
use crate::top_items::git_checkouts::*;
use crate::top_items::git_repos_bare::*;
use crate::top_items::registry_cache::*;
use crate::top_items::registry_sources::*;

pub(crate) fn get_top_crates(
    limit: u32,
    ccd: &CargoCachePaths,
    mut cache: &mut DirCache,
) -> String {
    let binaries = binary_stats(&ccd.bin_dir, limit, &mut cache);

    let reg_src = registry_source_stats(&ccd.registry_sources, limit, &mut cache);
    let reg_cache = registry_cache_stats(&ccd.registry_cache, limit, &mut cache);

    let bare_repos = git_repos_bare_stats(&ccd.git_repos_bare, limit, &mut cache);
    let repo_checkouts = git_checkouts_stats(&ccd.git_checkouts, limit, &mut cache);

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
