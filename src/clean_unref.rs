// Copyright 2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// remove all crates from a cache that are not referenced by a Cargo lockfile

//https://github.com/rust-lang/rust-clippy/issues/7202
#![allow(clippy::needless_collect)]

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::cache::caches::*;
use crate::cache::*;
use crate::library::*;
use crate::library::{CargoCachePaths, Error};
use crate::remove::*;
use cargo_metadata::{CargoOpt, MetadataCommand};

// the source of a crate inside the cargo cache can be represented in form of
// an extracted .crate or a checked out git repository
// the path is the absolute path to the source inside the ${CARGO_HOME}
#[derive(Debug, Clone, PartialEq, Eq)]
enum SourceKind {
    Crate(PathBuf),
    Git(PathBuf),
}

// get the path contained in a SourceKind
impl SourceKind {
    fn inner(self) -> PathBuf {
        match self {
            SourceKind::Crate(p) | SourceKind::Git(p) => p,
        }
    }
}

fn find_crate_name_git(toml_path: &Path, cargo_home: &Path) -> Option<SourceKind> {
    // ~/.cargo/git/checkouts/home-fb9469891e5cfbe6/3a6eccd/cargo.toml  => ~/.cargo/git/checkouts/home-fb9469891e5cfbe6/3a6eccd/

    // get the segments of the path
    let v: Vec<&OsStr> = toml_path.iter().collect();

    // if we could not find a position, return None
    let checkouts_pos = v.iter().position(|i| i == &"checkouts")?;

    // assuming git:
    // git checkouts repo-name ref
    let path_segments = &v[(checkouts_pos - 1)..(checkouts_pos + 3)];

    let mut path = cargo_home.to_path_buf();
    path_segments.iter().for_each(|p| path.push(p));

    Some(SourceKind::Git(path))
}

fn find_crate_name_crate(toml_path: &Path, cargo_home: &Path) -> Option<SourceKind> {
    //  ~/.cargo/registry/src/github.com-1ecc6299db9ec823/winapi-0.3.8/Cargo.toml => ~/.cargo/registry/src/github.com-1ecc6299db9ec823/winapi-0.3.8/

    let v: Vec<&OsStr> = toml_path.iter().collect();

    // if we could not find a position, return None
    let registry_pos = v.iter().position(|i| i == &"registry")?;

    let path_segments = &v[(registry_pos)..(registry_pos + 4)];
    let mut path = cargo_home.to_path_buf();
    path_segments.iter().for_each(|p| path.push(p));

    Some(SourceKind::Crate(path))
}

