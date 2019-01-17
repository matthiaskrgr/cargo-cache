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

pub(crate) struct RegistrySourceCache {
    path: PathBuf,
    total_size: Option<u64>,
    number_of_repos: Option<usize>,
    files_calculated: bool,
    files: Vec<PathBuf>,
    repos_calculated: bool,
    checkout_folders: Vec<PathBuf>,
}

impl Cache for RegistrySourceCache {
    fn new(path: PathBuf) -> Self {
        // calculate only as needed and cache
        Self {
            path,
            total_size: None,
            files_calculated: false,
            files: Vec::new(),
            repos_calculated: false,
            checkout_folders: Vec::new(),
            number_of_repos: None,
        }
    }

    #[inline]
    fn path_exists(&self) -> bool {
        self.path.exists()
    }

    fn invalidate(&mut self) {
        self.total_size = None;
        self.files_calculated = false;
        self.repos_calculated = false;
        self.number_of_repos = None;
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
}

impl RegistrySourceCache {
    pub(crate) fn number_of_files_at_depth_2(&mut self) -> usize {
        let root_dir_depth = self.path.iter().count();
        if self.number_of_repos.is_some() {
            self.number_of_repos.unwrap()
        } else if self.path_exists() {
            // dir must exist, dir must be as depth ${path}+2
            let count = self
                .files
                .par_iter()
                .filter(|p| p.is_dir())
                .filter(|p| p.iter().count() == root_dir_depth + 2)
                .count();
            self.number_of_repos = Some(count);
            count
        } else {
            0
        }
    }

    pub(crate) fn checkout_folders(&mut self) -> &[PathBuf] {
        if self.repos_calculated {
            &self.checkout_folders
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
                    if repo.is_file() {
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

                self.repos_calculated = true;
                self.checkout_folders = collection;
            } else {
                self.checkout_folders = Vec::new();
            }
            &self.checkout_folders
        }
    }
}
