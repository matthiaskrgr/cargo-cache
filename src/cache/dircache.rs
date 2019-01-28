// Copyright 2017-2019 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::path::PathBuf;

pub(crate) trait Cache {
    // creates a new cache object
    fn new(path: PathBuf) -> Self;
    // checks if the path to the directory of an object exists
    fn path_exists(&self) -> bool;
    // invalidates the cache
    fn invalidate(&mut self);
    // total size of the cache
    fn total_size(&mut self) -> u64;
    // list of files of the cache
    fn files(&mut self) -> &[PathBuf];
    // list of files of the cache, sorted
    fn files_sorted(&mut self) -> &[PathBuf];
}
