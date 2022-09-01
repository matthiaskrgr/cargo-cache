// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cache::caches::Cache;
use crate::cache::*;
use crate::tables::format_table;
use crate::top_items::common::{dir_exists, FileDesc, Pair};

use humansize::{file_size_opts, FileSize};
use rayon::prelude::*;
use walkdir::WalkDir;

#[inline]
fn name_from_path(path: &Path) -> String {
    // path:  ~/.cargo/git/checkouts/cargo-cache-16826c8e13331adc/0f9966c
    let dir = path
        .iter()
        .map(|p| p.to_str().unwrap())
        .rev()
        .nth(1)
        .unwrap();
    // dir: cargo-cache-16826c8e13331adc
    // skip the last element
    let mut all_but_last = dir.split('-').rev().skip(1).collect::<Vec<_>>(); //  cannot .rev() again, need to collect again
                                                                             // vec: cache cargo
    all_but_last.reverse();
    // vec: cargo cache
    all_but_last.join("-")
    // "cargo-cache"
}

impl FileDesc {
    fn new_from_git_checkouts(path: &Path) -> Self {
        let name = name_from_path(path);

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

        Self {
            name,
            size,
            path: path.into(),
        }
    } // fn new_from_git_checkouts()
} // impl FileDesc

#[derive(Debug)]
pub(crate) struct ChkInfo {
    name: String,
    #[allow(unused)]
    size: u64,
    counter: u32,
    total_size: u64, // sorted by this
}

impl ChkInfo {
    // sorted by total_size!

    fn new(path: &Path, counter: u32, total_size: u64) -> Self {
        let name: String;
        let size: u64;
        if path.exists() {
            size = fs::metadata(path)
                .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &path.display()))
                .len();
            let mut p = path.to_path_buf();
            let _ = p.pop();
            let name_tmp = p.file_name().unwrap().to_str().unwrap().to_string();
            let mut tmp = name_tmp.split('-').collect::<Vec<_>>();
            let _ = tmp.pop();
            name = tmp.join("-");
        } else {
            let name_tmp = path.file_name().unwrap().to_str().unwrap().to_string();
            size = 0;
            name = name_tmp;
        }
        Self {
            name,
            size,
            counter,
            total_size,
        }
    }
}

#[inline]
fn file_desc_from_path(git_checkouts_cache: &mut git_checkouts::GitCheckoutCache) -> Vec<FileDesc> {
    // get list of package all "...\.crate$" files and sort it
    git_checkouts_cache
        .items_sorted()
        .iter()
        .map(|path| FileDesc::new_from_git_checkouts(path))
        .collect::<Vec<_>>()
}

#[inline]
fn stats_from_file_desc_list(file_descs: Vec<FileDesc>) -> Vec<ChkInfo> {
    // take our list of file information and calculate the actual stats
    let mut out: Vec<ChkInfo> = Vec::new();
    let mut chkinfo: ChkInfo = ChkInfo::new(&PathBuf::from("ERROR 1/err1"), 0, 0);
    let mut counter: u32 = 0; // how many of a crate do we have
    let mut total_size: u64 = 0; // total size of these crates

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
                unreachable!("dead code triggered: while loop condition did not hold inside match");
            }

            Pair {
                current: Some(current),
                previous: None,
            } => {
                // this should always be first line ever
                // compute line but don't save it
                let current_size = &current.size;
                total_size += current_size;
                counter += 1;

                chkinfo = ChkInfo::new(&current.path, counter, total_size);
            }

            Pair {
                current: Some(current),
                previous: Some(previous),
            } => {
                if current.name == previous.name {
                    // update line but don't save it
                    let current_size = &current.size;
                    total_size += current_size;
                    counter += 1;

                    chkinfo = ChkInfo::new(&current.path, counter, total_size);
                } else if current.name != previous.name {
                    // save old line
                    out.push(chkinfo);
                    // reset counters
                    counter = 0;
                    total_size = 0;
                    // and update line
                    let current_size = &current.size;
                    total_size += current_size;
                    counter += 1;

                    chkinfo = ChkInfo::new(&current.path, counter, total_size);
                }
            }

            Pair {
                current: None,
                previous: Some(_previous),
            } => {
                // save old line
                out.push(chkinfo);
                chkinfo = ChkInfo::new(&PathBuf::from("ERROR 2/err2"), 0, 0); // uninit

                // reset counters
                counter = 0;
                total_size = 0;
            }
        };

        // switch and queue next()
        state.previous = state.current;
        state.current = iter.next();
    }
    out
}

