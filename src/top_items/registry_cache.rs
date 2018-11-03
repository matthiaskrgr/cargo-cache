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

use crate::top_items::common::{dir_exists, FileDesc};
use humansize::{file_size_opts, FileSize};

impl FileDesc {
    pub(crate) fn new_from_reg_cache(path: &PathBuf) -> Self {
        let last_item = path.to_str().unwrap().split('/').last().unwrap();
        let mut i = last_item.split('-').collect::<Vec<_>>();
        i.pop();
        let name = i.join("-");
        let size = fs::metadata(&path)
            .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &path.display()))
            .len();

        Self { name, size }
    } // fn new_from_reg_cache()
}

// registry cache
pub(crate) fn registry_cache_stats(path: &PathBuf, limit: u32) -> String {
    let mut output = String::new();
    // don't crash if the directory does not exist (issue #9)
    if !dir_exists(&path) {
        return output;
    }

    output.push_str(&format!("\nSummary of: {}\n", path.display()));

    // get list of package all "...\.crate$" files and sort it
    let mut collection = Vec::new();

    for repo in fs::read_dir(path).unwrap() {
        let crate_list = fs::read_dir(&repo.unwrap().path())
            .unwrap()
            .map(|cratepath| cratepath.unwrap().path())
            .collect::<Vec<PathBuf>>();

        collection.extend_from_slice(&crate_list);
    }
    collection.sort();

    let collections_vec = collection
        .iter()
        .map(|path| FileDesc::new_from_reg_cache(path))
        .collect::<Vec<_>>();

    let mut summary: Vec<String> = Vec::new();
    let mut current_name = String::new();
    let mut counter: u32 = 0;
    let mut total_size: u64 = 0;

    // first find out max_cratename_len
    let max_cratename_len = collections_vec.iter().map(|p| p.name.len()).max().unwrap();

    #[cfg_attr(feature = "cargo-clippy", allow(clippy::if_not_else))]
    collections_vec.into_iter().for_each(|pkg| {
        {
            if pkg.name != current_name {
                // don't push the first empty string
                if !current_name.is_empty() {
                    let total_size_hr = total_size.file_size(file_size_opts::DECIMAL).unwrap();
                    let average_crate_size = (total_size / u64::from(counter))
                        .file_size(file_size_opts::DECIMAL)
                        .unwrap();

                    summary.push(format!(
                        "{:0>20} {: <width$} archives: {: <3} {: <20}  total: {}\n",
                        total_size,
                        current_name,
                        counter,
                        format!("crate avg: {: >9}", average_crate_size),
                        total_size_hr,
                        width = max_cratename_len
                    ));
                } // !current_name.is_empty()
                  // new package, reset counting
                current_name = pkg.name;
                counter = 1;
                total_size = pkg.size;
            } else {
                counter += 1;
                total_size += pkg.size;
            }
        }
    });

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
