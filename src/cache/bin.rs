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

pub(crate) struct BinaryCache {
    path: PathBuf,
    number_of_files: Option<usize>,
    total_size: Option<u64>,
    files_calculated: bool,
    files: Vec<PathBuf>,
}

impl BinaryCache {
    // use this to init
    pub(crate) fn new(path: PathBuf) -> Self {
        // calculate only if it's needed
        Self {
            path,
            number_of_files: None,
            //number_of_files_recursively: None,
            total_size: None,
            files_calculated: false,
            files: Vec::new(),
        }
    }

    pub(crate) fn path_exists(&mut self) -> bool {
        self.path.exists()
    }

    pub(crate) fn number_of_files(&mut self) -> usize {
        if self.number_of_files.is_some() {
            self.number_of_files.unwrap()
        } else {
            // we don't have the value cached
            if self.path_exists() {
                let count = self.files().len();
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
            // save and return
            self.files = fs::read_dir(&self.path)
                .unwrap()
                .map(|f| f.unwrap().path())
                .collect::<Vec<PathBuf>>();
            self.files_calculated = true;
            &self.files
        }
    }
}