/// look at a crate manifest and remove all items from the cargo cache that are not referenced, also run --autoclean and invalidate caches
#[allow(clippy::too_many_arguments)]
pub(crate) fn clean_unref(
    cargo_cache_paths: &CargoCachePaths,
    manifest_path: Option<&str>,
    bin_cache: &mut bin::BinaryCache,
    checkouts_cache: &mut git_checkouts::GitCheckoutCache,
    bare_repos_cache: &mut git_bare_repos::GitRepoCache,
    registry_pkg_caches: &mut registry_pkg_cache::RegistryPkgCaches,
    registry_index_caches: &mut registry_index::RegistryIndicesCache,
    registry_sources_caches: &mut registry_sources::RegistrySourceCaches,
    dry_run: bool,
    size_changed: &mut bool,
) -> Result<(), Error> {
    // total cache size before removing, for the summary
    let original_total_cache_size = bin_cache.total_size()
        + checkouts_cache.total_size()
        + bare_repos_cache.total_size()
        + registry_pkg_caches.total_size()
        + registry_index_caches.total_size()
        + registry_sources_caches.total_size();

    // first get a list of all dependencies of the project
    let cargo_home = &cargo_cache_paths.cargo_home;

    // if "--manifest-path" is passed to the subcommand, take this
    // if it is not passed, try to find a close manifest somewhere
    let manifest = match manifest_path {
        Some(path_str) => PathBuf::from(path_str),
        None => crate::local::get_manifest()?,
    };

    let metadata = MetadataCommand::new()
        .manifest_path(&manifest)
        .features(CargoOpt::AllFeatures)
        .exec()
        .map_err(|e| Error::UnparsableManifest(manifest, e))?;

    let dependencies = metadata.packages;

    // get the path inside the CARGO_HOME of the source of the dependency
    #[allow(clippy::manual_filter_map)]
    let required_packages = dependencies
        .iter()
        .map(|pkg| PathBuf::from(&pkg.manifest_path))
        // we only care about tomls that are not local, i.e. tomls that are inside the $CARGO_HOME
        .filter(|toml_path| toml_path.starts_with(cargo_home))
        // map the manifest paths to paths to the roots of the crates inside the cargo_home
        .map(|toml_path| {
            if toml_path.starts_with(&cargo_cache_paths.git_checkouts) {
                find_crate_name_git(&toml_path, cargo_home).unwrap_or_else(|| {
                    panic!("Failed to find 'checkouts' in {} ", toml_path.display())
                })
            } else if toml_path.starts_with(&cargo_cache_paths.registry_sources) {
                find_crate_name_crate(&toml_path, cargo_home).unwrap_or_else(|| {
                    panic!("Failed to find 'registry' in {} ", toml_path.display())
                })
            } else {
                // if we find a source path that is neither a git nor a crate dep, this probably indicates a bug
                panic!("Failed to parse toml path: '{}'", toml_path.display());
            }
        })
        // we need to map the git repo checkouts to bare git repos
        // and the source-checkouts to pkg cache archives!
        .map(|sourcekind| match sourcekind {
            SourceKind::Crate(registry_src_path) => {
                // ~/.cargo/registry/src/github.com-1ecc6299db9ec823/semver-0.9.0
                // =>
                // ~/.cargo/registry/cache/github.com-1ecc6299db9ec823/semver-0.9.0.crate
                let path = registry_src_path.iter().collect::<Vec<&OsStr>>();
                let package_name = path[path.len() - 1];
                let registry = path[path.len() - 2];
                let mut registry_cache_path = cargo_cache_paths.registry_pkg_cache.clone();
                // we need to push the registry index as well
                registry_cache_path.push(registry);
                // this can probably be
                // can't use .set_extension() here because "cratename-0.1.3" will detect the ".3" as extension
                // and change it
                registry_cache_path.push(format!(
                    "{}{}",
                    package_name.to_os_string().into_string().unwrap(),
                    ".crate"
                ));
                SourceKind::Crate(registry_cache_path)
            }
            SourceKind::Git(gitpath) => {
                // ~/.cargo/git/checkouts/cargo-e7ff1db891893a9e/258c896
                // =>
                // ~/.cargo/git/db/cargo-e7ff1db891893a9e
                let mut repo_name = gitpath;
                let _ = repo_name.pop(); // remove /258c896
                let repo_name = repo_name.iter().last().unwrap(); // cargo-e7ff1db891893a9e

                let mut db_name = cargo_cache_paths.git_repos_bare.clone();
                db_name.push(repo_name);
                // ~/.cargo/git/db/cargo-e7ff1db891893a9e
                SourceKind::Git(db_name)
            }
        });

    // now we have a list of all cargo-home-entries a crate needs to build
    // we can walk the cargo-cache and remove everything that is not referenced;
    // remove: git checkouts, registry sources
    // keep, if referenced: registry pkg cache, bare git repos

    // debug
    // println!("required packages:");
    // required_packages.inspect(|toml| println!("{:?}", toml));

    // remove the git checkout cache since it is not needed
    remove_file(
        &cargo_cache_paths.git_checkouts,
        dry_run,
        size_changed,
        None,
        &DryRunMessage::Default,
        Some(checkouts_cache.total_size()),
    );
    // invalidate cache
    checkouts_cache.invalidate();

    // remove the registry_sources_cache as well
    remove_file(
        &cargo_cache_paths.registry_sources,
        dry_run,
        size_changed,
        None,
        &DryRunMessage::Default,
        Some(registry_sources_caches.total_size()),
    );
    // invalidate cache
    registry_sources_caches.invalidate();

    let (required_crates, required_git_repos): (Vec<SourceKind>, Vec<SourceKind>) =
        required_packages.partition(|dep| match dep {
            SourceKind::Crate(_) => true,
            SourceKind::Git(_) => false,
        });

    // extract the paths from the SouceKinds
    let required_crates: Vec<_> = required_crates.into_iter().map(SourceKind::inner).collect();

    let required_git_repos: Vec<_> = required_git_repos
        .into_iter()
        .map(SourceKind::inner)
        .collect();
    // for the bare_repos_cache and registry_package_cache,
    // remove all items but the ones that are referenced

    let bare_repos = bare_repos_cache.items();

    // get all .crates found in the cache (we need to check all subcaches)
    // @TODO add method to get all .crates of all caches via single method?
    let mut crates = Vec::new();

    for cache in registry_pkg_caches.caches() {
        crates.extend(cache.files());
    }

    // filter and remove git repos
    bare_repos
        .iter()
        .filter(|repo_in_cache|
            // in the iterator, only keep crates that are not contained in
            // our dependency list and remove them

            !required_git_repos.contains(repo_in_cache))
        .for_each(|repo| {
            /* remove the repo */

            remove_file(
                repo,
                dry_run,
                size_changed,
                None,
                &DryRunMessage::Default,
                Some(size_of_path(repo)),
            );
        });

    // filter and remove crate archives
    crates
        .iter()
        .filter(|crate_in_cache|
            // in the iterator, only keep crates that are not contained in
            // our dependency list and remove them

            !required_crates.contains(crate_in_cache))
        .for_each(|krate| {
            /* remove the crate */
            remove_file(
                krate,
                dry_run,
                size_changed,
                None,
                &DryRunMessage::Default,
                Some(size_of_path(krate)),
            );
        });

    // don't forget to invalidate caches..!
    bare_repos_cache.invalidate();
    registry_pkg_caches.invalidate();

    print_size_changed_summary(
        original_total_cache_size,
        cargo_cache_paths,
        bin_cache,
        checkouts_cache,
        bare_repos_cache,
        registry_pkg_caches,
        registry_index_caches,
        registry_sources_caches,
    );
    Ok(())
}

