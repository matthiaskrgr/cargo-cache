// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/// This file implements the "local" subcommand:
/// `cargo cache local`
/// `cargo cache l`
/// The goal of this subcommand is to provide a simple overview of the
/// target directory sizes which are "local" to the project
/// We print the total size of each subdirectory that we know to be rust-related (so debug/release/package etc)
/// and sum up the rest under "other:"; the output can look like this:
/// ````
/// Project "/home/matthias/vcs/github/cargo-cache"
/// Target dir: /home/matthias/vcs/github/cargo-cache/target
///
/// Total Size:         3.06 GB
/// debug:              2.48 GB
/// release:          224.26 MB
/// other:            360.57 MB
/// ````
use std::env;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs::read_dir;
use std::path::{Path, PathBuf};

use cargo_metadata::MetadataCommand;
use humansize::{file_size_opts, FileSize};
use walkdir::WalkDir;

use crate::library;
use crate::library::Error;
use crate::tables::*;

/// Checks if a cargo manifest named "Cargo.toml" is found in the current directory.
/// If yes, return a path to it, if not, return None
fn seeing_manifest(path: &Path) -> Option<PathBuf> {
    #[allow(clippy::manual_filter_map)]
    read_dir(path)
        .unwrap()
        .filter(Result::is_ok)
        .map(|d| d.unwrap().path())
        .find(|f| f.file_name() == Some(OsStr::new("Cargo.toml")))
}

/// start at the cwd, walk downwards and check if we encounter a Cargo.toml somewhere
pub(crate) fn get_manifest() -> Result<PathBuf, Error> {
    // get the cwd
    let mut cwd: PathBuf = if let Ok(cwd) = env::current_dir() {
        cwd
    } else {
        return Err(Error::NoCWD);
    };
    // save the original path since we call .pop() later
    let orig_cwd = cwd.clone();

    // walk downwards and try to find a "Cargo.toml"
    loop {
        if let Some(manifest_path) = seeing_manifest(&cwd) {
            // if the manifest is seen, return it
            return Ok(manifest_path);
        }
        // otherwise continue walking down and check again
        let fs_root_reached = !cwd.pop();

        if fs_root_reached {
            // we have reached the file system root without finding anything
            return Err(Error::NoCargoManifest(orig_cwd));
        }
    }
}

/// gather the sizes of subdirs of the `target` directory and prints a formatted table
/// of the data to stdout
pub(crate) fn local_subcmd() -> Result<(), Error> {
    // padding of the final formatting of the table
    const MIN_PADDING: usize = 6;

    // find the closest manifest, traverse up if necessary
    let manifest = get_manifest()?;

    // get the cargo metadata for the manifest
    let metadata = MetadataCommand::new()
        .manifest_path(&manifest)
        .no_deps()
        .exec()
        .unwrap_or_else(|error| {
            panic!(
                "Failed to parse manifest: '{}'\nError: '{:?}'",
                &manifest.display(),
                error
            )
        });

    // get the project target dir from the metadata
    let target_dir = PathBuf::from(metadata.target_directory);

    // the target dir might not exist!
    if !target_dir.is_dir() {
        return Err(Error::LocalNoTargetDir(target_dir));
    }

    // println!("Found target dir: '{}'", target_dir.display());

    // get the size
    let dirinfo = library::cumulative_dir_size(&target_dir);
    // and the human readable size
    let size_hr = dirinfo.dir_size.file_size(file_size_opts::DECIMAL).unwrap();

    let mut stdout = String::new();

    let mut lines = Vec::new();

    writeln!(stdout, "Project {:?}", metadata.workspace_root.to_string()).unwrap();

    // If there is no target dir, we can quit
    if !target_dir.exists() {
        stdout.push_str("No target dir found!");
        eprintln!("{}", stdout);
    }

    writeln!(stdout, "Target dir: {}\n", target_dir.display()).unwrap();
    lines.push(TableLine::new(0, &"Total Size: ", &size_hr));

    // we are going to check these directories:
    let p = &target_dir; // path
    let target_dir_debug = p.join("debug");
    let target_dir_rls = p.join("rls");
    let target_dir_release = p.join("release");
    let target_dir_package = p.join("package");
    let target_dir_doc = p.join("doc");

    // gather the sizes of all these directories, `TableLine` will be used for formatting
    let size_debug = library::cumulative_dir_size(&target_dir_debug).dir_size;
    if size_debug > 0 {
        lines.push(TableLine::new(
            0,
            &"debug: ".to_string(),
            &size_debug.file_size(file_size_opts::DECIMAL).unwrap(),
        ));
    }

    let size_rls = library::cumulative_dir_size(&target_dir_rls).dir_size;
    if size_rls > 0 {
        lines.push(TableLine::new(
            0,
            &"rls: ".to_string(),
            &size_rls.file_size(file_size_opts::DECIMAL).unwrap(),
        ));
    }

    let size_release = library::cumulative_dir_size(&target_dir_release).dir_size;
    if size_release > 0 {
        lines.push(TableLine::new(
            0,
            &"release: ".to_string(),
            &size_release.file_size(file_size_opts::DECIMAL).unwrap(),
        ));
    }

    let size_package = library::cumulative_dir_size(&target_dir_package).dir_size;
    if size_package > 0 {
        lines.push(TableLine::new(
            0,
            &"package: ".to_string(),
            &size_package.file_size(file_size_opts::DECIMAL).unwrap(),
        ));
    }

    let size_doc = library::cumulative_dir_size(&target_dir_doc).dir_size;
    if size_doc > 0 {
        lines.push(TableLine::new(
            0,
            &"doc: ".to_string(),
            &size_doc.file_size(file_size_opts::DECIMAL).unwrap(),
        ));
    }

    // For everything else ("other") that is inside the target dir, we need to do some extra work
    // to find out how big it is.
    // Get the immediate subdirs of the target/ dir, skip the known ones (rls, package, debug, release)
    // and look how big the remaining stuff is
    #[allow(clippy::manual_filter_map)] // meh
    let size_other: u64 = read_dir(&target_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|x| x.path())
        // skip these, since we already printed them
        .filter(|f| {
            !(f.starts_with(&target_dir_debug)
                || f.starts_with(&target_dir_release)
                || f.starts_with(&target_dir_rls)
                || f.starts_with(&target_dir_package)
                || f.starts_with(&target_dir_doc))
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
        lines.push(TableLine::new(
            0,
            &"other: ".to_string(),
            &size_other.file_size(file_size_opts::DECIMAL).unwrap(),
        ));
    }

    // add the formatted table to the output
    stdout.push_str(&two_row_table(MIN_PADDING, lines, true));
    // and finally print it
    println!("{}", stdout);
    Ok(())
}
