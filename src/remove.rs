// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fs;
use std::path::{Path, PathBuf};

use crate::cache::caches::{Cache, RegistrySuperCache};
use crate::cache::*;
use crate::library::*;

use humansize::{file_size_opts, FileSize};

/// dry run message setting
pub(crate) enum DryRunMessage<'a> {
    Custom(&'a str), // use the message that is passed
    Default,         // use the default message
    #[allow(dead_code)]
    None, // no message
}

fn parse_version(path: &Path) -> Result<(String, String), Error> {
    #[allow(clippy::single_match_else)]
    let filename = match path.file_stem() {
        Some(name) => name.to_str().unwrap().to_string(),
        None => {
            return Err(Error::MalformedPackageName(path.display().to_string()));
        }
    };

    let mut name = Vec::new();
    let mut version = Vec::new();
    let mut found_version = false;

    filename.split('-').for_each(|seg| {
        let first_char = seg.chars().next();
        if let Some(char) = first_char {
            if char.is_numeric() {
                // the first char of the segment is a number
                // conclude that this segment is the version string
                version.push(seg);
                // if we have found the first version, everything afterwards will be version info
                found_version = true;
            } else if found_version {
                version.push(seg);
            } else {
                // if char is not numeric
                name.push(seg);
            }
        }
    });

    let name = name.join("-");
    let version = version.join("-");

    Ok((name, version))
}

pub(crate) fn rm_old_crates(
    amount_to_keep: u64,
    dry_run: bool,
    registry_src_path: &Path,
    size_changed: &mut bool,
) -> Result<(), Error> {
    println!();

    // remove crate sources from cache
    // src can be completely removed since we can always rebuilt it from cache (by extracting packages)
    let mut removed_size = 0;
    // walk registry repos
    for repo in fs::read_dir(registry_src_path).unwrap() {
        let mut crate_list = fs::read_dir(&repo.unwrap().path())
            .unwrap()
            .map(|cratepath| cratepath.unwrap().path())
            .collect::<Vec<PathBuf>>();
        crate_list.sort();
        crate_list.reverse();

        let mut versions_of_this_package = 0;
        let mut last_pkgname = String::new();

        // iterate over all crates and extract name and version
        for pkgpath in &crate_list {
            let (pkgname, pkgver) = parse_version(pkgpath)?;

            if amount_to_keep == 0 {
                removed_size += fs::metadata(pkgpath)
                    .unwrap_or_else(|_| {
                        panic!("Failed to get metadata of file '{}'", &pkgpath.display())
                    })
                    .len();

                let dryrun_msg = format!(
                    "dry run: not actually deleting {} {} at {}",
                    pkgname,
                    pkgver,
                    pkgpath.display()
                );
                remove_file(
                    pkgpath,
                    dry_run,
                    size_changed,
                    None,
                    &DryRunMessage::Custom(&dryrun_msg),
                    None,
                );

                continue;
            }
            // println!("pkgname: {:?}, pkgver: {:?}", pkgname, pkgver);

            if last_pkgname == pkgname {
                versions_of_this_package += 1;
                if versions_of_this_package == amount_to_keep {
                    // we have seen this package too many times, queue for deletion
                    removed_size += fs::metadata(pkgpath)
                        .unwrap_or_else(|_| {
                            panic!("Failed to get metadata of file '{}'", &pkgpath.display())
                        })
                        .len();

                    let dryrun_msg = format!(
                        "dry run: not actually deleting {} {} at {}",
                        pkgname,
                        pkgver,
                        pkgpath.display()
                    );
                    remove_file(
                        pkgpath,
                        dry_run,
                        size_changed,
                        None,
                        &DryRunMessage::Custom(&dryrun_msg),
                        None,
                    );
                }
            } else {
                // last_pkgname != pkgname, we got to a new package, reset counter
                versions_of_this_package = 0;
                last_pkgname = pkgname;
            } // if last_pkgname == pkgname
        } // for pkgpath in &crate_list
    }
    println!(
        "Removed {} of compressed crate sources.",
        removed_size.file_size(file_size_opts::DECIMAL).unwrap()
    );
    Ok(())
}

