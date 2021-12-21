use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};

use crate::cache::caches::RegistrySuperCache;
use crate::cache::*;

use flate2::read::GzDecoder;
use rayon::iter::*;
use tar::Archive;
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FileWithSize {
    path: PathBuf,
    size: u64,
}

impl FileWithSize {
    fn from_disk(path_orig: &Path) -> Self {
        // we need to cut off .cargo/registry/src/github.com-1ecc6299db9ec823/
        let index = path_orig
            .iter()
            .enumerate()
            .position(|e| e.1 == OsStr::new("github.com-1ecc6299db9ec823").to_os_string())
            // @TODO fix this to be dynamic
            .unwrap()
            + 1;

        let path = path_orig.iter().skip(index).collect::<PathBuf>();

        FileWithSize {
            path,
            size: std::fs::metadata(path_orig).unwrap().len(),
        }
    }

    // TODO: understand this R: Read stuff
    fn from_archive<R: std::io::Read>(entry: &tar::Entry<'_, R>) -> Self {
        FileWithSize {
            path: entry.path().unwrap().into_owned(),
            size: entry.size(),
        }
    }
}

/// The Difference between extracted crate sources and an .crate tar.gz archive
#[derive(Debug, Clone)]
pub(crate) struct Diff {
    // the crate we are diffing
    krate_name: String,
    files_missing_in_checkout: Vec<PathBuf>,
    additional_files_in_checkout: Vec<PathBuf>,
    files_size_difference: Vec<FileSizeDifference>,
}

#[derive(Debug, Clone)]
pub(crate) struct FileSizeDifference {
    path: PathBuf,
    size_archive: u64,
    size_source: u64,
}

impl Diff {
    fn new() -> Self {
        Self {
            krate_name: String::new(),
            files_missing_in_checkout: Vec::new(),
            additional_files_in_checkout: Vec::new(),
            files_size_difference: Vec::new(),
        }
    }

    /// returns true if there is no diff
    fn is_ok(&self) -> bool {
        self.files_missing_in_checkout.is_empty()
            && self.additional_files_in_checkout.is_empty()
            && self.files_size_difference.is_empty()
    }

    pub(crate) fn details(&self) -> String {
        let mut s = format!("Crate: {}\n", self.krate_name);
        if !self.files_missing_in_checkout.is_empty() {
            s.push_str(&format!(
                "\nmissing files:\n{}",
                self.files_missing_in_checkout
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ));
        }
        if !self.additional_files_in_checkout.is_empty() {
            s.push_str(&format!(
                "\nadditional files:\n{}",
                self.additional_files_in_checkout
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ));
        }
        if !self.files_size_difference.is_empty() {
            s.push_str("\nFiles with differing size:\n");
            self.files_size_difference
                .iter()
                .map(|fsd| {
                    format!(
                        "File: {}, size in archive: {}b, size in checkout: {}b\n",
                        fsd.path.display(),
                        fsd.size_archive,
                        fsd.size_source
                    )
                })
                .for_each(|strg| s.push_str(&strg));
            s.push('\n');
        }
        s
    }
}
pub(crate) fn verify_crates(
    registry_sources_caches: &mut registry_sources::RegistrySourceCaches,
) -> Result<(), Vec<Diff>> {
    // iterate over all the extracted sources that we have

    let reg_sources = registry_sources_caches.items();
    let bad_sources: Vec<_> = reg_sources
        .par_iter()
        // get the paths to the source and the .crate for all extracted crates
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
        //  .collect();
        // this would fail if we for example have a crate source dir but no corresponding archive
        //  assert_eq!(crate_gzips_and_sources.len(), reg_sources.len());
        //let _x = crate_gzips_and_sources
        //  .iter()
        .map(|(source, krate)| {
            let krate_name = source.iter().last().unwrap();
            //println!("Verifying: {}", &krate_name.to_str().unwrap());
            // look into the .gz archive and get all the contained files+sizes
            let files_of_archive = {
                let tar_gz = File::open(krate).unwrap();
                // extract the tar
                let tar = GzDecoder::new(tar_gz);
                let mut archive = Archive::new(tar);

                let archive_files = archive.entries().unwrap();
                //  println!("files inside the archive");
                //  archive_files.for_each(|x| println!("{:?}", x.unwrap().path()));

                let x: Vec<_> = archive_files
                    .into_iter()
                    .map(|entry| FileWithSize::from_archive(&entry.unwrap()))
                    .collect();
                x
            };
            // get files + sizes of the crate extracted to disk
            // need to skip directories since the are only implicitly inside the tar (via file paths)
            let files_of_source: Vec<_> = WalkDir::new(source)
                .into_iter()
                .map(Result::unwrap)
                .filter(|de| de.file_type().is_file())
                .map(|direntry| {
                    let p = direntry.path();
                    p.to_owned()
                })
                .map(|p| FileWithSize::from_disk(&p))
                .collect();

            let mut diff = Diff::new();
            diff.krate_name = krate_name.to_str().unwrap().to_string();
            // compare

            let files_of_source_paths: Vec<&PathBuf> =
                files_of_source.iter().map(|fws| &fws.path).collect();

            for archive_file in &files_of_archive {
                let archive_f_path = &archive_file.path;
                if !files_of_source_paths.contains(&archive_f_path) {
                    // the file is contaied in the archive but not in the extracted source
                    diff.files_missing_in_checkout.push(archive_f_path.clone());
                } else if files_of_source_paths.contains(&archive_f_path) {
                    // file is contained in both, but sizes differ
                    match files_of_source
                        .iter()
                        .find(|fws| fws.path == archive_file.path)
                    {
                        Some(fws) => {
                            if fws.size != archive_file.size {
                                diff.files_size_difference.push(FileSizeDifference {
                                    path: fws.path.clone(),
                                    size_archive: archive_file.size,
                                    size_source: fws.size,
                                });
                            }
                        }
                        None => unreachable!(), // we already checked this
                    };
                }
            }

            let files_of_archive: Vec<&PathBuf> =
                files_of_archive.iter().map(|fws| &fws.path).collect();

            // cargo inserts ".cargo-ok" file to indicate that an archive has been fully extracted, ignore that too
            for source_file in files_of_source_paths
                .iter()
                .filter(|path| path.file_name().unwrap() != ".cargo-ok")
                .filter(|path| !path.is_dir() /* skip dirs */)
            {
                // dbg!(source_file);
                if !files_of_archive.iter().any(|path| path == source_file) {
                    diff.additional_files_in_checkout
                        .push(source_file.to_path_buf());
                }
            }

            // assert!(diff.files_size_difference.is_empty());
            diff
        })
        // save all the "bad" packages
        .filter(|diff| !diff.is_ok())
        .map(|diff| {
            eprintln!("Possibly corrupted source: {}", diff.krate_name);
            diff
        })
        .collect::<Vec<_>>();

    if bad_sources.is_empty() {
        Ok(())
    } else {
        Err(bad_sources)
    }
}
