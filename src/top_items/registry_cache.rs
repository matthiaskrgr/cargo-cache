// Copyright 2017-2018 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::cmp::Ordering;
use std::fs;
use std::path::PathBuf;

use crate::cache::dircache::DirCache;
use crate::top_items::common::{dir_exists, TOP_CRATES_SPACING};
use humansize::{file_size_opts, FileSize};

#[derive(Clone, Debug)]
struct FileDesc {
    path: PathBuf,
    name: String,
    size: u64,
}

impl FileDesc {
    pub(crate) fn new_from_reg_cache(path: &PathBuf) -> Self {
        let last_item = path.file_name().unwrap().to_str().unwrap().to_string();
        let mut i = last_item.split('-').collect::<Vec<_>>();
        i.pop();
        let name = i.join("-");
        let size = fs::metadata(&path)
            .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &path.display()))
            .len();

        Self {
            path: path.into(),
            name,
            size,
        }
    } // fn new_from_reg_cache()
} // impl FileDesc

#[derive(Clone, Debug, Eq)]
pub(crate) struct RgchInfo {
    name: String,
    size: u64,
    counter: u32,
    total_size: u64, // sort by this
}

impl RgchInfo {
    fn new(path: &PathBuf, counter: u32, total_size: u64) -> Self {
        let name: String;
        let size: u64;
        if path.exists() {
            size = fs::metadata(&path)
                .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &path.display()))
                .len();
            let n = path.file_name().unwrap().to_str().unwrap().to_string();
            let mut v = n.split('-').collect::<Vec<_>>();
            v.pop();
            name = v.join("-");
        } else {
            name = path
                .file_name()
                .unwrap()
                .to_os_string()
                .into_string()
                .unwrap();
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

impl PartialOrd for RgchInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RgchInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.total_size.cmp(&other.total_size)
    }
}

impl PartialEq for RgchInfo {
    fn eq(&self, other: &Self) -> bool {
        self.total_size == other.total_size
    }
}

// registry cache (extracted tarballs)
fn file_desc_list_from_path(cache: &mut DirCache) -> Vec<FileDesc> {
    cache
        .registry_cache
        .files()
        .iter()
        .map(|path| FileDesc::new_from_reg_cache(path))
        .collect::<Vec<_>>()
}

fn stats_from_file_desc_list(file_descs: Vec<FileDesc>) -> Vec<RgchInfo> {
    struct Pair {
        current: Option<FileDesc>,
        previous: Option<FileDesc>,
    }
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
            }

            Pair {
                current: Some(current),
                previous: None,
            } => {
                // this should always be first line ever
                //@TODO assert that its empty
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
                    // @todo assert that regcacheinfo is not empty
                    let current_size = &current.size;
                    total_size += current_size;
                    counter += 1;

                    regcacheinfo = RgchInfo::new(&current.path, counter, total_size);
                } else if current.name != previous.name {
                    // save old line
                    //  todo assert that regcacheinfo is not empty
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
                // todo assert that regcacheinfo is not empty
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
    // sort the RepoINfo Vec in reverse, biggest item first
    collections_vec.sort();
    collections_vec.reverse();
    let mut output = String::new();
    let max_cratename_len = collections_vec
        .iter()
        .take(limit as usize)
        .map(|p| p.name.len())
        .max()
        .unwrap_or(0);
    for regcache in collections_vec.into_iter().take(limit as usize) {
        let average_crate_size = (regcache.total_size / u64::from(regcache.counter))
            .file_size(file_size_opts::DECIMAL)
            .unwrap();
        let avg_string = format!("src avg: {: >9}", average_crate_size);
        output.push_str(&format!(
            "{: <width$} src ckt: {: <3} {: <20} total: {}\n",
            regcache.name,
            regcache.counter,
            avg_string,
            regcache
                .total_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap(),
            width = max_cratename_len + TOP_CRATES_SPACING,
        ));
    }
    output
}

// registry cache
pub(crate) fn registry_cache_stats(path: &PathBuf, limit: u32, mut cache: &mut DirCache) -> String {
    let mut stdout = String::new();
    // don't crash if the directory does not exist (issue #9)
    if !dir_exists(path) {
        return stdout;
    }

    stdout.push_str(&format!(
        "\nSummary of: {} ({} total)\n",
        path.display(),
        cache
            .registry_cache
            .total_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    ));

    let file_descs: Vec<FileDesc> = file_desc_list_from_path(&mut cache);
    let summary: Vec<RgchInfo> = stats_from_file_desc_list(file_descs);
    let string = regcache_list_to_string(limit, summary);
    stdout.push_str(&string);

    stdout
}

#[cfg(test)]
mod top_crates_git_repos_bare {
    use super::*;
    use pretty_assertions::assert_eq;

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
        let wanted = String::from("crateA    src ckt: 1   src avg:       1 B   total: 1 B\n");
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
            "crate-B    src ckt: 1   src avg:       2 B   total: 2 B\n",
            "crate-A    src ckt: 1   src avg:       1 B   total: 1 B\n",
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
            "crate-C    src ckt: 1   src avg:      10 B   total: 10 B\n",
            "crate-D    src ckt: 1   src avg:       6 B   total: 6 B\n",
            "crate-E    src ckt: 1   src avg:       4 B   total: 4 B\n",
            "crate-B    src ckt: 1   src avg:       2 B   total: 2 B\n",
            "crate-A    src ckt: 1   src avg:       1 B   total: 1 B\n",
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
        let wanted = String::from("crate-A    src ckt: 2   src avg:       3 B   total: 6 B\n");
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
        let wanted = String::from("crate-A    src ckt: 3   src avg:       3 B   total: 9 B\n");
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
        let wanted = String::from("crate-A    src ckt: 3   src avg:       6 B   total: 18 B\n");
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
            "crate-C    src ckt: 2   src avg:      50 B   total: 100 B\n",
            "crate-A    src ckt: 3   src avg:       6 B   total: 18 B\n",
            "crate-B    src ckt: 2   src avg:       5 B   total: 10 B\n",
            "crate-D    src ckt: 1   src avg:       1 B   total: 1 B\n",
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

            black_box(is);
        });
    }

}