#[inline] // only used in one place
fn chkout_list_to_string(limit: u32, mut collections_vec: Vec<ChkInfo>) -> String {
    if collections_vec.is_empty() {
        return String::new();
    }

    // sort the ChkInfo Vec in reverse
    collections_vec.par_sort_by_key(|gc| gc.total_size);
    collections_vec.reverse();
    let mut table_matrix: Vec<Vec<String>> = Vec::with_capacity(collections_vec.len() + 1);

    table_matrix.push(vec![
        String::from("Name"),
        String::from("Count"),
        String::from("Average"),
        String::from("Total"),
    ]);

    for chkout in collections_vec.into_iter().take(limit as usize) {
        #[allow(clippy::integer_division)]
        let average_size = (chkout.total_size / u64::from(chkout.counter))
            .file_size(file_size_opts::DECIMAL)
            .unwrap();
        let total_size = chkout
            .total_size
            .file_size(file_size_opts::DECIMAL)
            .unwrap();

        table_matrix.push(vec![
            chkout.name,
            chkout.counter.to_string(),
            average_size,
            total_size,
        ]);
    }

    format_table(&table_matrix, 0)
}

#[inline]
pub(crate) fn git_checkouts_stats(
    path: &Path,
    limit: u32,
    checkouts_cache: &mut git_checkouts::GitCheckoutCache,
) -> String {
    let mut output = String::new();
    // don't crash if the directory does not exist (issue #9)
    if !dir_exists(path) {
        return output;
    }

    writeln!(
        output,
        "\nSummary of: {} ({} total)",
        path.display(),
        checkouts_cache
            .total_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    )
    .unwrap();

    let collections_vec = file_desc_from_path(checkouts_cache);
    let summary: Vec<ChkInfo> = stats_from_file_desc_list(collections_vec);

    let tmp = chkout_list_to_string(limit, summary);
    output.push_str(&tmp);

    output
}

#[cfg(test)]
mod top_crates_git_checkouts {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn name_from_pb_cargo_cache() {
        let path = PathBuf::from(
            "/home/matthias/.cargo/git/checkouts/cargo-cache-16826c8e13331adc/0f9966c",
        );
        let name = name_from_path(&path);
        assert_eq!(name, "cargo-cache");
    }

    #[test]
    fn name_from_pb_alacritty() {
        let path =
            PathBuf::from("/home/matthias/.cargo/git/checkouts/alacritty-de74975f496aa2c0/f4fc9eb");
        let name = name_from_path(&path);
        assert_eq!(name, "alacritty");
    }

    #[test]
    fn stats_from_file_desc_none() {
        // empty list
        let list: Vec<FileDesc> = Vec::new();
        let stats = stats_from_file_desc_list(list);
        let is = chkout_list_to_string(4, stats);
        let empty = String::new();
        assert_eq!(is, empty);
    }

    #[test]
    fn stats_from_file_desc_one() {
        let fd = FileDesc {
            path: PathBuf::from("crateA"),
            name: "crateA".to_string(),
            size: 1,
        };
        let list_fd: Vec<FileDesc> = vec![fd];
        let list_cb: Vec<ChkInfo> = stats_from_file_desc_list(list_fd);
        let is: String = chkout_list_to_string(1, list_cb);
        let wanted = String::from("Name   Count Average Total\ncrateA 1     1 B     1 B\n");
        assert_eq!(is, wanted);
    }

