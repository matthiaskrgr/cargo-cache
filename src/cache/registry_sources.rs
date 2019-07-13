// Copyright 2017-2019 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fs;
use std::path::PathBuf;

use crate::cache::dircache::{get_cache_name, RegistrySubCache, RegistrySuperCache};

use rayon::prelude::*;
use walkdir::WalkDir;

// depth of a path
fn path_dept(path: &PathBuf) -> usize {
    path.iter().count()
}
#[derive(Debug, Clone)]
/// describes one registry source cache (extracted .crates)
pub(crate) struct RegistrySourceCache {
    /// the name of the index
    name: String,
    /// the path of the root dir of the index, this is uniqe
    path: PathBuf,
    /// total size of the index, computed on-demand
    size: Option<u64>,
    /// number of files of the cache
    number_of_files: Option<usize>,
    /// flag to check if we have already calculated the files
    files_calculated: bool, // TODO: make this Option<Vec<PathBuf>>
    /// list of files contained in the index
    files: Vec<PathBuf>,
    /// have we calculated the checkout folders
    checkouts_calculated: bool,
    /// the source checkout folders
    checkout_folders: Vec<PathBuf>,
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
            checkouts_calculated: false,
            checkout_folders: vec![],
        }
    }

    // returns the name of the registry
    fn name<'a>(&'a self) -> &'a str {
        &self.name
    }

    #[inline]
    fn path_exists(&self) -> bool {
        self.path.exists()
    }

    /// invalidate the cache
    #[inline]
    fn invalidate(&mut self) {
        self.size = None;
        self.files_calculated = false;
        self.number_of_files = None;
        self.files = vec![];
        self.checkouts_calculated = false;
        self.checkout_folders = vec![];
    }

    fn files(&mut self) -> &[PathBuf] {
        if self.files_calculated {
            &self.files
        } else {
            if self.path_exists() {
                let walkdir = WalkDir::new(self.path.display().to_string());
                let v = walkdir
                    .into_iter()
                    .map(|d| d.unwrap().into_path())
                    .filter(|d| d.is_file())
                    .collect::<Vec<PathBuf>>();
                self.files = v;
            } else {
                self.files = Vec::new();
            }
            &self.files
        }
    }

    fn total_size(&mut self) -> u64 {
        if self.size.is_some() {
            self.size.unwrap()
        } else if self.path.is_dir() {
            // get the size of all files in path dir
            let size = self
                .files()
                .par_iter()
                .filter(|f| f.is_file())
                .map(|f| fs::metadata(f).unwrap().len())
                .sum();
            self.size = Some(size);
            size
        } else {
            0
        }
    }

    fn files_sorted(&mut self) -> &[PathBuf] {
        let _ = self.files(); // prime cache
        self.files.sort();
        &self.files()
    }

    fn number_of_files(&mut self) -> usize {
        if self.number_of_files.is_some() {
            self.number_of_files.unwrap()
        } else {
            // we don't have the value cached
            if self.path_exists() {
                let count = self.files().len();
                self.number_of_files = Some(count);
                count
            } else {
                0
            }
        }
    }
}

impl RegistrySourceCache {
    fn number_of_source_checkout_folders(&mut self) -> usize {
        let _ = self.checkout_folders();
        self.checkout_folders.len()
    }

    fn checkout_folders(&mut self) -> &[PathBuf] {
        if self.checkouts_calculated {
            &self.checkout_folders
        } else {
            let folders = std::fs::read_dir(&self.path)
                .expect(&format!("Failed to read {:?}", self.path.display()))
                .map(|direntry| direntry.unwrap().path())
                .filter(|p| {
                    p.is_dir()
                        && p.file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string()
                            .contains('-')
                })
                .collect::<Vec<PathBuf>>();
            self.checkout_folders = folders;
            self.checkouts_calculated = true;
            &self.checkout_folders
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RegistrySourceCaches {
    /// root path of the cache
    path: PathBuf,
    /// list of pkg caches (from alternative registries or so)
    caches: Vec<RegistrySourceCache>,
    /// number of pkg caches found
    number_of_caches: usize,
    /// total size of all indices combined
    total_size: Option<u64>,
    /// number of files of all indices combined
    total_number_of_files: Option<usize>,
    /// all source checkout folders
    total_checkout_folders: Vec<PathBuf>,
    total_checkout_folders_calculated: bool,
}

impl RegistrySuperCache for RegistrySourceCaches {
    type SubCache = RegistrySourceCache;

    fn caches(&mut self) -> &mut Vec<Self::SubCache> {
        &mut self.caches
    }

    fn new(path: PathBuf) -> Self {
        let registries = std::fs::read_dir(&path)
            .unwrap_or_else(|_| panic!("failed to read directory {}", path.display()));
        #[allow(clippy::filter_map)]
        let registry_folders = registries
            .map(|direntry| direntry.unwrap().path())
            .filter(|p| {
                p.is_dir()
                    && p.file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string()
                        .contains('-')
            })
            .map(RegistrySourceCache::new)
            .collect::<Vec<RegistrySourceCache>>();

        Self {
            path,
            number_of_caches: registry_folders.len(),
            caches: registry_folders,
            total_number_of_files: None,
            total_size: None,
            total_checkout_folders: vec![],
            total_checkout_folders_calculated: false,
        }
    }

    fn invalidate(&mut self) {
        self.caches.iter_mut().for_each(|index| index.invalidate());
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
        match self.total_size {
            Some(size) => size,
            None => {
                let mut total_size = 0;
                for cache in &mut self.caches {
                    total_size += cache.total_size();
                }
                self.total_size = Some(total_size);
                total_size
            }
        }
    }
    fn number_of_items(&mut self) -> usize {
        self.caches.len()
    }
    fn total_number_of_files(&mut self) -> usize {
        match self.total_number_of_files {
            Some(number) => number,
            None => {
                let mut total = 0;
                self.caches
                    .iter_mut()
                    .for_each(|cache| total += cache.number_of_files());

                total
            }
        }
    }
}

impl RegistrySourceCaches {
    pub(crate) fn total_number_of_source_checkout_folders(&mut self) -> usize {
        let mut total = 0;
        let _ = self
            .caches
            .iter_mut()
            .for_each(|registry| total += registry.number_of_source_checkout_folders());
        total
    }

    pub(crate) fn total_checkout_folders(&mut self) -> &[PathBuf] {
        let mut all_checkout_folders = Vec::new();

        self.caches.iter_mut().for_each(|registry| {
            registry
                .checkout_folders()
                .iter()
                .for_each(|folder| all_checkout_folders.push(folder.clone()))
        });

        self.total_checkout_folders = all_checkout_folders;
        self.total_checkout_folders_calculated = true;
        &self.total_checkout_folders
    }

    pub(crate) fn total_checkout_folders_sorted(&mut self) -> &[PathBuf] {
        // prime cache
        let _ = self.total_checkout_folders();
        self.total_checkout_folders.sort();
        &self.total_checkout_folders
    }
}
