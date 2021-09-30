// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fs;
use std::path::PathBuf;

use crate::cache::caches::{get_cache_name, RegistrySubCache, RegistrySuperCache};

use rayon::prelude::*;

/// holds information on directory with .crates for one registry (subcache)
pub(crate) struct RegistryPkgCache {
    /// the name of the index
    name: String,
    /// the path of the root dir of the index, this is unique
    path: PathBuf,
    /// total size of the index, computed on-demand
    size: Option<u64>,
    /// number of files of the cache
    number_of_files: Option<usize>,
    /// flag to check if we have already calculated the files
    files_calculated: bool, // TODO: make this Option<Vec<PathBuf>>
    /// list of files contained in the index
    files: Vec<PathBuf>,
}

impl RegistrySubCache for RegistryPkgCache {
    /// create a new empty `RegistryPkgCache`
    fn new(path: PathBuf) -> Self {
        Self {
            name: get_cache_name(&path),
            path,
            size: None,
            number_of_files: None,
            files_calculated: false,
            files: vec![],
        }
    }

    // returns the name of the registry
    fn name(&self) -> &str {
        &self.name
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    /// invalidate the cache
    #[inline]
    fn invalidate(&mut self) {
        self.size = None;
        self.files_calculated = false;
        self.number_of_files = None;
        self.files = vec![];
    }

    fn known_to_be_empty(&mut self) {
        self.size = Some(0);
        self.files_calculated = true;
        self.number_of_files = Some(0);
        self.files = Vec::new();
    }

    fn total_size(&mut self) -> u64 {
        match self.size {
            Some(size) => size,
            None => {
                if self.path.is_dir() {
                    // get the size of all files in path https://news.ycombinator.com/https://news.ycombinator.com/dir
                    let total_size = self
                        .files()
                        .par_iter()
                        .filter(|f| f.is_file())
                        .map(|f| {
                            fs::metadata(f)
                                .unwrap_or_else(|_| panic!("Failed to get size of file: '{:?}'", f))
                                .len()
                        })
                        .sum();
                    self.size = Some(total_size);
                    total_size
                } else {
                    self.known_to_be_empty();
                    0
                }
            }
        }
    }

    // return a slice of files belonging to this cache
    fn files(&mut self) -> &[PathBuf] {
        if self.files_calculated {
            // just return
        } else if self.path_exists() {
            let collection = fs::read_dir(&self.path)
                .unwrap_or_else(|_| panic!("Failed to read directory (repo): '{:?}'", &self.path))
                .map(|cratepath| cratepath.unwrap().path())
                .collect::<Vec<_>>();

            self.files_calculated = true;
            self.number_of_files = Some(collection.len());
            self.files = collection;
        } else {
            // there is no directory
            // we need to reflect that in the cache
            self.files = Vec::new();
            self.files_calculated = true;
            self.number_of_files = Some(0);
        }
        &self.files
    }

    // number of files of the cache
    fn number_of_files(&mut self) -> usize {
        if let Some(number) = self.number_of_files {
            number
        } else {
            // prime the cache
            let _ = self.files();
            if let Some(n) = self.number_of_files {
                n
            } else {
                unreachable!("registry_pkg_cache: self.files is None!");
            }
        }
    }

    // sort the saved files and return them
    fn files_sorted(&mut self) -> &[PathBuf] {
        let _ = self.files(); // prime cache
        self.files.sort();
        self.files()
    }

    fn items(&mut self) -> &[PathBuf] {
        // we can use files() here
        self.files()
    }

    fn number_of_items(&mut self) -> usize {
        // we can use number_of_files() here
        self.number_of_files()
    }
}
/// holds several `RegistryPkgCaches` (supercache)
pub(crate) struct RegistryPkgCaches {
    /// root path of the cache
    #[allow(unused)]
    path: PathBuf,
    /// list of pkg caches (from alternative registries or so)
    caches: Vec<RegistryPkgCache>,
    /// number of pkg caches found
    #[allow(unused)]
    number_of_caches: usize,
    /// total size of all indices combined
    total_size: Option<u64>,
    /// number of files of all indices combined
    total_number_of_files: Option<usize>,
    // items @TODO
    items: Vec<PathBuf>,
}

impl RegistrySuperCache for RegistryPkgCaches {
    type SubCache = RegistryPkgCache;

    /// create a new empty `RegistryPkgCaches`
    fn new(path: PathBuf) -> Self {
        if !path.exists() {
            return Self {
                path,
                number_of_caches: 0,
                caches: vec![],
                total_number_of_files: None,
                total_size: None,
                items: Vec::new(),
            };
        }

        let cache_dirs = std::fs::read_dir(&path)
            .unwrap_or_else(|_| panic!("failed to read directory {}", path.display()));
        // map the dirs to RegistryIndexCaches and return them as vector
        #[allow(clippy::manual_filter_map)]
        let caches = cache_dirs
            .map(|direntry| direntry.unwrap().path())
            .filter(|p| p.is_dir() && p.file_name().unwrap().to_str().unwrap().contains('-'))
            //.inspect(|p| println!("p: {:?}", p))
            .map(RegistryPkgCache::new)
            .collect::<Vec<RegistryPkgCache>>();

        Self {
            path,
            number_of_caches: caches.len(),
            caches,
            total_number_of_files: None,
            total_size: None,
            items: Vec::new(),
        }
    }

    fn caches(&mut self) -> &mut Vec<Self::SubCache> {
        &mut self.caches
    }

    fn invalidate(&mut self) {
        self.number_of_caches = 0;
        self.total_size = None;
        self.total_number_of_files = None;
        self.caches
            .iter_mut()
            .for_each(RegistrySubCache::invalidate);
    }

    fn files(&mut self) -> Vec<PathBuf> {
        let mut all_files = Vec::new();
        for cache in &mut self.caches {
            all_files.extend(cache.files().to_vec());
        }

        all_files
    }
    fn files_sorted(&mut self) -> Vec<PathBuf> {
        let mut files_sorted = self.files();
        files_sorted.sort();
        files_sorted
    }

    // total size of all caches combined
    fn total_size(&mut self) -> u64 {
        if let Some(size) = self.total_size {
            size
        } else {
            let total_size = self
                .caches
                .iter_mut()
                .map(RegistrySubCache::total_size)
                .sum();
            self.total_size = Some(total_size);
            total_size
        }
    }
    fn number_of_subcaches(&mut self) -> usize {
        self.caches.len()
    }

    fn total_number_of_files(&mut self) -> usize {
        if let Some(number) = self.total_number_of_files {
            number
        } else {
            let number = self
                .caches
                .iter_mut()
                .map(RegistrySubCache::number_of_files)
                .sum();

            self.total_number_of_files = Some(number);
            number
        }
    }

    fn items(&mut self) -> &[PathBuf] {
        self.items = self
            .caches()
            .iter_mut()
            .flat_map(RegistrySubCache::items)
            .cloned()
            .collect::<Vec<PathBuf>>();
        &self.items
    }

    fn number_of_items(&mut self) -> usize {
        self.items().len()
    }
}
