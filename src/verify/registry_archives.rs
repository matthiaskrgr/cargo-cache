// Copyright 2017-2022 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Verify the registry sources and archives

use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};

use crate::cache::caches::RegistrySuperCache;
use crate::cache::*;
use crate::remove::remove_file;

use flate2::read::GzDecoder;
use rayon::iter::*;
use tar::Archive;
use walkdir::WalkDir;

use 