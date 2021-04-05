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

use rayon::prelude::*;
use walkdir::WalkDir;

pub(crate) struct GitRepoCache {
    path: PathBuf,
    total_size: Option<u64>,
    files_calculated: bool,
    files: Vec<PathBuf>,
    items_calculated: bool,
    items: Vec<PathBuf>,
    number_of_items: Option<usize>,
}

impl Cache for GitRepoCache {
    fn new(path: PathBuf) -> Self {
        // calculate as needed
        Self {
            path,
            total_size: None,
            files_calculated: false,
            files: Vec::new(),
            items_calculated: false,
            items: Vec::new(),
            number_of_items: None,
        }
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn invalidate(&mut self) {
        self.total_size = None;
        self.files_calculated = false;
        self.items_calculated = false;
        self.number_of_items = None;
    }

    fn known_to_be_empty(&mut self) {
        self.total_size = Some(0);
        self.files = Vec::new();
        self.files_calculated = true;
        self.items_calculated = true;
        self.number_of_items = Some(0);
    }

    fn total_size(&mut self) -> u64 {
        if Self::items(self).is_empty() {
            return 0;
        }

        if let Some(total_size) = self.total_size {
            total_size
        } else if self.path.is_dir() {
            // get the size of all files in path dir
            let total_size = self
                .files()
                .par_iter()
                .filter(|f| f.is_file())
                .map(|f| fs::metadata(f).unwrap().len())
                .sum();
            self.total_size = Some(total_size);
            total_size
        } else {
            self.known_to_be_empty();
            0
        }
    }

    // all files of this cache
    fn files(&mut self) -> &[PathBuf] {
        if self.files_calculated {
            // do nothing and return
        } else if self.path_exists() {
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

    fn files_sorted(&mut self) -> &[PathBuf] {
        let _ = self.files(); // prime cache
        self.files.sort();
        self.files()
    }

    // list of bare git repos
    fn items(&mut self) -> &[PathBuf] {
        if self.items_calculated {
            // do nothing and return
        } else if self.path_exists() {
            let repo_list = fs::read_dir(&self.path)
                .unwrap_or_else(|_| panic!("Failed to read directory: '{:?}'", &self.path))
                .map(|cratepath| cratepath.unwrap().path())
                .filter(|p| p.is_dir())
                .collect::<Vec<PathBuf>>();

            self.items_calculated = true;
            self.items = repo_list;
        } else {
            self.known_to_be_empty();
        }
        &self.items
    }

    // number of bare git repos
    fn number_of_items(&mut self) -> usize {
        if let Some(items_count) = &self.number_of_items {
            return *items_count;
        }

        let count = self.items().len();
        self.number_of_items = Some(count);
        count
    }
}

impl GitRepoCache {
    pub(crate) fn items_sorted(&mut self) -> &[PathBuf] {
        let _ = self.items(); // prime cache
        self.items.sort();
        &self.items
    }
}
