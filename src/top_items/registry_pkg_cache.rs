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

use crate::cache::caches::RegistrySuperCache;
use crate::cache::registry_pkg_cache;
use crate::tables::format_table;
use crate::top_items::common::{dir_exists, FileDesc, Pair};

use humansize::{file_size_opts, FileSize};
use rayon::prelude::*;

#[inline]
fn name_from_path(path: &Path) -> String {
    // path:  .../xz2-0.1.4.crate
    let last_item = path.file_name().unwrap().to_str().unwrap().to_string();
    // last_item: xz2-0.1.4.crate
    let mut v = last_item.split('-').collect::<Vec<_>>();
    let _ = v.pop(); // remove everything after last "-"
                     // xz2
    v.join("-") // rejoin remaining elements with "-"
}

impl FileDesc {
    pub(crate) fn new_from_reg_cache(path: &Path) -> Self {
        let name = name_from_path(path);
        let size = fs::metadata(path)
            .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &path.display()))
            .len();

        Self {
            path: path.into(),
            name,
            size,
        }
    } // fn new_from_reg_cache()
} // impl FileDesc

#[derive(Debug)]
pub(crate) struct RgchInfo {
    name: String,
    #[allow(unused)]
    size: u64,
    counter: u32,
    total_size: u64, // sort by this
}

impl RgchInfo {
    fn new(path: &Path, counter: u32, total_size: u64) -> Self {
        let name: String;
        let size: u64;
        if path.exists() {
            size = fs::metadata(path)
                .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &path.display()))
                .len();
            let n = path.file_name().unwrap().to_str().unwrap().to_string();
            let mut v = n.split('-').collect::<Vec<_>>();
            let _ = v.pop();
            name = v.join("-");
        } else {
            name = path.file_name().unwrap().to_str().unwrap().to_string();

            size = 0;
        }

        Self {
            name,
            size,
            counter,
            total_size,
        }
    }
}

// registry cache (extracted tarballs)
fn file_desc_list_from_path(
    registry_pkg_cache: &mut registry_pkg_cache::RegistryPkgCaches,
) -> Vec<FileDesc> {
    registry_pkg_cache
        .files_sorted()
        .iter()
        .map(|path| FileDesc::new_from_reg_cache(path))
        .collect::<Vec<FileDesc>>()
}

