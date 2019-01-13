// Copyright 2017-2019 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::cache::cache_trait::Cache;
use std::fs;
use std::path::PathBuf;

use rayon::iter::*;

pub(crate) struct RegistryCache {
    path: PathBuf,
    total_size: Option<u64>,
    number_of_files: Option<usize>,
    files_calculated: bool,
    files: Vec<PathBuf>,
}

impl Cache for RegistryCache {
    fn new(path: PathBuf) -> Self {
        // calculate once and save, return as needed
        Self {
            path,
            total_size: None,
            number_of_files: None,
            files_calculated: false,
            files: Vec::new(),
        }
    }
    #[inline]
    fn path_exists(&self) -> bool {
        self.path.exists()
    }
    fn invalidate(&mut self) {
        self.total_size = None;
        self.number_of_files = None;
        self.files_calculated = false;
    }
}

impl RegistryCache {
    pub(crate) fn number_of_files(&mut self) -> usize {
        if self.number_of_files.is_some() {
            self.number_of_files.unwrap()
        } else if self.path_exists() {
            let count = self.files().len();
            self.number_of_files = Some(count);
            count
        } else {
            0
        }
    }

    pub(crate) fn total_size(&mut self) -> u64 {
        if self.total_size.is_some() {
            self.total_size.unwrap()
        } else if self.path.is_dir() {
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
            self.total_size = Some(total_size);
            total_size
        } else {
            0
        }
    }

    pub(crate) fn files(&mut self) -> &[PathBuf] {
        if self.files_calculated {
            &self.files
        } else {
            if self.path_exists() {
                let mut collection = Vec::new();

                let crate_list = fs::read_dir(&self.path)
                    .unwrap_or_else(|_| {
                        panic!("Failed to read directory (crate list): '{:?}'", &self.path)
                    })
                    .map(|cratepath| cratepath.unwrap().path())
                    .collect::<Vec<PathBuf>>();
                // need to take 2 levels into account
                let mut both_levels_vec: Vec<PathBuf> = Vec::new();
                for repo in crate_list {
                    if !repo.is_dir() {
                        continue;
                    }
                    for i in fs::read_dir(&repo)
                        .unwrap_or_else(|_| {
                            panic!("Failed to read directory (repo): '{:?}'", &self.path)
                        })
                        .map(|cratepath| cratepath.unwrap().path())
                    {
                        both_levels_vec.push(i);
                    }
                }
                collection.extend_from_slice(&both_levels_vec);
                collection.sort();

                self.files_calculated = true;
                self.files = collection;
            } else {
                self.files = Vec::new();
            }
            &self.files
        }
    }
}
