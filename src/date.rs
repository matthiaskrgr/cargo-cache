use crate::cache::*;

use std::fs;

pub(crate) fn dates(reg_cache: &mut registry_sources::RegistrySourceCaches) {
    let files = reg_cache.total_checkout_folders();

    for file in files {
        let m = file.metadata();
    }

    let mut dates = files
        .iter()
        .map(|f| f.metadata().unwrap().accessed().unwrap())
        .collect::<Vec<_>>();

    dates.sort();

    println!("{:?}", dates);
}
