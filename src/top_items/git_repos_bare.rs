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

use crate::top_items::common::*;
use humansize::{file_size_opts, FileSize};
use rayon::iter::*;
use walkdir::WalkDir;

impl FileDesc {
    fn new_from_git_bare(path: &PathBuf) -> Self {
        let last_item = path.to_str().unwrap().split('/').last().unwrap();
        let mut i = last_item.split('-').collect::<Vec<_>>();
        i.pop();
        let name = i.join("-");

        let walkdir = WalkDir::new(path.display().to_string());

        let size = walkdir
            .into_iter()
            .map(|e| e.unwrap().path().to_owned())
            .filter(|f| f.exists())
            .collect::<Vec<_>>()
            .par_iter()
            .map(|f| {
                fs::metadata(f)
                    .unwrap_or_else(|_| {
                        panic!("Failed to get metadata of file '{}'", &path.display())
                    })
                    .len()
            })
            .sum();

        Self { name, size }
    } // fn new_from_git_bare()
}

fn file_desc_from_path(path: &PathBuf) -> Vec<FileDesc> {
    // get list of package all "...\.crate$" files and sort it
    let mut collection = Vec::new();
    let crate_list = fs::read_dir(&path)
        .unwrap()
        .map(|cratepath| cratepath.unwrap().path())
        .collect::<Vec<PathBuf>>();
    collection.extend_from_slice(&crate_list);
    collection.sort();

    collection
        .iter()
        .map(|path| FileDesc::new_from_git_bare(path))
        .collect::<Vec<_>>()

}


fn stats_from_file_desc_list(file_descs: Vec<FileDesc>) -> Vec<String> {
    struct Pair {
        current: Option<FileDesc>,
        previous: Option<FileDesc>,
    }
    // take our list of file information and calculate the actual stats
    let mut summary: Vec<String> = Vec::new();
    let mut line = String::new(); // line we will print
    let mut counter: u32 = 0; // how many of a crate do we have
    let mut total_size: u64 = 0; // total size of these crates
    let mut dbg_line_len: usize = line.len();
    // first find out max_cratename_len
    let max_cratename_len = &file_descs.iter().map(|p| p.name.len()).max().unwrap_or(0);

    // iterate over the fikles
    let mut iter = file_descs.into_iter();

    let mut state = Pair {
        current: None,
        previous: None,
    };

    // start looping
    state.previous = state.current;
    state.current = iter.next();

    // loop until .previous and .current are None which means we are at the end
    while state.previous.is_some() || state.current.is_some() {
        match &state {
            Pair {
                current: None,
                previous: None,
            } => {
                // we reached the end of the queue
            }

            Pair {
                current: Some(current),
                previous: None,
            } => {
                // this should always be first line ever
                debug_assert!(dbg_line_len == 0);
                // compute line but don't save it
                let current_name = &current.name;
                let current_size = &current.size;
                total_size += current_size;
                let total_size_hr = total_size.file_size(file_size_opts::DECIMAL).unwrap();
                counter += 1;
                let average_crate_size = (total_size / u64::from(counter))
                    .file_size(file_size_opts::DECIMAL)
                    .unwrap();
                line = format!(
                    "{:0>20} {: <width$} src ckt: {: <3} {: <20}  total: {}\n",
                    total_size,
                    current_name,
                    counter,
                    format!("src avg: {: >9}", average_crate_size),
                    total_size_hr,
                    width = max_cratename_len
                );
                dbg_line_len = line.len();
            }

            Pair {
                current: Some(current),
                previous: Some(previous),
            } => {
                if current.name == previous.name {
                    // update line but don't save it
                    debug_assert!(dbg_line_len > 0);
                    let current_name = &current.name;
                    let current_size = &current.size;
                    total_size += current_size;
                    let total_size_hr = total_size.file_size(file_size_opts::DECIMAL).unwrap();
                    counter += 1;
                    let average_crate_size = (total_size / u64::from(counter))
                        .file_size(file_size_opts::DECIMAL)
                        .unwrap();
                    line = format!(
                        "{:0>20} {: <width$} src ckt: {: <3} {: <20}  total: {}\n",
                        total_size,
                        current_name,
                        counter,
                        format!("src avg: {: >9}", average_crate_size),
                        total_size_hr,
                        width = max_cratename_len
                    );
                    dbg_line_len = line.len();
                } else if current.name != previous.name {
                    // save old line
                    debug_assert!(dbg_line_len > 0);
                    summary.push(line);
                    // reset counters
                    counter = 0;
                    total_size = 0;
                    // and update line
                    let current_name = &current.name;
                    let current_size = &current.size;
                    total_size += current_size;
                    let total_size_hr = total_size.file_size(file_size_opts::DECIMAL).unwrap();
                    counter += 1;
                    let average_crate_size = (total_size / u64::from(counter))
                        .file_size(file_size_opts::DECIMAL)
                        .unwrap();
                    line = format!(
                        "{:0>20} {: <width$} src ckt: {: <3} {: <20}  total: {}\n",
                        total_size,
                        current_name,
                        counter,
                        format!("src avg: {: >9}", average_crate_size),
                        total_size_hr,
                        width = max_cratename_len
                    );
                    dbg_line_len = line.len();
                }
            }

            Pair {
                current: None,
                previous: Some(_previous),
            } => {
                // save old line
                debug_assert!(dbg_line_len > 0); // line must not be empty
                summary.push(line);
                line = String::new();
                // reset counters
                counter = 0;
                total_size = 0;
            }
        };

        // switch and queue next()
        state.previous = state.current;
        state.current = iter.next();
    }
    // sort the string vector
    summary.sort();
    summary.reverse(); // largest package (biggest number) first
    summary
}


// bare git repos
pub(crate) fn git_repos_bare_stats(path: &PathBuf, limit: u32) -> String {
    let mut output = String::new();
    // don't crash if the directory does not exist (issue #9)
    if !dir_exists(&path) {
        return output;
    }

    output.push_str(&format!("\nSummary of: {}\n", path.display()));


    let collections_vec = file_desc_from_path(&path);
    let mut summary: Vec<String> = stats_from_file_desc_list(collections_vec);

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
