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

use crate::cache::dircache::Cache;

use rayon::iter::*;
use walkdir::WalkDir;

pub(crate) struct RegistryIndexCache {
    path: PathBuf,
    total_size: Option<u64>,
    files_calculated: bool,
    files: Vec<PathBuf>,
    number_of_indices: Option<u64>,
}

impl Cache for RegistryIndexCache {
    fn new(path: PathBuf) -> Self {
        // calculate and return as needed
        Self {
            path,
            total_size: None,
            files_calculated: false,
            files: Vec::new(),
            number_of_indices: None,
        }
    }

    #[inline]
    fn path_exists(&self) -> bool {
        self.path.exists()
    }

    fn invalidate(&mut self) {
        self.total_size = None;
        self.files_calculated = false;
    }

    fn total_size(&mut self) -> u64 {
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

    fn files(&mut self) -> &[PathBuf] {
        if self.files_calculated {
            &self.files
        } else {
            if self.path_exists() {
                let walkdir = WalkDir::new(self.path.display().to_string());
                let v = walkdir
                    .into_iter()
                    .map(|d| d.unwrap().into_path())
                    .collect::<Vec<PathBuf>>();
                self.files = v;
            } else {
                self.files = Vec::new();
            }
            &self.files
        }
    }

    fn files_sorted(&mut self) -> &[PathBuf] {
        let _ = self.files(); // prime cache
        self.files.sort();
        &self.files()
    }
}

impl RegistryIndexCache {
    pub(crate) fn number_of_indices(&mut self) -> u64 {
        if self.number_of_indices.is_some() {
            self.number_of_indices.unwrap()
        } else {
            if !self.path_exists() {
                self.number_of_indices = Some(0);
                return 0;
            }

            let number = std::fs::read_dir(&self.path)
                .unwrap()
                // try to filter out FPs in  spurious_files_in_cache test
                .map(|p| p.unwrap().path())
                .filter(|p| {
                    p.is_dir()
                        && p.file_name()
                            .unwrap()
                            .to_os_string()
                            .into_string()
                            .unwrap()
                            .contains('-')
                })
                .count() as u64;
            self.number_of_indices = Some(number);
            number
        }
    }
}
