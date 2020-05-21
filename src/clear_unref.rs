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

    let deps = metadata.packages.into_iter().map(|p| Dep {
        name: p.name.clone(),
        version: p.version.to_string(),
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

    Ok(())
}
