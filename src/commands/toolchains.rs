// Copyright 2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// dir: ~/.rustup/toolchains
// display:   toolchain name, number of files,  size

use std::env;
use std::fs;
use std::path::PathBuf;

use chrono::prelude::*;
use home;
use humansize::{file_size_opts, FileSize};
use walkdir::WalkDir;

use crate::library;
use crate::tables::format_table;

#[derive(Debug, Clone)]
struct File {
    path: PathBuf,
    access_date: NaiveDate,
}

/// calculate percentage (what % is X of Y)
fn percentage_of_as_string(fraction: u64, total: u64) -> String {
    // loss of precision is ok here since we trim down to 2 decimal places
    #[allow(clippy::cast_precision_loss)]
    let percentage: f32 = (fraction * 100) as f32 / (total) as f32;

    format!("{:.*} %", 2, percentage)
}

/// return a list of toolchains (subdirs in the toolchain directory)
fn toolchains() -> Result<std::fs::ReadDir, library::Error> {
    let toolchain_root = {
        let mut p = home::rustup_home().unwrap();
        p.push("toolchains");
        p
    };

    Ok(std::fs::read_dir(&toolchain_root).unwrap())
}

#[derive(Clone, Debug)]
struct Toolchain {
    name: String,
    path: PathBuf,
    number_files: usize,
    size: u64,
}

impl<'a> Toolchain {
    fn new(path: PathBuf) -> Self {
        let name = path.file_name().unwrap().to_owned().into_string().unwrap();
        let number_files = WalkDir::new(&path).into_iter().count();
        let size: u64 = WalkDir::new(&path)
            .into_iter()
            .map(|f| {
                let x = f.unwrap();
                let z = x.path().to_owned();
                z
            })
            .filter(|f| f.is_file())
            .map(|f| std::fs::metadata(&f).unwrap().len())
            .sum();

        Toolchain {
            name,
            path: path.clone(),
            number_files,
            size,
        }
    }
}

pub(crate) fn toolchain_stats() -> Result<(), library::Error> {
    let toolchains = toolchains()
        .unwrap()
        .map(|dir| dir.unwrap().path())
        .map(|p| Toolchain::new(p));

    toolchains.for_each(|x| println!("{:?}", x));

    panic!();
    /*
    // of all the files inside the sccache cache, gather last access time and path
    let files = vec![PathBuf::new()].into_iter();

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

    let mut total_size: u64 = 0;

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

            // calculate total file size sum for the summary
            total_size += total_size_bytes;

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
    let mut table_vec =
        Vec::with_capacity(table_matrix.len() + 2 /* header column + summary */);
    table_vec.push(vec![
        "Files".to_string(),
        "Day".to_string(),
        "Size".to_string(),
        "Percentage".to_string(),
    ]);
    table_vec.extend(table_matrix);

    // add a final summary
    // newline
    table_vec.push(vec![
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ]);
    // Total:
    table_vec.push(vec![
        String::from("Total"),
        String::new(),
        String::new(),
        String::new(),
    ]);

    let number_of_files = files_sorted.len();
    // summary
    table_vec.push(vec![
        number_of_files.to_string(),
        String::new(),
        total_size.file_size(file_size_opts::DECIMAL).unwrap(),
        "100 %".into(),
    ]);

    // generate the table and print it
    let table = format_table(&table_vec, 1); // need so strip whitespaces added by the padding
    let table_trimmed = table.trim();
    println!("{}", table_trimmed);
    Ok(())
    */
    Ok(())
}
