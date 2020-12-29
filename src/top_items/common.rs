// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct Pair<T> {
    pub(crate) current: Option<T>,
    pub(crate) previous: Option<T>,
}

#[derive(Clone, Debug)]
pub(crate) struct FileDesc {
    pub(crate) path: PathBuf,
    pub(crate) name: String,
    pub(crate) size: u64,
}

pub(crate) fn dir_exists(path: &Path) -> bool {
    // check if a directory exists and print an warning message if not
    if path.exists() {
        true
    } else {
        eprintln!("Skipping '{}' because it doesn't exist.", path.display());
        false
    }
}
