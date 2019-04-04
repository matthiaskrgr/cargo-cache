// Copyright 2017-2019 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::cache::*;
use crate::library::CargoCachePaths;
use crate::top_items::binaries::*;
use crate::top_items::git_checkouts::*;
use crate::top_items::git_repos_bare::*;
use crate::top_items::registry_pkg_cache::*;
use crate::top_items::registry_sources::*;

#[allow(clippy::complexity)]
pub(crate) fn get_top_crates(
    limit: u32,
    ccd: &CargoCachePaths,
    mut bin_cache: &mut bin::BinaryCache,
    mut checkouts_cache: &mut git_checkouts::GitCheckoutCache,
    mut bare_repos_cache: &mut git_repos_bare::GitRepoCache,
    mut registry_pkg_cache: &mut registry_pkg_cache::RegistryCache,
    mut registry_sources_cache: &mut registry_sources::RegistrySourceCache,
) -> String {
    let (((reg_src, reg_cache), (bare_repos, repo_checkouts)), binaries) = rayon::join(
        || {
            rayon::join(
                || {
                    rayon::join(
                        || {
                            registry_source_stats(
                                &ccd.registry_sources,
                                limit,
                                &mut registry_sources_cache,
                            )
                        },
                        || registry_pkg_cache_stats(&ccd.registry_pkg_cache, limit, &mut registry_pkg_cache),
                    )
                },
                || {
                    rayon::join(
                        || git_repos_bare_stats(&ccd.git_repos_bare, limit, &mut bare_repos_cache),
                        || git_checkouts_stats(&ccd.git_checkouts, limit, &mut checkouts_cache),
                    )
                },
            )
        },
        || binary_stats(&ccd.bin_dir, limit, &mut bin_cache),
    );

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
