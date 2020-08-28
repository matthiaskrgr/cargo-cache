// Copyright 2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// "cargo cache trim" command

// except according to those terms.

use std::fs;
use std::path::PathBuf;

use crate::cache::caches::*;
use crate::cache::*;
use crate::library::*;
use crate::library::{CargoCachePaths, Error};
use crate::remove::*;
use cargo_metadata::{CargoOpt, MetadataCommand};

use clap::ArgMatches;
use humansize::{file_size_opts, FileSize};
use rayon::prelude::*;

use regex::Regex;
use walkdir::WalkDir;

/*
fn gather_all_cache_items(
    cargo_cache_paths: &CargoCachePaths,
    manifest_path: &Option<&str>,
    checkouts_cache: &mut git_checkouts::GitCheckoutCache,
    bare_repos_cache: &mut git_repos_bare::GitRepoCache,
    registry_pkg_caches: &mut registry_pkg_cache::RegistryPkgCaches,
    registry_sources_caches: &mut registry_sources::RegistrySourceCaches,
    dry_run: bool,
    size_changed: &mut bool,
) -> () {

}

*/
