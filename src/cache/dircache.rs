// Copyright 2017-2019 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::cache;
use crate::cache::cache_trait::Cache;
use crate::library::CargoCachePaths;

pub(crate) struct DirCache {
    pub(crate) bin: cache::bin::BinaryCache,
    pub(crate) git_checkouts: cache::git_checkouts::GitCheckoutCache,
    pub(crate) git_repos_bare: cache::git_repos_bare::GitRepoCache,
    pub(crate) registry_cache: cache::registry_cache::RegistryCache,
    pub(crate) registry_index: cache::registry_index::RegistryIndexCache,
    pub(crate) registry_sources: cache::registry_sources::RegistrySourceCache,
}

impl DirCache {
    pub(crate) fn new(ccp: CargoCachePaths) -> Self {
        Self {
            bin: cache::bin::BinaryCache::new(ccp.bin_dir),
            git_checkouts: cache::git_checkouts::GitCheckoutCache::new(ccp.git_checkouts),
            git_repos_bare: cache::git_repos_bare::GitRepoCache::new(ccp.git_repos_bare),
            registry_cache: cache::registry_cache::RegistryCache::new(ccp.registry_cache),
            registry_index: cache::registry_index::RegistryIndexCache::new(ccp.registry_index),
            registry_sources: cache::registry_sources::RegistrySourceCache::new(
                ccp.registry_sources,
            ),
        }
    }

    pub(crate) fn invalidate(&mut self) {
        self.bin.invalidate();
        self.git_checkouts.invalidate();
        self.git_repos_bare.invalidate();
        self.registry_cache.invalidate();
        self.registry_index.invalidate();
        self.registry_sources.invalidate();
    }
}
