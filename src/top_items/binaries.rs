// Copyright 2018 Matthias Krüger. See the COPYRIGHT
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
use crate::top_items::common::*;
use humansize::{file_size_opts, FileSize};

#[derive(Clone, Debug, Eq)]
struct BinInfo {
    name: String,
    size: u64,
}

impl BinInfo {
    fn new(path: &PathBuf) -> Self {
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let size = fs::metadata(&path)
            .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &path.display()))
            .len();
        Self { name, size }
    }

    fn size_string(&self) -> String {
        let mut s = String::from("size: ");
        s.push_str(&self.size.file_size(file_size_opts::DECIMAL).unwrap());
        s
    }
}

impl PartialOrd for BinInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BinInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.size.cmp(&other.size)
    }
}

impl PartialEq for BinInfo {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size
    }
}

#[inline(always)]
fn bininfo_list_from_path(cache: &mut DirCache) -> Vec<BinInfo> {
    // returns unsorted!
    cache
        .bin
        .files()
        .iter()
        .map(|path| BinInfo::new(path))
        .collect::<Vec<BinInfo>>()
}

#[inline(always)]
fn bininfo_list_to_string(limit: u32, mut collections_vec: Vec<BinInfo>) -> String {
    // sort the BinInfo Vec in reverse
    collections_vec.sort();
    collections_vec.reverse();

    let mut output = String::new();

    let max_cratename_len = collections_vec
        .iter()
        .take(limit as usize)
        .map(|b| b.name.len())
        .max()
        .unwrap_or(0);

    for bininfo in collections_vec.into_iter().take(limit as usize) {
        output.push_str(&format!(
            "{: <width$} {}\n",
            bininfo.name,
            bininfo.size_string(),
            width = max_cratename_len + TOP_CRATES_SPACING,
        ));
    }
    output
}

pub(crate) fn binary_stats(path: &PathBuf, limit: u32, mut cache: &mut DirCache) -> String {
    let mut output = String::new();
    // don't crash if the directory does not exist (issue #9)
    if !dir_exists(path) {
        return output;
    }

    output.push_str(&format!(
        "\nSummary of: {} ({} total)\n",
        path.display(),
        cache
            .bin
            .total_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    ));

    let collections_vec = bininfo_list_from_path(&mut cache); // this is already sorted

    let bininfo_string = bininfo_list_to_string(limit, collections_vec);
    output.push_str(&bininfo_string);

    output
}

#[cfg(test)]
mod bininfo_struct {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn bininfo_new() {
        let bi = BinInfo {
            name: String::from("abc"),
            size: 123,
        };
        assert_eq!(bi.name, String::from("abc"));
        assert_eq!(bi.size, 123);
    }

    #[test]
    fn bininfo_size_str_small_size() {
        let bi = BinInfo {
            name: String::from("abc"),
            size: 123,
        };
        let size = bi.size_string();
        assert_eq!(size, "size: 123 B");
    }

    #[test]
    fn bininfo_size_str_large_size() {
        let bi = BinInfo {
            name: String::from("abc"),
            size: 1234567890,
        };
        let size = bi.size_string();
        assert_eq!(size, "size: 1.23 GB");
    }
}

#[cfg(test)]
mod top_crates_binaries {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn stats_from_file_desc_none() {
        // empty list
        let list: Vec<BinInfo> = Vec::new();
        let stats: String = bininfo_list_to_string(1, list);

        let empty = String::new();
        assert_eq!(stats, empty);
    }

    #[test]
    fn stats_from_file_desc_one() {
        let bi = BinInfo {
            name: "cargo-cache".to_string(),
            size: 1,
        };
        let list: Vec<BinInfo> = vec![bi];
        let stats: String = bininfo_list_to_string(1, list);
        let wanted = "cargo-cache    size: 1 B\n".to_string();
        assert_eq!(stats, wanted);
    }

    #[test]
    fn stats_from_file_desc_two() {
        let bi1 = BinInfo {
            name: "crate-A".to_string(),
            size: 1,
        };
        let bi2 = BinInfo {
            name: "crate-B".to_string(),
            size: 2,
        };
        let list: Vec<BinInfo> = vec![bi1, bi2];
        let stats: String = bininfo_list_to_string(2, list);
        let mut wanted = String::from("crate-B    size: 2 B\n");
        wanted.push_str("crate-A    size: 1 B\n");

        assert_eq!(stats, wanted);
    }

    #[test]
    fn stats_from_file_desc_multiple() {
        let bi1 = BinInfo {
            name: "crate-A".to_string(),
            size: 1,
        };
        let bi2 = BinInfo {
            name: "crate-B".to_string(),
            size: 2,
        };
        let bi3 = BinInfo {
            name: "crate-C".to_string(),
            size: 10,
        };
        let bi4 = BinInfo {
            name: "crate-D".to_string(),
            size: 6,
        };
        let bi5 = BinInfo {
            name: "crate-E".to_string(),
            size: 4,
        };
        let list: Vec<BinInfo> = vec![bi1, bi2, bi3, bi4, bi5];
        let stats: String = bininfo_list_to_string(10, list);
        let mut wanted = String::new();
        for i in &[
            "crate-C    size: 10 B\n",
            "crate-D    size: 6 B\n",
            "crate-E    size: 4 B\n",
            "crate-B    size: 2 B\n",
            "crate-A    size: 1 B\n",
        ] {
            wanted.push_str(i);
        }
        assert_eq!(stats, wanted);
    }

    // @TODO: we should not actually encounter several files of identical names...
    // maybe add an assert?
    #[test]
    fn stats_from_file_desc_same_name_2_one() {
        let bi1 = BinInfo {
            name: "crate-A".to_string(),
            size: 3,
        };
        let bi2 = BinInfo {
            name: "crate-A".to_string(),
            size: 3,
        };

        let list: Vec<BinInfo> = vec![bi1, bi2];
        let stats: String = bininfo_list_to_string(2, list);
        let mut wanted = String::new();
        for i in &["crate-A    size: 3 B\n", "crate-A    size: 3 B\n"] {
            wanted.push_str(i);
        }
        assert_eq!(stats, wanted);
    }

    #[test]
    fn stats_from_file_desc_same_name_3_one() {
        let bi1 = BinInfo {
            name: "crate-A".to_string(),
            size: 3,
        };
        let bi2 = BinInfo {
            name: "crate-A".to_string(),
            size: 3,
        };
        let bi3 = BinInfo {
            name: "crate-A".to_string(),
            size: 3,
        };

        let list: Vec<BinInfo> = vec![bi1, bi2, bi3];
        let stats: String = bininfo_list_to_string(4, list);
        let mut wanted = String::new();
        for i in &[
            "crate-A    size: 3 B\n",
            "crate-A    size: 3 B\n",
            "crate-A    size: 3 B\n",
        ] {
            wanted.push_str(i);
        }
        assert_eq!(stats, wanted);
    }

}