    #[test]
    fn stats_from_file_desc_two() {
        let fd1 = FileDesc {
            path: PathBuf::from("crate-A"),
            name: "crate-A".to_string(),
            size: 1,
        };
        let fd2 = FileDesc {
            path: PathBuf::from("crate-B"),
            name: "crate-B".to_string(),
            size: 2,
        };
        let list_fd: Vec<FileDesc> = vec![fd1, fd2];
        let list_cb: Vec<ChkInfo> = stats_from_file_desc_list(list_fd);
        let is: String = chkout_list_to_string(3, list_cb);

        let mut wanted = String::new();
        for i in &[
            "Name    Count Average Total\n",
            "crate-B 1     2 B     2 B\n",
            "crate-A 1     1 B     1 B\n",
        ] {
            wanted.push_str(i);
        }
        assert_eq!(is, wanted);
    }

    #[test]
    fn stats_from_file_desc_multiple() {
        let fd1 = FileDesc {
            path: PathBuf::from("crate-A"),
            name: "crate-A".to_string(),
            size: 1,
        };
        let fd2 = FileDesc {
            path: PathBuf::from("crate-B"),
            name: "crate-B".to_string(),
            size: 2,
        };
        let fd3 = FileDesc {
            path: PathBuf::from("crate-C"),
            name: "crate-C".to_string(),
            size: 10,
        };
        let fd4 = FileDesc {
            path: PathBuf::from("crate-D"),
            name: "crate-D".to_string(),
            size: 6,
        };
        let fd5 = FileDesc {
            path: PathBuf::from("crate-E"),
            name: "crate-E".to_string(),
            size: 4,
        };
        let list_fd: Vec<FileDesc> = vec![fd1, fd2, fd3, fd4, fd5];
        let list_cb: Vec<ChkInfo> = stats_from_file_desc_list(list_fd);

        let is: String = chkout_list_to_string(6, list_cb);

        let mut wanted = String::new();
        for i in &[
            "Name    Count Average Total\n",
            "crate-C 1     10 B    10 B\n",
            "crate-D 1     6 B     6 B\n",
            "crate-E 1     4 B     4 B\n",
            "crate-B 1     2 B     2 B\n",
            "crate-A 1     1 B     1 B\n",
        ] {
            wanted.push_str(i);
        }
        assert_eq!(is, wanted);
    }

    #[test]
    fn stats_from_file_desc_same_name_2_one() {
        let fd1 = FileDesc {
            path: PathBuf::from("crate-A"),
            name: "crate-A".to_string(),
            size: 3,
        };
        let fd2 = FileDesc {
            path: PathBuf::from("crate-A"),
            name: "crate-A".to_string(),
            size: 3,
        };

        let list_fd: Vec<FileDesc> = vec![fd1, fd2];
        let list_cb: Vec<ChkInfo> = stats_from_file_desc_list(list_fd);
        let is: String = chkout_list_to_string(2, list_cb);
        let wanted = String::from("Name    Count Average Total\ncrate-A 2     3 B     6 B\n");
        assert_eq!(is, wanted);
    }

    #[test]
    fn stats_from_file_desc_same_name_3_one() {
        let fd1 = FileDesc {
            path: PathBuf::from("crate-A"),
            name: "crate-A".to_string(),
            size: 3,
        };
        let fd2 = FileDesc {
            path: PathBuf::from("crate-A"),
            name: "crate-A".to_string(),
            size: 3,
        };
        let fd3 = FileDesc {
            path: PathBuf::from("crate-A"),
            name: "crate-A".to_string(),
            size: 3,
        };

        let list_fd: Vec<FileDesc> = vec![fd1, fd2, fd3];

        let list_cb: Vec<ChkInfo> = stats_from_file_desc_list(list_fd);
        let is: String = chkout_list_to_string(3, list_cb);
        let wanted = String::from("Name    Count Average Total\ncrate-A 3     3 B     9 B\n");
        assert_eq!(is, wanted);
    }

    #[test]
    fn stats_from_file_desc_same_name_3_one_2() {
        let fd1 = FileDesc {
            path: PathBuf::from("crate-A"),
            name: "crate-A".to_string(),
            size: 2,
        };
        let fd2 = FileDesc {
            path: PathBuf::from("crate-A"),
            name: "crate-A".to_string(),
            size: 4,
        };
        let fd3 = FileDesc {
            path: PathBuf::from("crate-A"),
            name: "crate-A".to_string(),
            size: 12,
        };

        let list_fd: Vec<FileDesc> = vec![fd1, fd2, fd3];
        let list_cb: Vec<ChkInfo> = stats_from_file_desc_list(list_fd);
        let is: String = chkout_list_to_string(3, list_cb);
        let wanted = String::from("Name    Count Average Total\ncrate-A 3     6 B     18 B\n");
        assert_eq!(is, wanted);
    }

