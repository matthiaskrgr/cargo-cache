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

pub(crate) struct GitRepoCache {
    path: PathBuf,
    total_size: Option<u64>,
    number_of_repos: Option<usize>,
    files_calculated: bool,
    files: Vec<PathBuf>,
    // number_of_files: Option<usize>,
    repos_calculated: bool,
    bare_repos_folders: Vec<PathBuf>,
}

impl Cache for GitRepoCache {
    fn new(path: PathBuf) -> Self {
        // calculate as needed
        Self {
            path,
            // number_of_files_recursively: None,
            total_size: None,
            // number_of_files: None,
            files_calculated: false,
            files: Vec::new(),
            repos_calculated: false,
            bare_repos_folders: Vec::new(),
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
                .map(|f| fs::metadata(f).unwrap().len())
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
                    .filter(|d| d.is_file())
                    .collect::<Vec<PathBuf>>();
                self.files = v;
            } else {
                self.files = Vec::new();
            }
            &self.files
        }
    }
}

impl GitRepoCache {
    pub(crate) fn number_of_checkout_repos(&mut self) -> Option<usize> {
        if self.number_of_repos.is_some() {
            self.number_of_repos
        } else {
            let c = self.bare_repo_folders().iter().count();
            // println!("{:?}", self.checkout_folders().iter());
            self.number_of_repos = Some(c);
            self.number_of_repos
        }
    }

    pub(crate) fn bare_repo_folders(&mut self) -> &[PathBuf] {
        if self.repos_calculated {
            &self.bare_repos_folders
        } else {
            if self.path_exists() {
                let mut crate_list = fs::read_dir(&self.path)
                    .unwrap_or_else(|_| panic!("Failed to read directory: '{:?}'", &self.path))
                    .map(|cratepath| cratepath.unwrap().path())
                    .collect::<Vec<PathBuf>>();

                crate_list.sort();

                self.repos_calculated = true;
                self.bare_repos_folders = crate_list;
            } else {
                self.bare_repos_folders = Vec::new();
            }
            &self.bare_repos_folders
        }
    }

    /*
        pub(crate) fn number_of_files(&mut self) -> usize {
            if self.number_of_repos.is_some() {
                self.number_of_repos.unwrap()
            } else {
                // we don't have the value cached
                if self.path_exists() {
                    let count = self.files().len();
                    self.number_of_repos = Some(count);
                    count
                } else {
                    0
                }
            }
        }

        pub(crate) fn number_of_files_at_depth_2(&mut self) -> usize {
            let root_dir_depth = self.path.iter().count();
            if self.number_of_repos.is_some() {
                self.number_of_repos.unwrap()
            } else {
                // we don't have the value cached
                if self.path_exists() {
                    // dir must exist, dir must be as depth ${path}+2
                    let count = self
                        .files
                        .iter()
                        .filter(|p| p.is_dir())
                        .filter(|p| p.iter().count() == root_dir_depth + 2)
                        .count();
                    self.number_of_repos = Some(count);
                    count
                } else {
                    0
                }
            }
        }
    */
}
