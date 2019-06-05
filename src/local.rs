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
use walkdir::WalkDir;

use crate::library;
use crate::library::pad_strings;

fn seeing_manifest(path: &PathBuf) -> Option<PathBuf> {
    // check if the "Cargo.toml" manifest can be seen in the current directory
    #[allow(clippy::filter_map)]
    read_dir(&path)
        .unwrap()
        .filter(Result::is_ok)
        .map(|d| d.unwrap().path())
        .find(|f| f.file_name() == Some(OsStr::new("Cargo.toml")))
}

fn get_manifest() -> PathBuf {
    // find the closest manifest (Cargo.toml)
    let mut cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("failed to determine current work directory '{}'", e);
            process::exit(1);
        }
    };

    let manifest;

    loop {
        if let Some(mf) = seeing_manifest(&cwd) {
            manifest = mf;
            break;
        } else {
            let fs_root_reached = !cwd.pop();

            if fs_root_reached {
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
        .manifest_path(&manifest)
        .features(CargoOpt::AllFeatures)
        .exec()
        .unwrap_or_else(|_| panic!("Failed to parse manifest: '{}'", &manifest.display()));

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

    output.push_str(&format!("Target dir: {}\n\n", target_dir.display()));

    output.push_str(&pad_strings(0, 15, "Total size: ", size_hr.as_str()));

    let p = &target_dir;
    let td_debug = p.clone().join("debug");
    let td_rls = p.clone().join("rls");
    let td_release = p.clone().join("release");
    let td_package = p.clone().join("package");
    let td_doc = p.join("doc");

    let size_debug = library::cumulative_dir_size(&td_debug).dir_size;
    if size_debug > 0 {
        output.push_str(&pad_strings(
            1,
            15,
            "debug: ",
            &size_debug.file_size(file_size_opts::DECIMAL).unwrap(),
        ));
    }

    let size_rls = library::cumulative_dir_size(&td_rls).dir_size;
    if size_rls > 0 {
        output.push_str(&pad_strings(
            1,
            15,
            "rls: ",
            &size_rls.file_size(file_size_opts::DECIMAL).unwrap(),
        ));
    }

    let size_release = library::cumulative_dir_size(&td_release).dir_size;
    if size_release > 0 {
        output.push_str(&pad_strings(
            1,
            15,
            "release: ",
            &size_release.file_size(file_size_opts::DECIMAL).unwrap(),
        ));
    }

    let size_package = library::cumulative_dir_size(&td_package).dir_size;
    if size_package > 0 {
        output.push_str(&pad_strings(
            1,
            15,
            "package: ",
            &size_package.file_size(file_size_opts::DECIMAL).unwrap(),
        ));
    }

    let size_doc = library::cumulative_dir_size(&td_doc).dir_size;
    if size_doc > 0 {
        output.push_str(&pad_strings(
            1,
            15,
            "doc: ",
            &size_doc.file_size(file_size_opts::DECIMAL).unwrap(),
        ));
    }

    // For everything else ("other") that is inside the target dir, we need to do some extra work
    // to find out how big it is.
    // Get the immediate subdirs of the target/ dir, skip the known ones (rls, package, debug, release)
    // and look how big the remaining stuff is
    #[allow(clippy::filter_map)] // meh
    let size_other: u64 = std::fs::read_dir(&target_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|x| x.path())
        // skip these, since we already printed them
        .filter(|f| {
            !(f.starts_with(&td_debug)
                || f.starts_with(&td_release)
                || f.starts_with(&td_rls)
                || f.starts_with(&td_package)
                || f.starts_with(&td_doc))
        })
        // for the other directories, crawl them recursively and flatten the walkdir items
        .flat_map(|f| {
            WalkDir::new(f.display().to_string())
                .into_iter()
                .skip(1)
                .map(|d| d.unwrap().into_path())
        })
        .filter(|f| f.exists())
        .map(|f| {
            std::fs::metadata(&f)
                .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &f.display()))
                .len()
        })
        .sum();

    if size_other > 0 {
        output.push_str(&pad_strings(
            1,
            15,
            "other: ",
            &size_other.file_size(file_size_opts::DECIMAL).unwrap(),
        ));
    }

    println!("{}", output);
}
