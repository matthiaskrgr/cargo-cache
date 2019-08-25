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
    // returns reference to the root path of the cache
    fn path(&self) -> &PathBuf;
    // checks if the path to the directory of an object exists
    fn path_exists(&self) -> bool {
        self.path().exists()
    }
    // invalidates the cache
    fn invalidate(&mut self);
    // total size of the cache
    fn total_size(&mut self) -> u64;
    // list of files of the cache
    fn files(&mut self) -> &[PathBuf];
    // list of files of the cache, sorted
    fn files_sorted(&mut self) -> &[PathBuf];
}

/// this is a super cache that is used to hold and access multiple multiple subcaches
/// example: `RegistrySuperCache`: `RegistryIndices`, `RegistrySubCache`: `RegistryIndex`
pub(crate) trait RegistrySuperCache {
    type SubCache;

    /// creates a new supercache object
    fn new(path: PathBuf) -> Self;
    /// invalidates all contained subcaches
    fn invalidate(&mut self);
    // returns a list of subcaches, (items that impls RegistrySubCache trait)
    fn caches(&mut self) -> &mut Vec<Self::SubCache>;
    /// total size of the cache
    fn files(&mut self) -> Vec<PathBuf>;
    /// list of files of the cache, sorted
    fn files_sorted(&mut self) -> Vec<PathBuf>;
    /// number of files in total
    fn total_size(&mut self) -> u64;
    /// number of subcaches
    fn number_of_items(&mut self) -> usize;
    /// total number of files over all subcaches
    fn total_number_of_files(&mut self) -> usize;
}

/// each registry is stored in a seperate subcache
pub(crate) trait RegistrySubCache {
    /// create a new subcache
    fn new(path: PathBuf) -> Self;
    // returns the name of the registry
    fn name(&self) -> &str;
    /// check if the root path of the Cache exists
    fn path_exists(&self) -> bool {
        self.path().exists()
    }
    /// invalidates the cache
    fn invalidate(&mut self);
    /// total size of the cache
    fn total_size(&mut self) -> u64;
    /// list of files contained in the cache
    fn files(&mut self) -> &[PathBuf];
    /// number of files in the cache
    fn number_of_files(&mut self) -> usize;
    /// sorted list of the files
    fn files_sorted(&mut self) -> &[PathBuf];
    // path of the cache
    fn path(&self) -> &PathBuf;
}

/// get the name of a cache directory from a path.
/// if the full path is bla/github.com-1ecc6299db9ec823, we return github.com
pub(crate) fn get_cache_name(path: &PathBuf) -> String {
    // save only the last path element bla/github.com-1ecc6299db9ec823 -> github.com-1ecc6299db9ec823
    let file_name = path.file_name();
    let last = file_name.unwrap().to_str().unwrap().to_string();
    let mut v = last.split('-').collect::<Vec<_>>();
    // remove the hash
    let _ = v.pop();
    // recombine as String
    v.join("-")
}
