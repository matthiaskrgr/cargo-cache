// Copyright 2017-2019 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//use std::fs;
use std::path::PathBuf;

pub(crate) trait Cache {
    fn new(path: PathBuf) -> Self;

    fn path_exists(&self) -> bool;

    fn invalidate(&mut self);

    fn total_size(&mut self) -> u64;
}
