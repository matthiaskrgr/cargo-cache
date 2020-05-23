// Copyright 2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// remove all crates from a cache that are not referenced by a Cargo lockfile

use crate::library::Error;

use cargo_metadata::{CargoOpt, MetadataCommand};

#[derive(Debug, Clone)]
struct Dep {
    name: String,
    version: String,
    is_git: bool,
}

fn get_deps() -> Result<impl Iterator<Item = Dep>, Error> {
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

    let deps = metadata.packages.into_iter().map(|p| {
        let is_git: bool = p.id.repr.contains("(git+");
        let path_in_cacheb = p.manifest_path;

        return Dep {
            version: p.version.to_string(),
            name: p.name,
            is_git,
            // @TODO get the source path
        };
    });

    Ok(deps)
}

pub(crate) fn clear_unref() -> Result<(), Error> {
    let deps = get_deps()?;
    // @TODO: check the cache for any crates that are not these and remove them
    deps.for_each(|dep| {
        let fmt = format!("{}-{}", dep.name, dep.version);
        println!("{}", fmt);
    });

    let deps = get_deps()?; //@TODO remove

    // we have acquired a list of all dependencies needed by a project.

    let manifest = crate::local::get_manifest().unwrap();

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

    let pkgs = metadata.packages;
    for pkg in pkgs {
        println!("{:?}\n\n\n", pkg);
    }

    Ok(())
}
