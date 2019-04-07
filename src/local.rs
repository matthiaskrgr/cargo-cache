// Copyright 2017-2019 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::env;
use std::ffi::OsStr;
use std::fs::read_dir;
use std::path::PathBuf;
use std::process;

use cargo_metadata::{CargoOpt, MetadataCommand};
use clap::ArgMatches;
use humansize::{file_size_opts, FileSize};

use crate::library;
use crate::library::pad_strings;

fn seeing_manifest(path: &PathBuf) -> Option<PathBuf> {
    #[allow(clippy::filter_map)]
    read_dir(&path)
        .unwrap()
        .filter(Result::is_ok)
        .map(|d| d.unwrap().path())
        .find(|f| f.file_name().is_some() && f.file_name().unwrap() == OsStr::new("Cargo.toml"))
}

fn get_manifest() -> PathBuf {
    let mut cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("failed to determine current work directory '{}'", e);
            process::exit(1);
        }
    };

    let mut manifest;

    loop {
        if let Some(mf) = seeing_manifest(&cwd) {
            manifest = mf;
            break;
        } else {
            let root_reached = !cwd.pop();

            if root_reached {
                eprintln!("Did not find manifest!");
                std::process::exit(123);
            }
        }
    }

    manifest
}

pub(crate) fn local_run(_local_config: &ArgMatches<'_>) {
    // find the closest manifest, traverse up if neccesary
    let manifest = get_manifest();

    // get the metadata
    let metadata = MetadataCommand::new()
        .manifest_path(manifest)
        .features(CargoOpt::AllFeatures)
        .exec()
        .unwrap(); // @TODO error handling

    let target_dir = metadata.target_directory;

    //println!("Found target dir: '{}'", target_dir.display());
    let dirinfo = library::cumulative_dir_size(&target_dir);

    let mut output = String::new();

    let size_hr = dirinfo.dir_size.file_size(file_size_opts::DECIMAL).unwrap();

    output.push_str(&format!(
        "Project {:?}\n",
        metadata
            .workspace_root
            .into_os_string()
            .into_string()
            .unwrap()
    ));

    // Do we actually have a target dir?
    if !target_dir.exists() {
        output.push_str("No target dir found!");
        println!("{}", output);
        return;
    }

    output.push_str(&format!("Target dir: {}\n", target_dir.display()));

    output.push_str(&format!("Total Size Size: {}\n", size_hr));

    let p = target_dir;
    let td_debug = p.clone().join("debug");
    let td_rls = p.clone().join("rls");
    let td_release = p.clone().join("release");
    let td_package = p.join("package");

    let size_debug = library::cumulative_dir_size(&td_debug).dir_size;

    if size_debug > 0 {
        output.push_str(&pad_strings(
            0,
            10,
            "debug: ",
            &size_debug.file_size(file_size_opts::DECIMAL).unwrap(),
        ));
    }

    let size_rls = library::cumulative_dir_size(&td_rls).dir_size;
    if size_rls > 0 {
        output.push_str(&format!(
            "rls: {}\n",
            size_rls.file_size(file_size_opts::DECIMAL).unwrap()
        ));
    }

    let size_release = library::cumulative_dir_size(&td_release).dir_size;
    if size_release > 0 {
        output.push_str(&format!(
            "release: {}\n",
            size_release.file_size(file_size_opts::DECIMAL).unwrap()
        ));
    }

    let size_package = library::cumulative_dir_size(&td_package).dir_size;
    if size_package > 0 {
        output.push_str(&format!(
            "package: {}",
            size_package.file_size(file_size_opts::DECIMAL).unwrap()
        ));
    }

    println!("{}", output);
}
