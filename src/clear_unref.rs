// Copyright 2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// remove all crates from a cache that are not referenced by a Cargo lockfile

use std::ffi::OsStr;
use std::path::PathBuf;

use crate::library::Error;

use cargo_metadata::{CargoOpt, MetadataCommand};

// data we need on a crate dependency
#[derive(Debug, Clone)]
struct Dep {
    name: String,
    version: String,
    source: SourceKind,
}

// the source of a crate inside the cargo cache can be represented in form of
// an extracted .crate or a checked out git repository
// the path is the absolute path to the source inside the ${CARGO_HOME}
#[derive(Debug, Clone)]
enum SourceKind {
    Crate(PathBuf),
    Git(PathBuf),
}

// get a list of all dependencies that we have
fn get_deps(cargo_home: &PathBuf) -> Result<impl Iterator<Item = Dep>, Error> {
    let manifest = crate::local::get_manifest()?;

    let metadata = MetadataCommand::new()
        .manifest_path(&manifest)
        .features(CargoOpt::AllFeatures)
        .exec()
        .unwrap_or_else(|error| {
            panic!(
                "Failed to parse manifest: '{}'\nError: '{:?}'",
                &manifest.display(),
                error
            )
        });

    let cargo_home = cargo_home.clone();

    #[allow(clippy::filter_map)]
    let deps = metadata
        .packages
        .into_iter()
        // skip local packages, only check packages that come from the cargo-cache
        //https://docs.rs/cargo_metadata/0.10.0/cargo_metadata/struct.Package.html#structfield.source
        .filter(|package| package.source.is_some())
        .map(move |p| {
            let is_git: bool = p.id.repr.contains("(git+");
            let toml_path = p.manifest_path;

            let source = if is_git {
                SourceKind::Git(find_crate_name_git(&toml_path, &cargo_home))
            } else {
                SourceKind::Crate(find_crate_name_crate(&toml_path, &cargo_home))
            };

            Dep {
                version: p.version.to_string(),
                name: p.name,
                source, // @TODO get the source path
            }
        });

    Ok(deps)
}

pub(crate) fn clear_unref(cargo_home: &PathBuf) -> Result<(), Error> {
    let deps = get_deps(cargo_home)?;
    // @TODO: check the cache for any crates that are not these and remove them
    deps.for_each(|dep| {
        let fmt = format!("{}-{}", dep.name, dep.version);
        println!("{}", fmt);
    });

    // we have acquired a list of all dependencies needed by a project.

    let manifest = crate::local::get_manifest().unwrap();

    let metadata = MetadataCommand::new()
        .manifest_path(&manifest)
        .features(CargoOpt::AllFeatures)
        .exec()
        .unwrap_or_else(|error| {
            panic!(
                //@FIXME
                "Failed to parse manifest: '{}'\nError: '{:?}'",
                &manifest.display(),
                error
            )
        });
    let pkgs = metadata.packages;
    for pkg in pkgs {
        println!("{:?}\n\n\n", pkg);
    }

    Ok(())
}

// NOTE we need to skip the toml of the root project

fn find_crate_name_git(toml_path: &PathBuf, cargo_home: &PathBuf) -> PathBuf {
    //  ~/.cargo/registry/src/github.com-1ecc6299db9ec823/winapi-0.3.8/Cargo.toml => ~/.cargo/registry/src/github.com-1ecc6299db9ec823/winapi-0.3.8/

    // get the segments of the path
    let v: Vec<&OsStr> = toml_path.iter().collect();

    let checkouts_pos = v
        .iter()
        .position(|i| i == &"checkouts")
        .unwrap_or_else(|| panic!("failed to parse! 1: {:?}", v)); //@FIXME

    // assuming git:
    // git checkouts repo-name ref
    let path_segments = &v[(checkouts_pos - 1)..(checkouts_pos + 3)];

    let mut path = cargo_home.clone();
    path_segments.iter().for_each(|p| path.push(p));

    path
}

fn find_crate_name_crate(toml_path: &PathBuf, cargo_home: &PathBuf) -> PathBuf {
    // ~/.cargo/git/checkouts/home-fb9469891e5cfbe6/3a6eccd  => ~/.cargo/git/checkouts/home-fb9469891e5cfbe6/3a6eccd/

    let v: Vec<&OsStr> = toml_path.iter().collect();
    let registry_pos = v
        .iter()
        .position(|i| i == &"registry")
        .unwrap_or_else(|| panic!("failed to parse! 2: {:?}", v)); //@FIXME

    let path_segments = &v[(registry_pos)..(registry_pos + 4)];
    let mut path = cargo_home.clone();
    path_segments.iter().for_each(|p| path.push(p));
    path
}