#[cfg(test)]
mod clitests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn sourcekind_inner() {
        let sk_crate = SourceKind::Crate(PathBuf::from("abc"));
        assert_eq!(sk_crate.inner(), PathBuf::from("abc"));

        let sk_git = SourceKind::Git(PathBuf::from("def"));
        assert_eq!(sk_git.inner(), PathBuf::from("def"));
    }

    #[test]
    fn crate_name_git_some() {
        let toml_path =
            PathBuf::from(".cargo/git/checkouts/home-fb9469891e5cfbe6/3a6eccd/Cargo.toml");
        let cargo_home = PathBuf::from(".cargo/");

        let name = find_crate_name_git(&toml_path, &cargo_home);

        assert_eq!(
            name,
            Some(SourceKind::Git(PathBuf::from(
                ".cargo/git/checkouts/home-fb9469891e5cfbe6/3a6eccd/",
            ))),
        );
    }

    #[test]
    fn crate_name_git_none() {
        // pare failure should return None
        let toml_path =
            PathBuf::from(".cargo/git/failuretoparse/home-fb9469891e5cfbe6/3a6eccd/Cargo.toml");
        let cargo_home = PathBuf::from(".cargo/");

        let name = find_crate_name_git(&toml_path, &cargo_home);

        assert_eq!(name, None);
    }

    #[test]
    fn crate_name_crate_some() {
        let toml_path = PathBuf::from(
            ".cargo/registry/src/github.com-1ecc6299db9ec823/winapi-0.3.8/Cargo.toml",
        );
        let cargo_home = PathBuf::from(".cargo/");

        let name = find_crate_name_crate(&toml_path, &cargo_home);

        assert_eq!(
            name,
            Some(SourceKind::Crate(PathBuf::from(
                ".cargo/registry/src/github.com-1ecc6299db9ec823/winapi-0.3.8/",
            ))),
        );
    }

    #[test]
    fn crate_name_crate_none() {
        // parse failure should return None
        let toml_path = PathBuf::from(
            ".cargo/AAAAAAHH/src/github.com-1ecc6299db9ec823/winapi-0.3.8/Cargo.toml",
        );
        let cargo_home = PathBuf::from(".cargo/");

        let name = find_crate_name_crate(&toml_path, &cargo_home);

        assert_eq!(name, None,);
    }
}
