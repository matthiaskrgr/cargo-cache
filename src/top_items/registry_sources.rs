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

use crate::top_items::common::{dir_exists, FileDesc};

impl FileDesc {
    pub(crate) fn new_from_reg_src(path: &PathBuf) -> Self {
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
}

fn file_desc_list_from_path(path: &PathBuf) -> Vec<FileDesc> {
    let mut collection = Vec::new();

    for repo in fs::read_dir(path).unwrap() {
        let crate_list = fs::read_dir(&repo.unwrap().path())
            .unwrap()
            .map(|cratepath| cratepath.unwrap().path())
            .collect::<Vec<PathBuf>>();

        collection.extend_from_slice(&crate_list);
    }
    collection.sort();

    collection
        .iter()
        .map(|path| FileDesc::new_from_reg_src(path))
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
    let mut dbg_line_len: usize = line.len() ;
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

// registry src
pub(crate) fn registry_source_stats(path: &PathBuf, limit: u32) -> String {
    let mut stdout = String::new();
    // don't crash if the directory does not exist (issue #9)
    if !dir_exists(&path) {
        return stdout;
    }

    stdout.push_str(&format!("\nSummary of: {}\n", path.display()));

    let file_descs: Vec<FileDesc> = file_desc_list_from_path(&path);
    let summary: Vec<String> = stats_from_file_desc_list(file_descs);

    for (count, data) in summary.into_iter().enumerate() {
        if count == limit as usize {
            break;
        }
        let data = &data[21..]; // remove first word used for sorting
        stdout.push_str(data);
    }

    stdout
}

#[cfg(test)]
mod top_crates_registry_sources {
    use super::*;
    use crate::top_items::common::FileDesc;
    use pretty_assertions::assert_eq;

    #[test]
    fn stats_from_file_desc_none() {
        // empty list
        let list: Vec<FileDesc> = Vec::new();
        let stats: Vec<String> = stats_from_file_desc_list(list);
        // list should be empty
        let empty: Vec<String> = Vec::new();
        assert_eq!(stats, empty);
    }

    #[test]
    fn stats_from_file_desc_one() {
        let fd = FileDesc {
            name: "crate".to_string(),
            size: 1,
        };
        let list: Vec<FileDesc> = vec![fd];
        let stats: Vec<String> = stats_from_file_desc_list(list);
        let wanted: Vec<String> = vec![
            "00000000000000000001 crate src ckt: 1   src avg:       1 B    total: 1 B\n"
                .to_string(),
        ];
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
        let stats: Vec<String> = stats_from_file_desc_list(list);
        let wanted: Vec<String> = vec![
            "00000000000000000002 crate-B src ckt: 1   src avg:       2 B    total: 2 B\n"
                .to_string(),
            "00000000000000000001 crate-A src ckt: 1   src avg:       1 B    total: 1 B\n"
                .to_string(),
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
        let stats: Vec<String> = stats_from_file_desc_list(list);
        let wanted: Vec<String> = vec![
            "00000000000000000010 crate-C src ckt: 1   src avg:      10 B    total: 10 B\n"
                .to_string(),
            "00000000000000000006 crate-D src ckt: 1   src avg:       6 B    total: 6 B\n"
                .to_string(),
            "00000000000000000004 crate-E src ckt: 1   src avg:       4 B    total: 4 B\n"
                .to_string(),
            "00000000000000000002 crate-B src ckt: 1   src avg:       2 B    total: 2 B\n"
                .to_string(),
            "00000000000000000001 crate-A src ckt: 1   src avg:       1 B    total: 1 B\n"
                .to_string(),
        ];
        assert_eq!(stats, wanted);
    }

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
        let stats: Vec<String> = stats_from_file_desc_list(list);
        let wanted: Vec<String> = vec![
            "00000000000000000006 crate-A src ckt: 2   src avg:       3 B    total: 6 B\n"
                .to_string(),
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
        let stats: Vec<String> = stats_from_file_desc_list(list);
        let wanted: Vec<String> = vec![
            "00000000000000000009 crate-A src ckt: 3   src avg:       3 B    total: 9 B\n"
                .to_string(),
        ];
        assert_eq!(stats, wanted);
    }

    #[test]
    fn stats_from_file_desc_same_name_3_one_2() {
        let fd1 = FileDesc {
            name: "crate-A".to_string(),
            size: 2,
        };
        let fd2 = FileDesc {
            name: "crate-A".to_string(),
            size: 4,
        };
        let fd3 = FileDesc {
            name: "crate-A".to_string(),
            size: 12,
        };

        let list: Vec<FileDesc> = vec![fd1, fd2, fd3];
        let stats: Vec<String> = stats_from_file_desc_list(list);
        let wanted: Vec<String> = vec![
            "00000000000000000018 crate-A src ckt: 3   src avg:       6 B    total: 18 B\n"
                .to_string(),
        ];
        assert_eq!(stats, wanted);
    }

    #[test]
    fn stats_from_file_desc_multi() {
        let fd1 = FileDesc {
            name: "crate-A".to_string(),
            size: 2,
        };
        let fd2 = FileDesc {
            name: "crate-A".to_string(),
            size: 4,
        };
        let fd3 = FileDesc {
            name: "crate-A".to_string(),
            size: 12,
        };

        let fd4 = FileDesc {
            name: "crate-B".to_string(),
            size: 2,
        };
        let fd5 = FileDesc {
            name: "crate-B".to_string(),
            size: 8,
        };

        let fd6 = FileDesc {
            name: "crate-C".to_string(),
            size: 0,
        };
        let fd7 = FileDesc {
            name: "crate-C".to_string(),
            size: 100,
        };

        let fd8 = FileDesc {
            name: "crate-D".to_string(),
            size: 1,
        };

        let list: Vec<FileDesc> = vec![fd1, fd2, fd3, fd4, fd5, fd6, fd7, fd8];
        let stats: Vec<String> = stats_from_file_desc_list(list);
        let wanted: Vec<String> = vec![
            "00000000000000000100 crate-C src ckt: 2   src avg:      50 B    total: 100 B\n"
                .to_string(),
            "00000000000000000018 crate-A src ckt: 3   src avg:       6 B    total: 18 B\n"
                .to_string(),
            "00000000000000000010 crate-B src ckt: 2   src avg:       5 B    total: 10 B\n"
                .to_string(),
            "00000000000000000001 crate-D src ckt: 1   src avg:       1 B    total: 1 B\n"
                .to_string(),
        ];
        assert_eq!(stats, wanted);
    }

}
