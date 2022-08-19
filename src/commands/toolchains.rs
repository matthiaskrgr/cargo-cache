// Copyright 2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::path::PathBuf;

use chrono::prelude::*;
use humansize::{file_size_opts, FileSize};
use walkdir::WalkDir;

use crate::library;
use crate::sccache::percentage_of_as_string;
use crate::tables::format_table;

#[derive(Debug, Clone)]
struct File {
    #[allow(unused)]
    path: PathBuf,
    #[allow(unused)]
    access_date: NaiveDate,
}

/// return a list of toolchains (subdirs in the toolchain directory)
fn toolchains() -> Result<std::fs::ReadDir, library::Error> {
    let toolchain_root = {
        // intentionally map the Err to our own type
        #[allow(clippy::map_err_ignore)]
        let mut p = home::rustup_home().map_err(|_| library::Error::NoRustupHome)?;
        p.push("toolchains");
        p
    };

    match std::fs::read_dir(&toolchain_root) {
        Ok(readdir) => Ok(readdir),
        // we might be on a system that has rust installed purley via package manager and not via rustup! (#121)
        _ => Err(library::Error::NoRustupHome),
    }
}

#[derive(Clone, Debug)]
struct Toolchain {
    name: String,
    #[allow(unused)]
    path: PathBuf,
    number_files: usize,
    size: u64,
}

impl Toolchain {
    fn new(path: PathBuf) -> Self {
        let name = path.file_name().unwrap().to_owned().into_string().unwrap();
        let number_files = WalkDir::new(&path).into_iter().count();
        #[allow(clippy::manual_filter_map)]
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
            path,
            number_files,
            size,
        }
    }
}

pub(crate) fn toolchain_stats() {
    // get a list of toolchains, sorted by size
    let toolchains = {
        let toolchain_readdir = match toolchains() {
            Ok(readdir) => readdir,
            Err(library::Error::NoRustupHome) => {
                eprintln!("Could not find any toolchains installed via rustup!");
                std::process::exit(0);
            }
            Err(e) => unreachable!("encountered unexpected error: '{:?}'", e),
        };

        let mut tcs = toolchain_readdir
            .map(|dir| dir.unwrap().path())
            .map(Toolchain::new)
            .collect::<Vec<_>>();
        tcs.sort_by_key(|tc| tc.size);
        tcs.reverse();
        tcs
    };

    // get the size
    let total_size: u64 = toolchains.iter().map(|toolchain| toolchain.size).sum();

    // extract the unique dates from the unique vec
    let table_matrix: Vec<Vec<String>> = toolchains
        .iter()
        .map(|toolchain| {
            vec![
                toolchain.name.clone(),
                toolchain.number_files.to_string(),
                toolchain.size.file_size(file_size_opts::DECIMAL).unwrap(),
                percentage_of_as_string(toolchain.size, total_size),
            ]
        })
        .collect();

    // add column descriptions
    let mut table_vec = Vec::with_capacity(
        table_matrix.len() + 3, /* header column + summary stats */
    );
    table_vec.push(vec![
        "Toolchain Name".to_string(),
        "Files".to_string(),
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
    let number_of_files: usize = toolchains
        .iter()
        .map(|toolchain| toolchain.number_files)
        .sum();

    // summary
    table_vec.push(vec![
        String::from("Total"),
        number_of_files.to_string(),
        total_size.file_size(file_size_opts::DECIMAL).unwrap(),
        "100 %".into(),
    ]);

    // generate the table and print it
    let table = format_table(&table_vec, 1); // need so strip whitespaces added by the padding
    let table_trimmed = table.trim();
    println!("{}", table_trimmed);
}
