// Copyright 2017-2018 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

use rayon::iter::*;

pub(crate) struct GitCheckoutCache {
    path: PathBuf,
    total_size: Option<u64>,
    number_of_checkouts: Option<usize>,
    files_calculated: bool,
    files: Vec<PathBuf>,
    // number_of_files: Option<usize>,
    checkouts_calculated: bool,
    checkout_folders: Vec<PathBuf>,
}

impl GitCheckoutCache {
    pub(crate) fn new(path: PathBuf) -> Self {
        // lazy cache, compute and save only as needed
        Self {
            path,
            total_size: None,
            // number_of_files: None,
            files_calculated: false,
            files: Vec::new(),
            checkouts_calculated: false,
            checkout_folders: Vec::new(),
            number_of_checkouts: None,
        }
    }

    pub(crate) fn invalidate(&mut self) {
        self.total_size = None;
        self.files_calculated = false;
        self.checkouts_calculated = false;
        self.number_of_checkouts = None;
    }

    #[inline]
    pub(crate) fn path_exists(&mut self) -> bool {
        self.path.exists()
    }

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
                .map(|f| fs::metadata(f).unwrap().len())
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

    pub(crate) fn checkout_folders(&mut self) -> &[PathBuf] {
        if self.checkouts_calculated {
            &self.checkout_folders
        } else {
            if self.path_exists() {
                let mut collection = Vec::new();

                let crate_list = fs::read_dir(&self.path)
                    .unwrap()
                    .map(|cratepath| cratepath.unwrap().path())
                    .collect::<Vec<PathBuf>>();
                // need to take 2 levels into account
                let mut both_levels_vec: Vec<PathBuf> = Vec::new();
                for repo in crate_list {
                    for i in fs::read_dir(&repo)
                        .unwrap()
                        .map(|cratepath| cratepath.unwrap().path())
                    {
                        both_levels_vec.push(i);
                    }
                }
                collection.extend_from_slice(&both_levels_vec);
                collection.sort();

                self.checkouts_calculated = true;
                self.checkout_folders = collection;
            } else {
                self.checkout_folders = Vec::new();
            }
            &self.checkout_folders
        }
    }

    /*
    pub(crate) fn number_of_checkouts(&mut self) -> Option<usize> {
        if self.number_of_checkouts.is_some() {
            self.number_of_checkouts
        } else {
            let c = self.checkout_folders().iter().count();
            self.number_of_checkouts = Some(c);
            self.number_of_checkouts
        }
    }
    */

    /*
        pub(crate) fn number_of_files(&mut self) -> usize {
            if self.number_of_checkouts.is_some() {
                self.number_of_checkouts.unwrap()
            } else {
                // we don't have the value cached
                if self.path_exists() {
                    let count = self.files().len();
                    self.number_of_checkouts = Some(count);
                    count
                } else {
                    0
                }
            }
        }
    */
}
