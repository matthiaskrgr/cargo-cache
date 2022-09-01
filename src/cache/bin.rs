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

use crate::cache::caches::Cache;

use rayon::iter::*;

pub(crate) struct BinaryCache {
    path: PathBuf,
    number_of_files: Option<usize>,
    total_size: Option<u64>,
    files_calculated: bool,
    files: Vec<PathBuf>,
}

impl BinaryCache {
    pub(crate) fn number_of_files(&mut self) -> usize {
        if let Some(number_of_files) = self.number_of_files {
            number_of_files
        } else if self.path_exists() {
            let count = self.files().len();
            self.number_of_files = Some(count);
            count
        } else {
            0
        }
    }
}

impl Cache for BinaryCache {
    fn new(path: PathBuf) -> Self {
        // init fields lazily and only compute/save values as needed
        Self {
            path,
            number_of_files: None,
            total_size: None,
            files_calculated: false,
            files: Vec::new(),
        }
    }
    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn invalidate(&mut self) {
        self.number_of_files = None;
        self.total_size = None;
        self.files_calculated = false;
    }

    fn known_to_be_empty(&mut self) {
        self.total_size = Some(0);
        self.files = Vec::new();
        self.files_calculated = true;
    }

    fn total_size(&mut self) -> u64 {
        if let Some(total_size) = self.total_size {
            total_size
        } else if self.path().is_dir() {
            let total_size = self
                .files()
                .par_iter()
                .map(|f| {
                    fs::metadata(f)
                        .unwrap_or_else(|_| panic!("Failed to get size of file: '{:?}'", f))
                        .len()
                })
                .sum();
            self.total_size = Some(total_size);
            total_size
        } else {
            self.known_to_be_empty();
            0
        }
    }

    fn files(&mut self) -> &[PathBuf] {
        if self.files_calculated {
            // do nothing and return
        } else {
            self.files = fs::read_dir(self.path())
                .unwrap_or_else(|_| panic!("Failed to read directory: '{:?}'", &self.path))
                .map(|f| f.unwrap().path())
                .filter(|f| f.is_file())
                .collect::<Vec<PathBuf>>();
            self.files_calculated = true;
        }
        &self.files
    }

    fn files_sorted(&mut self) -> &[PathBuf] {
        let _ = self.files(); // prime cache
        self.files.sort();
        self.files()
    }

    fn items(&mut self) -> &[PathBuf] {
        // shell out to files() here
        self.files()
    }

    fn number_of_items(&mut self) -> usize {
        self.files().len()
    }
}
