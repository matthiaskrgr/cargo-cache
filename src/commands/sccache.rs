// Copyright 2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// find ~/.cache/sccache -type f -printf "\n%AD %AT %p"  | cut -d' ' -f1 | sort -n | uniq -c

use std::env;
use std::fs;
use std::path::PathBuf;

use chrono::prelude::*;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
struct File {
    path: PathBuf,
    access_date: NaiveDate,
}

fn sccache_dir() -> Option<PathBuf> {
    match env::var_os("SCCACHE_DIR").map(PathBuf::from) {
        Some(path) => Some(path),
        // if SCCACHE_DIR variable is not present,
        None => {
            // get the cache dir from "dirs" crate
            let mut cache_dir: Option<PathBuf> = dirs::cache_dir();

            if cache_dir.is_some() {
                let mut cache = cache_dir.unwrap();
                cache.push("sccache");
                return Some(cache);
            } else {
                return None;
            }
        }
    }
}

pub(crate) fn sccache_stats() {
    let sccache_path: PathBuf = sccache_dir()
        .expect("Failed to get a valid sccache cache dir such as \"~/.cache/sccache\"");

    // we need to get all the files in the cache
    // get path, creation time and access time
    let files = WalkDir::new(sccache_path.display().to_string())
        .into_iter()
        .filter_map(|f| {
            if let Ok(direntry) = f {
                let path = direntry.path().to_path_buf();
                if path.is_file() {
                    if let Ok(metadata) = fs::metadata(&path) {
                        if let Ok(access_time) = metadata.accessed() {
                            // let creation_time =
                            //   chrono::DateTime::<Local>::from(create_time).naive_local();
                            let access_time =
                                chrono::DateTime::<Local>::from(access_time).naive_local();
                            let access_date = access_time.date();
                            return Some(File {
                                path,
                                // creation_time,
                                access_date,
                            });
                        }
                    }
                }
            };

            None
        });

    let files_sorted = {
        let mut v: Vec<File> = files.collect();
        v.sort_by_key(|file| file.access_date);
        v
    };

    let mut unique = files_sorted.clone();
    unique.dedup_by_key(|f| f.access_date);

    let unique: Vec<NaiveDate> = unique.into_iter().map(|f| f.access_date).collect();

    let date_occurrences: Vec<(usize, &NaiveDate)> = unique
        .iter()
        .map(|unique_date| {
            let count = files_sorted
                .iter()
                .filter(|f| f.access_date == *unique_date)
                .count();

            (count, unique_date)
        })
        .collect();

    date_occurrences
        .iter()
        .map(|x| x)
        // .filter(|x| x.access_time != x.creation_time)
        .for_each(|x| {
            println!("{:?}", x);
        });
}