/// take a list of cache items via cmdline and remove them, invalidate caches too
#[allow(clippy::too_many_arguments)]
pub(crate) fn remove_dir_via_cmdline(
    directory: Option<&str>,
    dry_run: bool,
    ccd: &CargoCachePaths,
    size_changed: &mut bool,
    checkouts_cache: &mut git_checkouts::GitCheckoutCache,
    bare_repos_cache: &mut git_bare_repos::GitRepoCache,
    registry_index_caches: &mut registry_index::RegistryIndicesCache,
    registry_pkgs_cache: &mut registry_pkg_cache::RegistryPkgCaches,
    registry_sources_caches: &mut registry_sources::RegistrySourceCaches,
) -> Result<(), Error> {
    // @TODO the passing of the cache is really a mess here... :(

    let dirs_to_remove = components_from_groups(directory)?;

    let mut size_removed: u64 = 0;

    if dry_run {
        println!(); // newline
    }

    for component in dirs_to_remove {
        match component {
            Component::RegistryCrateCache => {
                let size = registry_pkgs_cache.total_size();
                size_removed += size;
                remove_with_default_message(
                    &ccd.registry_pkg_cache,
                    dry_run,
                    size_changed,
                    Some(size),
                );
                if !dry_run {
                    registry_pkgs_cache.invalidate();
                }
            }

            Component::RegistrySources => {
                let size = registry_sources_caches.total_size();
                size_removed += size;
                remove_with_default_message(
                    &ccd.registry_sources,
                    dry_run,
                    size_changed,
                    Some(size),
                );
                if !dry_run {
                    registry_sources_caches.invalidate();
                }
            }
            Component::RegistryIndex => {
                // sum the sizes of the separate indices
                let size_of_all_indices: u64 = registry_index_caches.total_size();

                size_removed += size_of_all_indices;
                // @TODO only remove specified index
                remove_with_default_message(
                    &ccd.registry_index,
                    dry_run,
                    size_changed,
                    Some(size_of_all_indices),
                );
                if !dry_run {
                    registry_index_caches.invalidate();
                }
            }
            Component::GitRepos => {
                let size = checkouts_cache.total_size();
                size_removed += size;
                remove_with_default_message(&ccd.git_checkouts, dry_run, size_changed, Some(size));
                if !dry_run {
                    checkouts_cache.invalidate();
                }
            }
            Component::GitDB => {
                let size = bare_repos_cache.total_size();
                size_removed += size;
                remove_with_default_message(&ccd.git_repos_bare, dry_run, size_changed, Some(size));
                if !dry_run {
                    bare_repos_cache.invalidate();
                }
            }
        }
    }

    if dry_run {
        println!(
            "dry-run: would remove in total: {}",
            size_removed.file_size(file_size_opts::DECIMAL).unwrap()
        );
    }

    Ok(())
}

/// remove a file with a default "removing: {file}" message
pub(crate) fn remove_with_default_message(
    dir: &Path,
    dry_run: bool,
    size_changed: &mut bool,
    total_size_from_cache: Option<u64>,
) {
    // remove a specified subdirectory from cargo cache
    let msg = Some(format!("removing: '{}'", dir.display()));

    remove_file(
        dir,
        dry_run,
        size_changed,
        msg,
        &DryRunMessage::Default,
        total_size_from_cache,
    );
}

/// remove a file with a custom message
pub(crate) fn remove_file(
    // path of the file to be deleted
    path: &Path,
    // is this only a dry run? if yes, remove nothing
    dry_run: bool,
    // did we actually remove anything?
    size_changed: &mut bool,
    // print a custom deletion message
    deletion_msg: Option<String>,
    // print a custom dryrun message
    dry_run_msg: &DryRunMessage<'_>,
    // size of the file according to cache
    total_size_from_cache: Option<u64>,
) {
    if dry_run {
        match dry_run_msg {
            DryRunMessage::Custom(msg) => {
                println!("{}", msg);
            }
            DryRunMessage::Default => {
                #[allow(clippy::single_match_else)]
                match total_size_from_cache {
                    Some(size) => {
                        // print the size that is saved from the cache before removing
                        let size_hr = size.file_size(file_size_opts::DECIMAL).unwrap();
                        println!("dry-run: would remove: '{}' ({})", path.display(), size_hr);
                    }
                    None => {
                        // default case: print this message
                        println!("dry-run: would remove: '{}'", path.display());
                    }
                }
            }
            DryRunMessage::None => {}
        }
    } else {
        // no dry run
        // print deletion message if we have one
        if let Some(msg) = deletion_msg {
            println!("{}", msg);
        }

        if path.is_file() && fs::remove_file(path).is_err() {
            eprintln!("Warning: failed to remove file \"{}\".", path.display());
        } else {
            *size_changed = true;
        }

        if path.is_dir() {
            if let Err(error) = remove_dir_all::remove_dir_all(path) {
                eprintln!(
                    "Warning: failed to recursively remove directory \"{}\".",
                    path.display()
                );
                eprintln!("error: {:?}", error);
            } else {
                *size_changed = true;
            }
        }
    }
}

#[cfg(test)]
mod libtests {
    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn test_parse_version() {
        let (name, version): (String, String) =
            parse_version(&PathBuf::from("heim-runtime-0.1.0-beta.1.crate")).unwrap();

        assert_eq!(name, "heim-runtime");
        assert_eq!(version, "0.1.0-beta.1");

        let (name2, version2): (String, String) =
            parse_version(&PathBuf::from("cargo-cache-0.4.3.crate")).unwrap();

        assert_eq!(name2, "cargo-cache");
        assert_eq!(version2, "0.4.3");
    }
}
