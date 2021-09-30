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
use walkdir::WalkDir;

#[derive(Debug, Clone)]
/// describes one registry source cache (extracted .crates)
pub(crate) struct RegistrySourceCache {
    /// the name of the index
    name: String,
    /// the path of the root dir of the index, this is unique
    path: PathBuf,
    /// total size of the cache, computed on-demand
    size: Option<u64>,
    /// number of files of the cache
    number_of_files: Option<usize>,
    /// flag to check if we have already calculated the files
    files_calculated: bool, // TODO: make this Option<Vec<PathBuf>>
    /// list of files contained in the index
    files: Vec<PathBuf>,
    /// have we calculated the checkout folders
    items_calculated: bool,
    /// the source checkout folders
    items: Vec<PathBuf>,
}

impl RegistrySubCache for RegistrySourceCache {
    fn new(path: PathBuf) -> Self {
        Self {
            name: get_cache_name(&path),
            path,
            size: None,
            number_of_files: None,
            files_calculated: false,
            files: vec![],
            items_calculated: false,
            items: vec![],
        }
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    // returns the name of the registry
    fn name(&self) -> &str {
        &self.name
    }

    /// invalidate the cache
    #[inline]
    fn invalidate(&mut self) {
        self.size = None;
        self.files_calculated = false;
        self.number_of_files = None;
        self.files = vec![];
        self.items_calculated = false;
        self.items = vec![];
    }

    fn known_to_be_empty(&mut self) {
        self.size = Some(0);
        self.files_calculated = true;
        self.number_of_files = Some(0);
        self.files = Vec::new();
        self.items_calculated = true;
        self.items = Vec::new();
    }

    fn files(&mut self) -> &[PathBuf] {
        if self.files_calculated {
            // do nothing as everything is already calculated
        }
        if self.path_exists() {
            let walkdir = WalkDir::new(self.path.display().to_string());
            let v = walkdir
                .into_iter()
                .map(|d| d.unwrap().into_path())
                .filter(|d| d.is_file())
                .collect::<Vec<PathBuf>>();
            self.files = v;
        } else {
            self.known_to_be_empty();
        }
        &self.files
    }

    fn total_size(&mut self) -> u64 {
        if let Some(size) = self.size {
            return size;
        } else if self.path.is_dir() {
            // get the size of all files in path dir
            let size = self
                .files()
                .par_iter()
                .filter(|f| f.is_file())
                .map(|f| fs::metadata(f).unwrap().len())
                .sum();
            self.size = Some(size);
        } else {
            self.known_to_be_empty();
        }
        self.size.unwrap()
    }

    fn files_sorted(&mut self) -> &[PathBuf] {
        let _ = self.files(); // prime cache
        self.files.sort();
        self.files()
    }

    fn number_of_files(&mut self) -> usize {
        if let Some(number_of_files) = self.number_of_files {
            number_of_files
        } else {
            // we don't have the value cached
            if self.path_exists() {
                let count = self.files().len();
                self.number_of_files = Some(count);
                count
            } else {
                self.known_to_be_empty();
                0
            }
        }
    }

    #[allow(clippy::if_not_else)]
    fn items(&mut self) -> &[PathBuf] {
        if self.items_calculated {
            // we can just return them
        } else if !&self.path.exists() {
            // if there is no path, init the cache as empty
            self.items = vec![];
            self.items_calculated = true;
        } else {
            // calculate the items
            let folders = std::fs::read_dir(&self.path)
                .unwrap_or_else(|_| panic!("Failed to read {:?}", self.path.display()))
                .map(|direntry| direntry.unwrap().path())
                .filter(|p| p.is_dir() && p.file_name().unwrap().to_str().unwrap().contains('-'))
                .collect::<Vec<PathBuf>>();
            self.items = folders;
            self.items_calculated = true;
        }
        &self.items
    }

    fn number_of_items(&mut self) -> usize {
        // initialize the cache
        let _ = self.items();
        // return the number of files
        self.items.len()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RegistrySourceCaches {
    /// root path of the cache
    #[allow(unused)]
    path: PathBuf,
    /// list of pkg caches (from alternative registries or so)
    caches: Vec<RegistrySourceCache>,
    /// number of pkg caches found
    #[allow(unused)]
    number_of_caches: usize,
    /// total size of all indices combined
    total_size: Option<u64>,
    /// number of files of all indices combined
    total_number_of_files: Option<usize>,
    /// all source checkout folders
    items_calculated: bool,
    // items
    items: Vec<PathBuf>,
}

impl RegistrySuperCache for RegistrySourceCaches {
    type SubCache = RegistrySourceCache;

    fn caches(&mut self) -> &mut Vec<Self::SubCache> {
        &mut self.caches
    }

    fn new(path: PathBuf) -> Self {
        if !path.exists() {
            return Self {
                path,
                number_of_caches: 0,
                caches: vec![],
                total_number_of_files: None,
                total_size: None,
                items_calculated: false,
                items: Vec::new(),
            };
        }

        let registries = std::fs::read_dir(&path)
            .unwrap_or_else(|_| panic!("failed to read directory {}", path.display()));
        #[allow(clippy::manual_filter_map)]
        let registry_folders = registries
            .map(|direntry| direntry.unwrap().path())
            .filter(|p| p.is_dir() && p.file_name().unwrap().to_str().unwrap().contains('-'))
            .map(RegistrySourceCache::new)
            .collect::<Vec<RegistrySourceCache>>();

        Self {
            path,
            number_of_caches: registry_folders.len(),
            caches: registry_folders,
            total_number_of_files: None,
            total_size: None,
            items_calculated: false,
            items: Vec::new(),
        }
    }

    fn invalidate(&mut self) {
        self.total_number_of_files = None;
        self.total_size = None;
        self.items = vec![];
        self.items_calculated = false;
        self.caches
            .iter_mut()
            .for_each(RegistrySubCache::invalidate);
    }

    fn files(&mut self) -> Vec<PathBuf> {
        let mut all_files = Vec::new();
        self.caches
            .iter_mut()
            .for_each(|cache| all_files.extend(cache.files().to_vec()));

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

    /// number of caches this supercache holds, should be equal to the number of registries
    fn number_of_subcaches(&mut self) -> usize {
        self.caches.len()
    }

    fn total_number_of_files(&mut self) -> usize {
        if let Some(number) = self.total_number_of_files {
            number
        } else {
            let total = self
                .caches
                .iter_mut()
                .map(RegistrySubCache::number_of_files)
                .sum();

            self.total_number_of_files = Some(total);
            total
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

impl RegistrySourceCaches {
    pub(crate) fn total_checkout_folders_sorted(&mut self) -> &[PathBuf] {
        // prime cache
        let _ = self.items();
        self.items.sort();
        self.items()
    }
}