fn stats_from_file_desc_list(file_descs: Vec<FileDesc>) -> Vec<RgchInfo> {
    // take our list of file information and calculate the actual stats

    let mut out: Vec<RgchInfo> = Vec::new();
    let mut regcacheinfo: RgchInfo = RgchInfo::new(&PathBuf::from("ERROR 1/err1"), 0, 0);
    let mut counter: u32 = 0; // how many of a crate do we have
    let mut total_size: u64 = 0; // total size of these crates
                                 // iterate over the files
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

                regcacheinfo = RgchInfo::new(&current.path, counter, total_size);
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

                    regcacheinfo = RgchInfo::new(&current.path, counter, total_size);
                } else if current.name != previous.name {
                    // save old line
                    out.push(regcacheinfo);
                    // reset counters
                    counter = 0;
                    total_size = 0;
                    // and update line
                    let current_size = &current.size;
                    total_size += current_size;
                    counter += 1;

                    regcacheinfo = RgchInfo::new(&current.path, counter, total_size);
                }
            }

            Pair {
                current: None,
                previous: Some(_previous),
            } => {
                // save old line
                out.push(regcacheinfo);
                regcacheinfo = RgchInfo::new(&PathBuf::from("ERROR 2/err2"), 0, 0);

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

pub(crate) fn regcache_list_to_string(limit: u32, mut collections_vec: Vec<RgchInfo>) -> String {
    if collections_vec.is_empty() {
        return String::new();
    }

    // sort the RepoInfo Vec in reverse, biggest item first
    collections_vec.par_sort_by_key(|rpc| rpc.total_size);
    collections_vec.reverse();
    let mut table_matrix: Vec<Vec<String>> = Vec::with_capacity(collections_vec.len() + 1);

    table_matrix.push(vec![
        String::from("Name"),
        String::from("Count"),
        String::from("Average"),
        String::from("Total"),
    ]);

    for regcache in collections_vec.into_iter().take(limit as usize) {
        #[allow(clippy::integer_division)]
        let average_size = (regcache.total_size / u64::from(regcache.counter))
            .file_size(file_size_opts::DECIMAL)
            .unwrap();

        let total_size = regcache
            .total_size
            .file_size(file_size_opts::DECIMAL)
            .unwrap();

        table_matrix.push(vec![
            regcache.name,
            regcache.counter.to_string(),
            average_size,
            total_size,
        ]);
    }
    format_table(&table_matrix, 0)
}

// registry cache
pub(crate) fn registry_pkg_cache_stats(
    path: &Path,
    limit: u32,
    registry_pkg_caches: &mut registry_pkg_cache::RegistryPkgCaches,
) -> String {
    let mut stdout = String::new();
    // don't crash if the directory does not exist (issue #9)
    if !dir_exists(path) {
        return stdout;
    }

    writeln!(
        stdout,
        "\nSummary of: {} ({} total)",
        path.display(),
        registry_pkg_caches
            .total_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    )
    .unwrap();

    let file_descs: Vec<FileDesc> = file_desc_list_from_path(registry_pkg_caches);
    let summary: Vec<RgchInfo> = stats_from_file_desc_list(file_descs);
    let string = regcache_list_to_string(limit, summary);
    stdout.push_str(&string);

    stdout
}

#[cfg(test)]
mod top_crates_registry_pkg_cache {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn name_from_pb_cargo_cache() {
        let path = PathBuf::from(
            "/home/matthias/.cargo/registry/cache/github.com-1ecc6299db9ec823/cargo-cache-0.1.1.crate"
        );
        let name = name_from_path(&path);
        assert_eq!(name, "cargo-cache");
    }

    #[test]
    fn name_from_pb_alacritty() {
        let path =
            PathBuf::from("/home/matthias/.cargo/registry/cache/github.com-1ecc6299db9ec823/alacritty-0.0.1.crate");
        let name = name_from_path(&path);
        assert_eq!(name, "alacritty");
    }

    #[test]
    fn stats_from_file_desc_none() {
        // empty list
        let list: Vec<FileDesc> = Vec::new();
        let stats = stats_from_file_desc_list(list);
        let is = regcache_list_to_string(4, stats);
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
        let list_cb: Vec<RgchInfo> = stats_from_file_desc_list(list_fd);
        let is: String = regcache_list_to_string(1, list_cb);
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
        let list_cb: Vec<RgchInfo> = stats_from_file_desc_list(list_fd);
        let is: String = regcache_list_to_string(3, list_cb);

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
        let list_cb: Vec<RgchInfo> = stats_from_file_desc_list(list_fd);

        let is: String = regcache_list_to_string(6, list_cb);

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
        let list_cb: Vec<RgchInfo> = stats_from_file_desc_list(list_fd);
        let is: String = regcache_list_to_string(2, list_cb);
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

        let list_cb: Vec<RgchInfo> = stats_from_file_desc_list(list_fd);
        let is: String = regcache_list_to_string(3, list_cb);
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
        let list_cb: Vec<RgchInfo> = stats_from_file_desc_list(list_fd);
        let is: String = regcache_list_to_string(3, list_cb);
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
        let list_cb: Vec<RgchInfo> = stats_from_file_desc_list(list_fd);
        let is: String = regcache_list_to_string(5, list_cb);

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
            let list_cb: Vec<RgchInfo> = stats_from_file_desc_list(list_fd);
            let is: String = regcache_list_to_string(5, list_cb);

            let _ = black_box(is);
        });
    }
}
