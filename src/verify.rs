use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs::File;
use std::path::{Path, PathBuf};

use crate::cache::caches::RegistrySuperCache;
use crate::cache::*;
use crate::remove::remove_file;

use flate2::read::GzDecoder;
use rayon::iter::*;
use tar::Archive;
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FileWithSize {
    path: PathBuf,
    size: u64,
}

// #113 'verify' incorrectly determines paths as missing due to different unicode representations.
fn normalized(path: PathBuf) -> PathBuf {
    use unicode_normalization::{is_nfkc, UnicodeNormalization};
    match path.to_str() {
        Some(path) if !is_nfkc(path) => path.chars().nfc().collect::<String>().into(),
        _ => path,
    }
}

impl FileWithSize {
    fn from_disk(path_orig: &Path, krate_root: &OsStr) -> Self {
        // we need to cut off .cargo/registry/src/github.com-1ecc6299db9ec823/
        let index = path_orig
            .iter()
            .enumerate()
            .position(|e| e.1 == krate_root)
            .expect("must find cargo root in path contained within it");

        let path = path_orig.iter().skip(index).collect::<PathBuf>();

        FileWithSize {
            path: normalized(path),
            size: std::fs::metadata(path_orig).unwrap().len(),
        }
    }

    // TODO: understand this R: Read stuff
    fn from_archive<R: std::io::Read>(entry: &tar::Entry<'_, R>) -> Self {
        FileWithSize {
            path: normalized(entry.path().unwrap().into_owned()),
            size: entry.size(),
        }
    }
}

/// Size difference of a file in the .gz archive and extracted source
#[derive(Debug, Clone)]
pub(crate) struct FileSizeDifference {
    path: PathBuf,
    size_archive: u64,
    size_source: u64,
}

/// The Difference between extracted crate sources and an .crate tar.gz archive
#[derive(Debug, Clone)]
pub(crate) struct Diff {
    // the crate we are diffing
    krate_name: String,
    files_missing_in_checkout: Vec<PathBuf>,
    additional_files_in_checkout: Vec<PathBuf>,
    files_size_difference: Vec<FileSizeDifference>,
    source_path: Option<PathBuf>,
}

impl Diff {
    fn new() -> Self {
        Self {
            krate_name: String::new(),
            files_missing_in_checkout: Vec::new(),
            additional_files_in_checkout: Vec::new(),
            files_size_difference: Vec::new(),
            source_path: None,
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
            write!(
                s,
                "Missing from source:\n{}",
                self.files_missing_in_checkout
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            )
            .unwrap();
            s.push('\n');
        }
        if !self.additional_files_in_checkout.is_empty() {
            write!(
                s,
                "Not found in archive/additional:\n{}",
                self.additional_files_in_checkout
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            )
            .unwrap();
            s.push('\n');
        }
        if !self.files_size_difference.is_empty() {
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
        }
        s
    }
}

/// take a path to an extracted .crate source and map it to the corresponding .carte archive path
fn map_src_path_to_cache_path(src_path: &Path) -> PathBuf {
    // for each directory, find the path to the corresponding .crate archive
    // .cargo/registry/src/github.com-1ecc6299db9ec823/bytes-0.4.12
    // corresponds to
    // .cargo/registry/cache/github.com-1ecc6299db9ec823/bytes-0.4.12.crate

    // reverse, and "pop" the front components
    let mut dir = src_path.iter().collect::<Vec<&OsStr>>();

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
    dir.into_iter().collect::<PathBuf>()
}

/// look into the .gz archive and get all the contained files+sizes

fn sizes_of_archive_files(path: &Path) -> Vec<FileWithSize> {
    let tar_gz = File::open(path).unwrap();
    // extract the tar
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    let archive_files = archive.entries().unwrap();
    //  println!("files inside the archive");
    //  archive_files.for_each(|x| println!("{:?}", x.unwrap().path()));

    archive_files
        .into_iter()
        .map(|entry| FileWithSize::from_archive(&entry.unwrap()))
        .collect::<Vec<FileWithSize>>()
}

/// get the files and their sizes of the extracted .crate sources
fn sizes_of_src_dir(source: &Path) -> Vec<FileWithSize> {
    let krate_root = source.iter().last().unwrap();
    WalkDir::new(source)
        .into_iter()
        .map(Result::unwrap)
        // need to skip directories since the are only implicitly inside the tar (via file paths)
        .filter(|de| de.file_type().is_file())
        .map(|direntry| {
            let p = direntry.path();
            p.to_owned()
        })
        .map(|p| FileWithSize::from_disk(&p, krate_root))
        .collect()
}

/// compare files of a .crate gz archive and extracted sources and return a Diff object which describes those changes
fn diff_crate_and_source(krate: &Path, source: &Path) -> Diff {
    let files_of_archive: Vec<FileWithSize> = sizes_of_archive_files(krate);
    let files_of_source: Vec<FileWithSize> = sizes_of_src_dir(source);
    let mut diff = Diff::new();
    diff.source_path = Some(source.to_path_buf());
    diff.krate_name = source.iter().last().unwrap().to_str().unwrap().to_string();
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
    let files_of_archive: Vec<&PathBuf> = files_of_archive.iter().map(|fws| &fws.path).collect();
    for source_file in files_of_source_paths
        .iter()
        .filter(|path| path.file_name().unwrap() != ".cargo-ok")
        .filter(|path| !path.is_dir() /* skip dirs */)
    {
        // dbg!(source_file);
        #[allow(clippy::implicit_clone)]
        if !files_of_archive.iter().any(|path| path == source_file) {
            diff.additional_files_in_checkout
                .push(source_file.to_path_buf());
        }
    }
    diff
}
pub(crate) fn verify_crates(
    registry_sources_caches: &mut registry_sources::RegistrySourceCaches,
) -> Result<(), Vec<Diff>> {
    // iterate over all the extracted sources that we have

    let bad_sources: Vec<_> = registry_sources_caches
        .items()
        .par_iter()
        // get the paths to the source and the .crate for all extracted crates
        .map(|source| (source, map_src_path_to_cache_path(source)))
        // we need both the .crate and the directory to exist for verification
        .filter(|(source, krate)| source.exists() && krate.exists())
        // look into the .gz archive and get all the contained files+sizes
        .map(|(source, krate)| diff_crate_and_source(&krate, source))
        // save only the "bad" packages
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

pub(crate) fn clean_corrupted(
    registry_sources_caches: &mut registry_sources::RegistrySourceCaches,
    diff_list: &[Diff],
    dry_run: bool,
) {
    // hack because we need a &mut bool in remove_file()
    let mut bool = false;

    diff_list
        .iter()
        .filter_map(|diff| diff.source_path.as_ref())
        .filter(|path| path.is_dir())
        .for_each(|path| {
            remove_file(
                path,
                dry_run,
                &mut bool,
                Some(format!("removing corrupted source: {}", path.display())),
                &crate::remove::DryRunMessage::Default,
                // we don't print a summary or anything (yet..)
                None,
            );
        });

    // just in case
    registry_sources_caches.invalidate();
}

#[cfg(test)]
mod verification_tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_map_src_path_to_cache_path() {
        let old_src_path = PathBuf::from(
            "/home/matthias/.cargo/registry/src/github.com-1ecc6299db9ec823/bytes-0.4.12",
        );
        let new_archive_path = PathBuf::from(
            "/home/matthias/.cargo/registry/cache/github.com-1ecc6299db9ec823/bytes-0.4.12.crate",
        );

        let new = map_src_path_to_cache_path(&old_src_path);

        assert_eq!(new, new_archive_path);
    }
}
