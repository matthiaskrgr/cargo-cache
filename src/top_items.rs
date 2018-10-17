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

use humansize::{file_size_opts, FileSize};
use rayon::iter::*;
use walkdir::WalkDir;

use crate::library::*;

#[derive(Debug, Clone)]
struct FileDesc {
    name: String,
    size: u64,
}

impl FileDesc {
    fn new_from_reg_src(path: &PathBuf) -> Self {
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
    } // fn new_from_reg_src()

    fn new_from_reg_cache(path: &PathBuf) -> Self {
        let last_item = path.to_str().unwrap().split('/').last().unwrap();
        let mut i = last_item.split('-').collect::<Vec<_>>();
        i.pop();
        let name = i.join("-");
        let size = fs::metadata(&path)
            .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &path.display()))
            .len();

        Self { name, size }
    } // fn new_from_reg_cache

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

    fn new_from_git_checkouts(path: &PathBuf) -> Self {
        //let last_item = path.to_str().unwrap().split('/').last().unwrap();
        //let mut i = last_item.split('-').collect::<Vec<_>>();
        let mut paths = path.to_str().unwrap().split('/').collect::<Vec<&str>>();
        let last = paths.pop().unwrap();
        let last_but_one = paths.pop().unwrap();
        let last_but_2 = paths.pop().unwrap();

        let mut i = vec![last_but_2, last_but_one, last];

        let string = last_but_one
            .split('/')
            .collect::<Vec<_>>()
            .pop()
            .unwrap()
            .to_string();
        let mut vec = string.split('-').collect::<Vec<_>>();
        let _ = vec.pop();
        let name = vec.join("-").to_string();
        i.pop();

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
    } // fn new_from_git_checkouts()
}

pub(crate) fn get_top_crates(limit: u32, ccd: &CargoCachePaths) -> String {
    // run the functions in parallel for a tiny speedup
    let (reg_src_and_cache, git_bare_repos_and_checkouts) = rayon::join(
        || {
            rayon::join(
                || registry_source_stats(&ccd.registry_sources, limit),
                || registry_cache_stats(&ccd.registry_cache, limit),
            )
        },
        || {
            rayon::join(
                || git_repos_bare_stats(&ccd.git_repos_bare, limit),
                || git_checkouts_stats(&ccd.git_checkouts, limit),
            )
        },
    );

    let (reg_src, reg_cache) = reg_src_and_cache;
    let (bare_repos, repo_checkouts) = git_bare_repos_and_checkouts;

    let mut output = String::new();

    output.push_str(&reg_src);
    output.push_str(&reg_cache);
    output.push_str(&bare_repos);
    output.push_str(&repo_checkouts);

    output
}

fn dir_exists(path: &PathBuf) -> bool {
    if !path.exists() {
        eprintln!("Skipping '{}' because it doesn't exist.", path.display());
        return false;
    }
    true
}

// registry src
fn registry_source_stats(path: &PathBuf, limit: u32) -> String {
    let mut output = String::new();
    // don't crash if the directory does not exist (issue #9)
    if !dir_exists(&path) {
        return output;
    }

    output.push_str(&format!("\nSummary of: {}\n", path.display()));

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
        .map(|path| FileDesc::new_from_reg_src(path))
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
                        "{:0>20} {: <width$} src ckt: {: <3} {: <20}  total: {}\n",
                        total_size,
                        current_name,
                        counter,
                        format!("src avg: {: >9}", average_crate_size),
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

// registry cache
fn registry_cache_stats(path: &PathBuf, limit: u32) -> String {
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

// bare git repos
fn git_repos_bare_stats(path: &PathBuf, limit: u32) -> String {
    let mut output = String::new();
    // don't crash if the directory does not exist (issue #9)
    if !dir_exists(&path) {
        return output;
    }

    output.push_str(&format!("\nSummary of: {}\n", path.display()));

    // get list of package all "...\.crate$" files and sort it
    let mut collection = Vec::new();
    let crate_list = fs::read_dir(&path)
        .unwrap()
        .map(|cratepath| cratepath.unwrap().path())
        .collect::<Vec<PathBuf>>();
    collection.extend_from_slice(&crate_list);
    collection.sort();

    let collections_vec = collection
        .iter()
        .map(|path| FileDesc::new_from_git_bare(path))
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
                        "{:0>20} {: <width$} repo: {: <3} {: <20}  total: {}\n",
                        total_size,
                        current_name,
                        counter,
                        format!("repo avg: {: >9}", average_crate_size),
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

// bare git repos
fn git_checkouts_stats(path: &PathBuf, limit: u32) -> String {
    let mut output = String::new();
    // don't crash if the directory does not exist (issue #9)
    if !dir_exists(&path) {
        return output;
    }

    output.push_str(&format!("\nSummary of: {}\n", path.display()));

    // get list of package all "...\.crate$" files and sort it
    let mut collection = Vec::new();

    let crate_list = fs::read_dir(&path)
        .unwrap()
        .map(|cratepath| cratepath.unwrap().path())
        .collect::<Vec<PathBuf>>();
    // need to take 2 levels into account
    let mut both_levels_vec: Vec<PathBuf> = Vec::new();
    for repo in crate_list {
        for i in fs::read_dir(&repo)
            .unwrap()
            .map(|cratepath| cratepath.unwrap().path())
        {
            both_levels_vec.push(i);
        }
    }
    collection.extend_from_slice(&both_levels_vec);

    collection.sort();

    let collections_vec = collection
        .iter()
        .map(|path| FileDesc::new_from_git_checkouts(path))
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
                        "{:0>20} {: <width$} repo ckt: {: <3} {: <20}  total: {}\n",
                        total_size,
                        current_name,
                        counter,
                        format!("ckt avg: {: >9}", average_crate_size),
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
