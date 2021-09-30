// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
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
use crate::top_items::git_bare_repos::*;
use crate::top_items::git_checkouts::*;
use crate::top_items::registry_pkg_cache::*;
use crate::top_items::registry_sources::*;

#[allow(clippy::complexity)]
pub(crate) fn get_top_crates(
    limit: u32,
    ccd: &CargoCachePaths,
    bin_cache: &mut bin::BinaryCache,
    checkouts_cache: &mut git_checkouts::GitCheckoutCache,
    bare_repos_cache: &mut git_bare_repos::GitRepoCache,
    registry_pkg_caches: &mut registry_pkg_cache::RegistryPkgCaches,
    registry_sources_caches: &mut registry_sources::RegistrySourceCaches,
) -> String {
    let mut reg_src = String::new();
    let mut reg_cache = String::new();
    let mut bare_repos = String::new();
    let mut repo_checkouts = String::new();
    let mut binaries = String::new();

    rayon::scope(|s| {
        s.spawn(|_| {
            reg_src = registry_source_stats(&ccd.registry_sources, limit, registry_sources_caches);
        });

        s.spawn(|_| {
            reg_cache =
                registry_pkg_cache_stats(&ccd.registry_pkg_cache, limit, registry_pkg_caches);
        });

        s.spawn(|_| {
            bare_repos = git_repos_bare_stats(&ccd.git_repos_bare, limit, bare_repos_cache);
        });

        s.spawn(|_| {
            repo_checkouts = git_checkouts_stats(&ccd.git_checkouts, limit, checkouts_cache);
        });

        s.spawn(|_| {
            binaries = binary_stats(&ccd.bin_dir, limit, bin_cache);
        });
    });

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
