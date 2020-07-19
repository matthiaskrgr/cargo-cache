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
use humansize::{file_size_opts, FileSize};
use walkdir::WalkDir;

use crate::tables::format_table;

#[derive(Debug, Clone)]
struct File {
    path: PathBuf,
    access_date: NaiveDate,
}

/// calculate percentage (what % is X of Y)
fn percentage_of_as_string(fraction: u64, total: u64) -> String {
    let percentage: f32 = (fraction * 100) as f32 / (total) as f32;

    format!("{:.*} %", 2, percentage)
}

/// get the location of a local sccache path
fn sccache_dir() -> Option<PathBuf> {
    if let Some(path) = env::var_os("SCCACHE_DIR").map(PathBuf::from) {
        Some(path)
    } else {
        // if SCCACHE_DIR variable is not present, get the cache dir from "dirs" crate
        let mut cache_dir: Option<PathBuf> = dirs::cache_dir();

        if let Some(cache_dir) = cache_dir.as_mut() {
            cache_dir.push("sccache");
            Some(cache_dir.to_path_buf())
        } else {
            cache_dir
        }
    }
}

pub(crate) fn sccache_stats() {
    let sccache_path: PathBuf = sccache_dir()
        .expect("Failed to get a valid sccache cache dir such as \"~/.cache/sccache\"");
    //@TODO ^ turn this into a proper error message ^ !

    // of all the files inside the sccache cache, gather last access time and path
    let files = WalkDir::new(sccache_path.display().to_string())
        .into_iter()
        .filter_map(|direntry| {
            if let Ok(direntry) = direntry {
                let path = direntry.path().to_path_buf();
                if path.is_file() {
                    if let Ok(metadata) = fs::metadata(&path) {
                        if let Ok(access_time) = metadata.accessed() {
                            let access_time =
                                chrono::DateTime::<Local>::from(access_time).naive_local();
                            let access_date = access_time.date();
                            return Some(File { path, access_date });
                        }
                    }
                }
            };

            None
        });

    // sort files by access date (date, not time!)
    let files_sorted = {
        let mut v: Vec<File> = files.collect();
        v.sort_by_key(|file| file.access_date);
        v
    };

    // get unique access dates, the dates which we have files accessed at
    let unique_access_dates: Vec<File> = {
        let mut unique = files_sorted.clone();
        unique.dedup_by_key(|f| f.access_date);
        unique
    };

    #[allow(clippy::filter_map)]
    let total_size_entire_cache: u64 = files_sorted
        .iter()
        .filter_map(|file| fs::metadata(&file.path).ok())
        .map(|metadata| metadata.len())
        .sum();

    // extract the unique dates from the unique vec
    let table_matrix: Vec<Vec<String>> = unique_access_dates
        .into_iter()
        // dates extracted, now..
        .map(|unique_date| {
            // ..count how often each date is contained inside the files_sorted() array and return that
            // together with the date
            let count = files_sorted
                .iter()
                .filter(|file| file.access_date == unique_date.access_date)
                .count();

            #[allow(clippy::filter_map)]
            let total_size_bytes: u64 = files_sorted
                .iter()
                .filter(|file| file.access_date == unique_date.access_date)
                .filter_map(|file| fs::metadata(&file.path).ok())
                .map(|metadata| metadata.len())
                .sum();

            let size_human_readable = total_size_bytes.file_size(file_size_opts::DECIMAL).unwrap();

            let percentage = percentage_of_as_string(total_size_bytes, total_size_entire_cache);

            vec![
                count.to_string(),
                unique_date.access_date.to_string(),
                size_human_readable,
                percentage,
            ]
        })
        .collect();

    // add column descriptions
    let mut table_vec = Vec::with_capacity(table_matrix.len() + 1);
    table_vec.push(vec![
        "Files".to_string(),
        "Day".to_string(),
        "Total size".to_string(),
        "Percentage".to_string(),
    ]);
    table_vec.extend(table_matrix);

    //let mut tab_columns: Vec<TableLine> = Vec::with_capacity(date_occurrences.len() + 1);
    //tab_columns.push(TableLine::new(2, &"Files".to_string(), &"Day".to_string()));
    //tab_columns.extend(date_occurrences);

    let table = format_table(&table_vec);
    print!("{}", table);
}
