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
    pub(crate) fn new(path: PathBuf) -> Self {
        // init fields lazily and only compute/save values as needed
        Self {
            path,
            number_of_files: None,
            total_size: None,
            files_calculated: false,
            files: Vec::new(),
        }
    }

    pub(crate) fn invalidate(&mut self) {
        self.number_of_files = None;
        self.total_size = None;
        self.files_calculated = false;
    }

    #[inline]
    pub(crate) fn path_exists(&mut self) -> bool {
        self.path.exists()
    }

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
            let total_size = self
                .files()
                .iter()
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
            self.files = fs::read_dir(&self.path)
                .unwrap()
                .map(|f| f.unwrap().path())
                .collect::<Vec<PathBuf>>();
            self.files_calculated = true;
            &self.files
        }
    }
}
