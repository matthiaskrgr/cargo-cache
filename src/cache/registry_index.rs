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

use rayon::iter::*;
use walkdir::WalkDir;

/// describes a single index of a crate registry index
pub(crate) struct RegistryIndex {
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

impl RegistrySubCache for RegistryIndex {
    /// create a new empty `RegistryIndex`
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

    /// check if the path is still present
    #[inline]
    fn path_exists(&self) -> bool {
        self.path().exists()
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
                    // get the size of all files in path dir
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
            // do nothing and return
        } else if self.path_exists() {
            let walkdir = WalkDir::new(self.path.display().to_string());
            let vec = walkdir
                .into_iter()
                .map(|direntry| direntry.unwrap().into_path())
                .collect::<Vec<PathBuf>>();

            self.number_of_files = Some(vec.len());

            self.files = vec;
            self.files_calculated = true;
        } else {
            self.known_to_be_empty();
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
                unreachable!();
            }
        }
    }

    // sort the saved files and return them
    fn files_sorted(&mut self) -> &[PathBuf] {
        let _ = self.files(); // prime cache
        self.files.sort();
        self.files()
    }

    // note: it does not really make sense to have
    // items()
    // and
    // number_of_items()
    // here since the registry index does not contain any items besides "files"
    // and these are already covered
    fn items(&mut self) -> &[PathBuf] {
        &[]
    }

    // see above
    fn number_of_items(&mut self) -> usize {
        0
    }
}

pub(crate) struct RegistryIndicesCache {
    /// root path of the cache
    #[allow(unused)]
    path: PathBuf,
    /// list of indices (from alternative registries or so)
    indices: Vec<RegistryIndex>,
    /// number of indices found
    #[allow(unused)]
    number_of_indices: usize,
    /// total size of all indices combined
    total_size: Option<u64>,
    /// number of files of all indices combined
    total_number_of_files: Option<usize>,
    /// indices but as paths
    indices_paths: Vec<PathBuf>,
}

impl RegistrySuperCache for RegistryIndicesCache {
    type SubCache = RegistryIndex;

    /// create a new empty `RegistryIndexCache`
    fn new(path: PathBuf) -> Self {
        if !path.exists() {
            return Self {
                path,
                number_of_indices: 0,
                indices: vec![],
                total_number_of_files: None,
                total_size: None,
                indices_paths: Vec::new(),
            };
        }

        let indices_dirs = std::fs::read_dir(&path)
            .unwrap_or_else(|_| panic!("failed to read directory {}", path.display()));
        // map the dirs to RegistryIndexCaches and return them as vector
        #[allow(clippy::manual_filter_map)]
        let indices = indices_dirs
            .map(|direntry| direntry.unwrap().path())
            .filter(|p| p.is_dir() && p.file_name().unwrap().to_str().unwrap().contains('-'))
            //.inspect(|p| println!("p: {:?}", p))
            .map(RegistryIndex::new)
            .collect::<Vec<RegistryIndex>>();

        Self {
            path,
            number_of_indices: indices.len(),
            indices,
            total_number_of_files: None,
            total_size: None,
            indices_paths: Vec::new(),
        }
    }

    fn caches(&mut self) -> &mut Vec<Self::SubCache> {
        &mut self.indices
    }

    fn invalidate(&mut self) {
        self.number_of_indices = 0;
        self.total_size = None;
        self.total_number_of_files = None;
        self.indices
            .iter_mut()
            .for_each(RegistrySubCache::invalidate);
    }

    fn files(&mut self) -> Vec<PathBuf> {
        let mut all_files = Vec::new();
        for index in &mut self.indices {
            all_files.extend(index.files().to_vec());
        }

        all_files
    }

    fn files_sorted(&mut self) -> Vec<PathBuf> {
        let mut files_sorted = self.files();
        files_sorted.sort();
        files_sorted
    }

    // total size of all indices combined
    fn total_size(&mut self) -> u64 {
        if let Some(size) = self.total_size {
            size
        } else {
            let total_size = self
                .indices
                .iter_mut()
                .map(RegistrySubCache::total_size)
                .sum();

            self.total_size = Some(total_size);
            total_size
        }
    }
    fn number_of_subcaches(&mut self) -> usize {
        self.indices.len()
    }

    fn total_number_of_files(&mut self) -> usize {
        match self.total_number_of_files {
            Some(number) => number,
            None => {
                //@TODO make everything used here return usize
                #[allow(clippy::cast_possible_truncation)]
                self.indices
                    .iter_mut()
                    .map(|index| index.total_size() as usize)
                    .sum()
            }
        }
    }

    fn items(&mut self) -> &[PathBuf] {
        let v: Vec<PathBuf> = self
            .caches()
            .iter()
            .map(|index| index.path().clone())
            .collect();
        self.indices_paths = v;
        &self.indices_paths
    }

    // see above
    fn number_of_items(&mut self) -> usize {
        self.caches().len()
    }
}
