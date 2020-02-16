// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fs;
use std::path::PathBuf;

use crate::cache::caches::{Cache, RegistrySuperCache};
use crate::cache::*;
use crate::library::*;

use humansize::{file_size_opts, FileSize};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum CacheComponent {
    All,
    GitDB,
    GitRepos,
    RegistrySources,
    RegistryCrateCache,
    RegistryIndex,
    Registry,
}

impl std::str::FromStr for CacheComponent {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "all" => Ok(CacheComponent::All),
            "git-db" => Ok(CacheComponent::GitDB),
            "git-repos" => Ok(CacheComponent::GitRepos),
            "registry-sources" => Ok(CacheComponent::RegistrySources),
            "registry-crate-cache" => Ok(CacheComponent::RegistryCrateCache),
            "registry-index" => Ok(CacheComponent::RegistryIndex),
            "registry" => Ok(CacheComponent::Registry),
            other => Err(other.to_string()),
        }
    }
}

pub(crate) fn rm_old_crates(
    amount_to_keep: u64,
    dry_run: bool,
    registry_src_path: &PathBuf,
    size_changed: &mut bool,
) -> Result<(), Error> {
    println!();

    // remove crate sources from cache
    // src can be completely removed since we can always rebuilt it from cache (by extracting packages)
    let mut removed_size = 0;
    // walk registry repos
    for repo in fs::read_dir(&registry_src_path).unwrap() {
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
            let path_end = match pkgpath.iter().last() {
                Some(path_end) => path_end,
                None => return Err(Error::MalformedPackageName(pkgpath.display().to_string())),
            };

            let mut vec = path_end.to_str().unwrap().split('-').collect::<Vec<&str>>();
            let pkgver = match vec.pop() {
                Some(pkgver) => pkgver,
                None => return Err(Error::MalformedPackageName(pkgpath.display().to_string())),
            };
            let pkgname = vec.join("-");

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
                remove_file(pkgpath, dry_run, size_changed, None, Some(dryrun_msg), None);

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
                    remove_file(pkgpath, dry_run, size_changed, None, Some(dryrun_msg), None);
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn remove_dir_via_cmdline(
    directory: Option<&str>,
    dry_run: bool,
    ccd: &CargoCachePaths,
    size_changed: &mut bool,
    checkouts_cache: &mut git_checkouts::GitCheckoutCache,
    bare_repos_cache: &mut git_repos_bare::GitRepoCache,
    registry_index_caches: &mut registry_index::RegistryIndicesCache,
    registry_pkgs_cache: &mut registry_pkg_cache::RegistryPkgCaches,
    registry_sources_caches: &mut registry_sources::RegistrySourceCaches,
) -> Result<(), Error> {
    // @TODO the passing of the cache is really a mess here... :(
    fn rm(
        dir: &PathBuf,
        dry_run: bool,
        size_changed: &mut bool,
        total_size_from_cache: Option<u64>,
    ) -> Result<(), Error> {
        // remove a specified subdirectory from cargo cache
        let msg = Some(format!("removing: '{}'", dir.display()));

        remove_file(dir, dry_run, size_changed, msg, None, total_size_from_cache);
        Ok(())
    }

    let input = if let Some(value) = directory {
        value
    } else {
        return Err(Error::RemoveDirNoArg);
    };

    // sort failed and successfull parses
    #[allow(clippy::type_complexity)]
    let (dirs, errors): (
        Vec<Result<CacheComponent, String>>,
        Vec<Result<CacheComponent, String>>,
    ) = input.split(',').map(str::parse).partition(Result::is_ok);

    // validate input
    let invalid_dirs = errors
        .into_iter()
        .map(|e| e.err().unwrap())
        .collect::<Vec<String>>();
    // check if we have errors, if yes, abort and return a list of the unrecognized directories
    if !invalid_dirs.is_empty() {
        let inv_dirs_joined = invalid_dirs.join(" ");
        let inv_dirs_trimmed = inv_dirs_joined.trim();
        return Err(Error::InvalidDeletableDirs(inv_dirs_trimmed.to_string()));
    }

    // unwrap the Results
    let mut dirs = dirs
        .into_iter()
        .map(|d| d.ok().unwrap())
        .collect::<Vec<CacheComponent>>();

    while dirs.contains(&CacheComponent::All) {
        dirs.extend_from_slice(&[
            CacheComponent::Registry,
            CacheComponent::RegistrySources,
            CacheComponent::RegistryIndex,
            CacheComponent::GitRepos,
            CacheComponent::GitDB,
        ]);
        // find item position and remove it
        // @FIXME use vec::remove_item() here
        let pos = dirs.iter().position(|c| *c == CacheComponent::All).unwrap();
        let _ = dirs.remove(pos);
    }
    // remove duplicates
    dirs.sort();
    dirs.dedup();

    let mut size_removed: u64 = 0;

    if dry_run {
        println!(); // newline
    }

    for component in dirs {
        match component {
            CacheComponent::All => {
                // we can't reach this case since we remove this variant previously
                unreachable!();
            }
            CacheComponent::Registry | CacheComponent::RegistryCrateCache => {
                let size = registry_sources_caches.total_size();
                size_removed += size;
                rm(&ccd.registry_sources, dry_run, size_changed, Some(size))?;

                let size = registry_pkgs_cache.total_size();
                size_removed += size;
                rm(&ccd.registry_pkg_cache, dry_run, size_changed, Some(size))?;
            }
            CacheComponent::RegistrySources => {
                let size = registry_sources_caches.total_size();
                size_removed += size;
                rm(&ccd.registry_sources, dry_run, size_changed, Some(size))?;
            }
            CacheComponent::RegistryIndex => {
                // sum the sizes of the separate indices
                let size_of_all_indices: u64 = registry_index_caches.total_size();

                size_removed += size_of_all_indices;
                // @TODO only remove specified index
                rm(
                    &ccd.registry_index,
                    dry_run,
                    size_changed,
                    Some(size_of_all_indices),
                )?
            }
            CacheComponent::GitRepos => {
                let size = checkouts_cache.total_size();
                size_removed += size;
                rm(&ccd.git_checkouts, dry_run, size_changed, Some(size))?;
            }
            CacheComponent::GitDB => {
                let size = bare_repos_cache.total_size();
                size_removed += size;
                rm(&ccd.git_repos_bare, dry_run, size_changed, Some(size))?;

                let size = checkouts_cache.total_size();
                size_removed += size;
                rm(&ccd.git_checkouts, dry_run, size_changed, Some(size))?;
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

pub(crate) fn remove_file(
    path: &PathBuf,
    dry_run: bool,
    size_changed: &mut bool,
    deletion_msg: Option<String>,
    dry_run_msg: Option<String>,
    total_size_from_cache: Option<u64>,
) {
    if dry_run {
        if let Some(dr_msg) = dry_run_msg {
            println!("{}", dr_msg)
        } else if let Some(size) = total_size_from_cache {
            let size_hr = size.file_size(file_size_opts::DECIMAL).unwrap();
            println!("dry-run: would remove: '{}' ({})", path.display(), size_hr);
        } else {
            println!("dry-run: would remove: '{}'", path.display());
        }
    } else {
        // print deletion message if we have one
        if let Some(msg) = deletion_msg {
            println!("{}", msg);
        }

        if path.is_file() && fs::remove_file(&path).is_err() {
            eprintln!("Warning: failed to remove file \"{}\".", path.display());
        } else {
            *size_changed = true;
        }

        if path.is_dir() && fs::remove_dir_all(&path).is_err() {
            eprintln!(
                "Warning: failed to recursively remove directory \"{}\".",
                path.display()
            );
        } else {
            *size_changed = true;
        }
    }
}
