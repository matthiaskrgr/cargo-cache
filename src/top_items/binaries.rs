// Copyright 2018 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fs;
use std::path::PathBuf;

use crate::top_items::common::*;
use humansize::{file_size_opts, FileSize};

impl FileDesc {
    fn new_from_binary(path: &PathBuf) -> Self {
        let name = path.file_name().unwrap().to_str().unwrap().to_string();

        let size = fs::metadata(&path)
            .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &path.display()))
            .len();

        Self { name, size }
    } // fn new_from_git_bare()
}

fn file_desc_from_path(path: &PathBuf) -> Vec<FileDesc> {
    let mut crate_list = fs::read_dir(&path)
        .unwrap()
        .map(|cratepath| cratepath.unwrap().path())
        .collect::<Vec<PathBuf>>();

    crate_list.sort();

    crate_list
        .iter()
        .map(|path| FileDesc::new_from_binary(path))
        .collect::<Vec<FileDesc>>()
}

fn stats_from_file_desc_list(file_descs: &[FileDesc]) -> Vec<String> {
    // take our list of file information and calculate the actual stats
    let mut summary: Vec<String> = Vec::new();

    // first find out max_cratename_len
    let max_cratename_len = &file_descs.iter().map(|p| p.name.len()).max().unwrap_or(0);

    for binary in file_descs {
        let size = &binary.size;
        let size_hr = size.file_size(file_size_opts::DECIMAL).unwrap();
        let name = &binary.name;
        let line = format!(
            "{:0>20} {: <width$} size: {}\n",
            size,
            name,
            size_hr,
            width = max_cratename_len,
        );
        summary.push(line);
    }

    // sort the string vecregistry_sourcestor
    summary.sort();
    summary.reverse(); // largest package (biggest number) first
    summary
}

// bare git repos
pub(crate) fn binary_stats(path: &PathBuf, limit: u32) -> String {
    let mut output = String::new();
    // don't crash if the directory does not exist (issue #9)
    if !dir_exists(&path) {
        return output;
    }

    output.push_str(&format!("\nSummary of: {}\n", path.display()));

    let collections_vec = file_desc_from_path(&path);
    let mut summary: Vec<String> = stats_from_file_desc_list(&collections_vec);

    summary.sort();
    summary.reverse();

    for (c, i) in summary.into_iter().enumerate() {
        if c == limit as usize {
            break;
        }
        let i = &i[21..]; // remove first word used for sorting
        output.push_str(i);
    }

    output
}

// @TODO add tests
