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

pub(crate) struct GitCheckoutCache {
    path: PathBuf,
    total_size: Option<u64>,
    number_of_checkouts: Option<usize>,
    files_calculated: bool,
    files: Vec<PathBuf>,
    checkouts_calculated: bool,
    checkout_folders: Vec<PathBuf>,
}

impl Cache for GitCheckoutCache {
    fn new(path: PathBuf) -> Self {
        // lazy cache, compute only as needed and save
        Self {
            path,
            total_size: None,
            files_calculated: false,
            files: Vec::new(),
            checkouts_calculated: false,
            checkout_folders: Vec::new(),
            number_of_checkouts: None,
        }
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn invalidate(&mut self) {
        self.total_size = None;
        self.files_calculated = false;
        self.checkouts_calculated = false;
        self.number_of_checkouts = None;
    }

    fn known_to_be_empty(&mut self) {
        self.files = Vec::new();
        self.files_calculated = true;
        self.number_of_checkouts = Some(0);
        self.checkouts_calculated = true;
    }

    fn total_size(&mut self) -> u64 {
        if Self::checkout_folders(self).is_empty() {
            return 0;
        }

        if self.total_size.is_some() {
            self.total_size.unwrap()
        } else if self.path.is_dir() {
            // get the size of all files in path dir
            let total_size = self
                .files()
                .par_iter()
                .map(|f| {
                    fs::metadata(f)
                        .unwrap_or_else(|_| panic!("Failed to read size of file: '{:?}'", f))
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
            &self.files
        } else {
            if self.path_exists() {
                let walkdir = WalkDir::new(self.path.display().to_string());
                let v = walkdir
                    .into_iter()
                    .map(|d| d.unwrap().into_path())
                    .filter(|f| f.exists())
                    .collect::<Vec<PathBuf>>();
                self.files = v;
            } else {
                self.total_size = Some(0);
                self.files = Vec::new();
                self.files_calculated = true;
                self.number_of_checkouts = Some(0);
                self.checkouts_calculated = true;
            }
            &self.files
        }
    }

    fn files_sorted(&mut self) -> &[PathBuf] {
        let _ = self.files(); // prime cache
        self.files.sort();
        self.files()
    }

    fn items(&mut self) -> &[PathBuf] {
        todo!()
    }

    fn number_of_items(&mut self) -> usize {
        todo!()
    }
}

impl GitCheckoutCache {
    pub(crate) fn number_of_files_at_depth_2(&mut self) -> usize {
        let root_dir_depth = self.path.iter().count();
        if self.number_of_checkouts.is_some() {
            self.number_of_checkouts.unwrap()
        } else if self.path_exists() {
            // dir must exist, dir must be as deep as ${path}+2
            let count = self
                .files
                .par_iter()
                .filter(|p| p.is_dir())
                .filter(|p| p.iter().count() == root_dir_depth + 2)
                .count();
            self.number_of_checkouts = Some(count);
            count
        } else {
            self.known_to_be_empty();
            0
        }
    }

    pub(crate) fn checkout_folders(&mut self) -> &[PathBuf] {
        if self.checkouts_calculated {
            &self.checkout_folders
        } else {
            if self.path_exists() {
                let mut collection = Vec::new();

                let crate_list = fs::read_dir(&self.path)
                    .unwrap_or_else(|_| panic!("Failed to read directory: '{:?}'", &self.path))
                    .map(|cratepath| cratepath.unwrap().path())
                    .filter(|p| p.is_dir())
                    .collect::<Vec<PathBuf>>();
                // need to take 2 levels into account
                let mut both_levels_vec: Vec<PathBuf> = Vec::new();
                for repo in crate_list {
                    for i in fs::read_dir(&repo)
                        .unwrap_or_else(|_| panic!("Failed to read directory: '{:?}'", &repo))
                        .map(|cratepath| cratepath.unwrap().path())
                        .filter(|f| f.is_dir())
                    {
                        both_levels_vec.push(i);
                    }
                }
                collection.extend_from_slice(&both_levels_vec);

                self.checkouts_calculated = true;
                self.checkout_folders = collection;
            } else {
                self.checkout_folders = Vec::new();
            }
            &self.checkout_folders
        }
    }

    pub(crate) fn checkout_folders_sorted(&mut self) -> &[PathBuf] {
        let _ = self.checkout_folders(); // prime cache
        self.checkout_folders.sort();
        &self.checkout_folders
    }
}
