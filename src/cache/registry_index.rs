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

pub(crate) struct RegistryIndexCache {
    path: PathBuf,
    total_size: Option<u64>,
    files_calculated: bool,
    files: Vec<PathBuf>,
    #[allow(unused)]
    number_of_files: Option<usize>,
}

impl RegistryIndexCache {
    // use this to init
    pub(crate) fn new(path: PathBuf) -> Self {
        // calculate only if it's needed
        Self {
            path,
            total_size: None,
            number_of_files: None,
            files_calculated: false,
            files: Vec::new(),
        }
    }

    pub(crate) fn path_exists(&mut self) -> bool {
        self.path.exists()
    }

    #[allow(unused)]
    pub(crate) fn number_of_files(&mut self) -> usize {
        if self.number_of_files.is_some() {
            self.number_of_files.unwrap()
        } else {
            // we don't have the value cached
            if self.path_exists() {
                let count = self.files().iter().count();
                self.number_of_files = Some(count);
                count
            } else {
                0
            }
        }
    }

    pub(crate) fn total_size(&mut self) -> u64 {
        if self.total_size.is_some() {
            self.total_size.unwrap()
        } else {
            // is it cached?
            if self.path.is_dir() {
                // get the size of all files in path dir
                self.files()
                    .iter()
                    .map(|f| fs::metadata(f).unwrap().len())
                    .sum()
            } else {
                0
            }
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
}
