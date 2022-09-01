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
use std::path::Path;

use crate::cache::caches::Cache;
use crate::cache::*;
use crate::tables::format_table;
use crate::top_items::common::*;

use humansize::{file_size_opts, FileSize};
use rayon::prelude::*;

#[derive(Debug)]
struct BinInfo {
    name: String,
    size: u64,
}

impl BinInfo {
    fn new(path: &Path) -> Self {
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let size = fs::metadata(path)
            .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &path.display()))
            .len();
        Self { name, size }
    }

    fn size_string(&self) -> String {
        self.size.file_size(file_size_opts::DECIMAL).unwrap()
    }
}

#[inline] // only called in one place
fn bininfo_list_from_path(bin_cache: &mut bin::BinaryCache) -> Vec<BinInfo> {
    // returns unsorted!
    bin_cache
        .files()
        .iter()
        .map(|path| BinInfo::new(path))
        .collect::<Vec<BinInfo>>()
}

#[inline] // only called in one place
fn bininfo_list_to_string(limit: u32, mut collections_vec: Vec<BinInfo>) -> String {
    if collections_vec.is_empty() {
        return String::new();
    }
    // sort the BinInfo Vec in reverse
    collections_vec.par_sort_by_key(|b| b.size);
    collections_vec.reverse();

    let mut table_matrix: Vec<Vec<String>> = Vec::with_capacity(collections_vec.len() + 1);

    table_matrix.push(vec!["Name".into(), "Size".into()]); // table header

    for bininfo in collections_vec.into_iter().take(limit as usize) {
        let size = bininfo.size_string();
        table_matrix.push(vec![bininfo.name, size]);
    }

    format_table(&table_matrix, 0)
}

#[inline] // only called in one place
pub(crate) fn binary_stats(path: &Path, limit: u32, bin_cache: &mut bin::BinaryCache) -> String {
    let mut output = String::new();
    // don't crash if the directory does not exist (issue #9)
    if !dir_exists(path) {
        return output;
    }

    writeln!(
        output,
        "\nSummary of: {} ({} total)",
        path.display(),
        bin_cache
            .total_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    )
    .unwrap();

    let collections_vec = bininfo_list_from_path(bin_cache); // this is already sorted

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
    fn bininfo_new_name_dot() {
        let bi = BinInfo {
            name: String::from("ab.cd"),
            size: 1234,
        };
        assert_eq!(bi.name, String::from("ab.cd"));
        assert_eq!(bi.size, 1234);
    }

    #[test]
    fn bininfo_new_cargo_cache() {
        let bi = BinInfo {
            name: String::from("cargo-cache"),
            size: 1337,
        };
        assert_eq!(bi.name, String::from("cargo-cache"));
        assert_eq!(bi.size, 1337);
    }

    #[test]
    fn bininfo_new_cargo_cache_exe() {
        let bi = BinInfo {
            name: String::from("cargo-cache.exe"),
            size: 1337,
        };
        assert_eq!(bi.name, String::from("cargo-cache.exe"));
        assert_eq!(bi.size, 1337);
    }

    #[test]
    fn bininfo_size_str_small_size() {
        let bi = BinInfo {
            name: String::from("abc"),
            size: 123,
        };
        let size = bi.size_string();
        assert_eq!(size, "123 B");
    }

    #[test]
    fn bininfo_size_str_large_size() {
        let bi = BinInfo {
            name: String::from("abc"),
            size: 1_234_567_890,
        };
        let size = bi.size_string();
        assert_eq!(size, "1.23 GB");
    }

    #[test]
    fn bininfo_sort() {
        let bi_a = BinInfo {
            name: String::from("a"),
            size: 5,
        };

        let bi_b = BinInfo {
            name: String::from("b"),
            size: 3,
        };
        let bi_c = BinInfo {
            name: String::from("c"),
            size: 10,
        };

        let mut v = vec![bi_a, bi_b, bi_c];
        v.sort_by_key(|b| b.size);
        let mut order_string = String::new();
        for bi in v {
            write!(order_string, "{:?}", bi).unwrap();
        }
        println!("{}", order_string);
        let mut wanted = String::new();
        for i in &[
            r#"BinInfo { name: "b", size: 3 }"#,
            r#"BinInfo { name: "a", size: 5 }"#,
            r#"BinInfo { name: "c", size: 10 }"#,
        ] {
            wanted.push_str(i);
        }
        assert_eq!(order_string, wanted);
    }

    #[test]
    fn bininfo_sort_stable() {
        let bi_a = BinInfo {
            name: String::from("a"),
            size: 5,
        };

        let bi_b = BinInfo {
            name: String::from("b"),
            size: 5,
        };
        let bi_c = BinInfo {
            name: String::from("c"),
            size: 5,
        };

        let mut v = vec![bi_a, bi_b, bi_c];
        v.par_sort_by_key(|b| b.size);
        let mut order_string = String::new();
        for bi in v {
            write!(order_string, "{:?}", bi).unwrap();
        }
        println!("{}", order_string);
        let mut wanted = String::new();
        for i in &[
            r#"BinInfo { name: "a", size: 5 }"#,
            r#"BinInfo { name: "b", size: 5 }"#,
            r#"BinInfo { name: "c", size: 5 }"#,
        ] {
            wanted.push_str(i);
        }
        assert_eq!(order_string, wanted);
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
        let wanted = String::from("Name        Size\ncargo-cache 1 B\n");
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
        let wanted = String::from("Name    Size\ncrate-B 2 B\ncrate-A 1 B\n");
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
            "Name    Size\n",
            "crate-C 10 B\n",
            "crate-D 6 B\n",
            "crate-E 4 B\n",
            "crate-B 2 B\n",
            "crate-A 1 B\n",
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
        for i in &["Name    Size\n", "crate-A 3 B\n", "crate-A 3 B\n"] {
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
            "Name    Size\n",
            "crate-A 3 B\n",
            "crate-A 3 B\n",
            "crate-A 3 B\n",
        ] {
            wanted.push_str(i);
        }
        assert_eq!(stats, wanted);
    }
}
