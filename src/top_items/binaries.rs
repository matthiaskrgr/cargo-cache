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

use crate::cache::dircache::DirCache;
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

fn file_desc_from_path(cache: &mut DirCache) -> Vec<FileDesc> {
    cache
        .bin
        .files()
        .iter()
        .map(|path| FileDesc::new_from_binary(path))
        .collect::<Vec<FileDesc>>()
}

fn stats_from_file_desc_list(file_descs: &[FileDesc]) -> Vec<String> {
    // take our list of file information and calculate the actual stats
    let mut summary: Vec<String> = Vec::new();

    for binary in file_descs {
        let size = &binary.size;
        let size_hr = size.file_size(file_size_opts::DECIMAL).unwrap();
        let name = &binary.name;
        let line = format!("{:0>20} {} size: {}\n", size, name, size_hr,);
        summary.push(line);
    }

    // sort the string vector
    summary.sort();
    summary.reverse(); // largest package (biggest number) first
    summary
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

    let collections_vec = file_desc_from_path(&mut cache);
    let summary: Vec<String> = stats_from_file_desc_list(&collections_vec);

    let max_cratename_len = summary
        .iter()
        .take(limit as usize)
        .map(|line| {
            let pkg = line
                .split_whitespace()
                .nth(1) // 0 is the line used for sorting, 1 is package name
                .unwrap();
            pkg.len()
        })
        .max()
        .unwrap_or(0);

    for line in summary.into_iter().take(limit as usize) {
        let mut split = line.split_whitespace();
        let _numbers = split.next();
        let package = split.next().unwrap();

        let size = split.next().unwrap();
        let number = split.next().unwrap();
        let unit = split.next().unwrap();
        debug_assert!(
            !split.next().is_some(),
            "line contained more words than expected!"
        );

        output.push_str(&format!(
            "{: <width$} {} {} {}\n",
            package,
            size,
            number,
            unit,
            width = max_cratename_len
        ));
    }

    output
}

#[cfg(test)]
mod top_crates_binaries {
    use super::*;
    use crate::top_items::common::FileDesc;
    use pretty_assertions::assert_eq;

    #[test]
    fn stats_from_file_desc_none() {
        // empty list
        let list: Vec<FileDesc> = Vec::new();
        let stats: Vec<String> = stats_from_file_desc_list(&list);
        // list should be empty
        let empty: Vec<String> = Vec::new();
        assert_eq!(stats, empty);
    }

    #[test]
    fn stats_from_file_desc_one() {
        let fd = FileDesc {
            name: "cargo-cache".to_string(),
            size: 1,
        };
        let list: Vec<FileDesc> = vec![fd];
        let stats: Vec<String> = stats_from_file_desc_list(&list);
        let wanted: Vec<String> = vec!["00000000000000000001 cargo-cache size: 1 B\n".to_string()];
        assert_eq!(stats, wanted);
    }

    #[test]
    fn stats_from_file_desc_two() {
        let fd1 = FileDesc {
            name: "crate-A".to_string(),
            size: 1,
        };
        let fd2 = FileDesc {
            name: "crate-B".to_string(),
            size: 2,
        };
        let list: Vec<FileDesc> = vec![fd1, fd2];
        let stats: Vec<String> = stats_from_file_desc_list(&list);
        let wanted: Vec<String> = vec![
            "00000000000000000002 crate-B size: 2 B\n".to_string(),
            "00000000000000000001 crate-A size: 1 B\n".to_string(),
        ];
        assert_eq!(stats, wanted);
    }

    #[test]
    fn stats_from_file_desc_multiple() {
        let fd1 = FileDesc {
            name: "crate-A".to_string(),
            size: 1,
        };
        let fd2 = FileDesc {
            name: "crate-B".to_string(),
            size: 2,
        };
        let fd3 = FileDesc {
            name: "crate-C".to_string(),
            size: 10,
        };
        let fd4 = FileDesc {
            name: "crate-D".to_string(),
            size: 6,
        };
        let fd5 = FileDesc {
            name: "crate-E".to_string(),
            size: 4,
        };
        let list: Vec<FileDesc> = vec![fd1, fd2, fd3, fd4, fd5];
        let stats: Vec<String> = stats_from_file_desc_list(&list);
        let wanted: Vec<String> = vec![
            "00000000000000000010 crate-C size: 10 B\n".to_string(),
            "00000000000000000006 crate-D size: 6 B\n".to_string(),
            "00000000000000000004 crate-E size: 4 B\n".to_string(),
            "00000000000000000002 crate-B size: 2 B\n".to_string(),
            "00000000000000000001 crate-A size: 1 B\n".to_string(),
        ];
        assert_eq!(stats, wanted);
    }

    // @TODO: we should not actually encounter several files of identical names...
    // maybe add an assert?
    #[test]
    fn stats_from_file_desc_same_name_2_one() {
        let fd1 = FileDesc {
            name: "crate-A".to_string(),
            size: 3,
        };
        let fd2 = FileDesc {
            name: "crate-A".to_string(),
            size: 3,
        };

        let list: Vec<FileDesc> = vec![fd1, fd2];
        let stats: Vec<String> = stats_from_file_desc_list(&list);
        let wanted: Vec<String> = vec![
            "00000000000000000003 crate-A size: 3 B\n".to_string(),
            "00000000000000000003 crate-A size: 3 B\n".to_string(),
        ];
        assert_eq!(stats, wanted);
    }

    #[test]
    fn stats_from_file_desc_same_name_3_one() {
        let fd1 = FileDesc {
            name: "crate-A".to_string(),
            size: 3,
        };
        let fd2 = FileDesc {
            name: "crate-A".to_string(),
            size: 3,
        };
        let fd3 = FileDesc {
            name: "crate-A".to_string(),
            size: 3,
        };

        let list: Vec<FileDesc> = vec![fd1, fd2, fd3];
        let stats: Vec<String> = stats_from_file_desc_list(&list);
        let wanted: Vec<String> = vec![
            "00000000000000000003 crate-A size: 3 B\n".to_string(),
            "00000000000000000003 crate-A size: 3 B\n".to_string(),
            "00000000000000000003 crate-A size: 3 B\n".to_string(),
        ];
        assert_eq!(stats, wanted);
    }

}
