use crate::cache::caches::Cache;
use crate::cache::caches::RegistrySubCache;
use crate::cache::caches::RegistrySuperCache;
use crate::cache::*;
use crate::library::*;

use flate2::read::GzDecoder;
use tar::Archive;

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct FileWithSize {
    path: PathBuf,
    size: u64,
}

impl FileWithSize {
    fn from_disk(path: &PathBuf) -> Self {
        FileWithSize {
            path: path.clone(),
            size: std::fs::metadata(path).unwrap().len(),
        }
    }

    // TODO: understand this R: Read stuff
    fn from_archive<'a, R: std::io::Read>(entry: &tar::Entry<'a, R>) -> Self {
        FileWithSize {
            path: entry.path().unwrap().into_owned(),
            size: entry.size(),
        }
    }
}
pub(crate) fn verify_crates(
    registry_pkg_caches: &mut registry_pkg_cache::RegistryPkgCaches,
    registry_sources_caches: &mut registry_sources::RegistrySourceCaches,
) -> Result<(), ()> {
    // iterate over all the extracted sources that we have

    let reg_sources = registry_sources_caches.items();
    let crate_gzips_and_sources: Vec<_> = reg_sources
        .iter()
        .map(|source| {
            // for each directory, find the path to the corresponding .crate archive
            // .cargo/registry/src/github.com-1ecc6299db9ec823/bytes-0.4.12
            // corresponds to
            // .cargo/registry/cache/github.com-1ecc6299db9ec823/bytes-0.4.12.crate

            // reverse, and "pop" the front components
            let mut dir = source.iter().collect::<Vec<&OsStr>>();

            let comp1 = dir.pop().unwrap(); // /bytes-0.4.12
            let comp2 = dir.pop().unwrap(); // github.com-1ecc6299db9ec823
            let _src = dir.pop().unwrap(); // throw this away and add "cache" instead

            // reconstruct the fixed path in reverse order

            dir.push(OsStr::new("cache"));
            dir.push(comp2); // github.com...
                             // we need to add the .crate extension (path to the gzip archive)
            let mut comp1_with_crate_ext = comp1.to_os_string();
            comp1_with_crate_ext.push(".crate");

            dir.push(&comp1_with_crate_ext); // bytes-0.4.12.crate
            let krate: PathBuf = dir.into_iter().collect::<PathBuf>();
            (source, krate)
        })
        // we need both the .crate and the directory for verification
        .filter(|(source, krate)| source.exists() && krate.exists())
        .collect();

    // this would fail if we for example have a crate source dir but no corresponding archive
    assert_eq!(crate_gzips_and_sources.len(), reg_sources.len());

    crate_gzips_and_sources
        .iter()
        .map(|(source, krate)| {
            let files_of_archive = {
                let tar_gz = File::open(krate).unwrap();
                // extract the tar
                let tar = GzDecoder::new(tar_gz);
                let mut archive = Archive::new(tar);

                let archive_files = archive.entries().unwrap();

                let x: Vec<_> = archive_files
                    .into_iter()
                    .map(|entry| {
                        let e = entry.unwrap();
                        e.path().unwrap().into_owned()
                    })
                    .collect();
                x
            };

            let files_of_source: Vec<_> = std::fs::read_dir(source)
                .unwrap()
                .map(|direntry| {
                    let x = direntry.unwrap();
                    x.path()
                })
                .collect();

            println!("{:?}|||||{:?}", files_of_archive, files_of_source);
        })
        .collect::<Vec<_>>();

    if false {
        return Err(());
    }
    Ok(())
}