    #[test]
    fn stats_from_file_desc_multi() {
        let fd1 = FileDesc {
            path: PathBuf::from("crate-A"),
            name: "crate-A".to_string(),
            size: 2,
        };
        let fd2 = FileDesc {
            path: PathBuf::from("crate-A"),
            name: "crate-A".to_string(),
            size: 4,
        };
        let fd3 = FileDesc {
            path: PathBuf::from("crate-A"),
            name: "crate-A".to_string(),
            size: 12,
        };

        let fd4 = FileDesc {
            path: PathBuf::from("crate-B"),
            name: "crate-B".to_string(),
            size: 2,
        };
        let fd5 = FileDesc {
            path: PathBuf::from("crate-B"),
            name: "crate-B".to_string(),
            size: 8,
        };

        let fd6 = FileDesc {
            path: PathBuf::from("crate-C"),
            name: "crate-C".to_string(),
            size: 0,
        };
        let fd7 = FileDesc {
            path: PathBuf::from("crate-C"),
            name: "crate-C".to_string(),
            size: 100,
        };

        let fd8 = FileDesc {
            path: PathBuf::from("crate-D"),
            name: "crate-D".to_string(),
            size: 1,
        };

        let list_fd: Vec<FileDesc> = vec![fd1, fd2, fd3, fd4, fd5, fd6, fd7, fd8];
        let list_cb: Vec<ChkInfo> = stats_from_file_desc_list(list_fd);
        let is: String = chkout_list_to_string(5, list_cb);

        let mut wanted = String::new();

        for i in &[
            "Name    Count Average Total\n",
            "crate-C 2     50 B    100 B\n",
            "crate-A 3     6 B     18 B\n",
            "crate-B 2     5 B     10 B\n",
            "crate-D 1     1 B     1 B\n",
        ] {
            wanted.push_str(i);
        }
        assert_eq!(is, wanted);
    }
}

#[cfg(all(test, feature = "bench"))]
mod benchmarks {
    use super::*;
    use crate::test::black_box;
    use crate::test::Bencher;

    #[bench]
    fn bench_few(b: &mut Bencher) {
        let fd1 = FileDesc {
            path: PathBuf::from("crate-A"),
            name: "crate-A".to_string(),
            size: 2,
        };
        let fd2 = FileDesc {
            path: PathBuf::from("crate-A"),
            name: "crate-A".to_string(),
            size: 4,
        };
        let fd3 = FileDesc {
            path: PathBuf::from("crate-A"),
            name: "crate-A".to_string(),
            size: 12,
        };

        let fd4 = FileDesc {
            path: PathBuf::from("crate-B"),
            name: "crate-B".to_string(),
            size: 2,
        };
        let fd5 = FileDesc {
            path: PathBuf::from("crate-B"),
            name: "crate-B".to_string(),
            size: 8,
        };

        let fd6 = FileDesc {
            path: PathBuf::from("crate-C"),
            name: "crate-C".to_string(),
            size: 0,
        };
        let fd7 = FileDesc {
            path: PathBuf::from("crate-C"),
            name: "crate-C".to_string(),
            size: 100,
        };

        let fd8 = FileDesc {
            path: PathBuf::from("crate-D"),
            name: "crate-D".to_string(),
            size: 1,
        };

        let list_fd: Vec<FileDesc> = vec![fd1, fd2, fd3, fd4, fd5, fd6, fd7, fd8];

        b.iter(|| {
            let list_fd = list_fd.clone(); // @FIXME  don't?
            let list_cb: Vec<ChkInfo> = stats_from_file_desc_list(list_fd);
            let is: String = chkout_list_to_string(5, list_cb);

            let _ = black_box(is);
        });
    }
}
