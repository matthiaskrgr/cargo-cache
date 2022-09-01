// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/// This file provides core logic of the crate
use std::fmt;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cache::caches::{Cache, RegistrySuperCache};
use crate::cache::*;
use crate::dirsizes::DirSizes;

use humansize::{file_size_opts, FileSize};
use rayon::iter::*;
use walkdir::WalkDir;

// lets us call let z =  None.unwrap_oe_exit_with_error();
pub(crate) type CargoCacheResult<T, E> = Result<T, E>;
pub(crate) trait ErrorHandling<T, E: std::fmt::Display> {
    fn unwrap_or_fatal_error(self) -> T;
    fn exit_or_fatal_error(self);
}

impl<T, E: std::fmt::Display> ErrorHandling<T, E> for CargoCacheResult<T, E> {
    /// return the wrapped value or print the wrapped error and terminate cargo-cache
    fn unwrap_or_fatal_error(self) -> T {
        match self {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    }

    /// print the wrapped value or print the wrapped error and exit with 0 or 1 respectively
    fn exit_or_fatal_error(self) {
        match self {
            Ok(_) => {
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    }
}

/// `DirInfo` is used so to be able to easily differentiate between size and number of files of a directory
#[derive(Debug, Clone)]
pub(crate) struct DirInfo {
    // make sure we do not accidentally confuse dir_size and file_number
    // since both are of the same type
    /// size of a directory
    pub(crate) dir_size: u64,
    /// number of files of a directory
    #[allow(unused)] // used in tests iirc
    pub(crate) file_number: u64,
}
/// `CargoCachePaths` contains paths to all the subcomponents of the cargo cache
#[derive(Debug, Clone)]
pub(crate) struct CargoCachePaths {
    /// the root path to the cargo home
    pub(crate) cargo_home: PathBuf,
    /// the directory where installed (cargo install..) binaries are located
    pub(crate) bin_dir: PathBuf,
    /// path where registries are stored
    pub(crate) registry: PathBuf,
    /// path where registry caches are stored (the .crate archives)
    pub(crate) registry_pkg_cache: PathBuf,
    /// path where registry sources (.rs files / extracted .crate archives) are stored
    pub(crate) registry_sources: PathBuf,
    /// path where the registry indices (git repo containing information on available crates, versions etc) are stored
    pub(crate) registry_index: PathBuf,
    /// bare git repositories are stored here
    pub(crate) git_repos_bare: PathBuf,
    /// git repository checkouts are stored here
    pub(crate) git_checkouts: PathBuf,
}

/// possible errors the crate may encounter, most of them unrecoverable
#[derive(Debug)]
pub(crate) enum Error {
    /// git-rs failed to open a git repo
    GitRepoNotOpened(PathBuf),
    /// a repository expected to be a git repo was not found
    GitRepoDirNotFound(PathBuf),
    /// git gc errored
    GitGCFailed(PathBuf, std::io::Error),
    /// git pack-refs errored
    GitPackRefsFailed(PathBuf, std::io::Error),
    /// git reflog errored
    GitReflogFailed(PathBuf, std::io::Error),
    /// git fsck errored
    GitFsckFailed(PathBuf, std::io::Error),
    /// git repack errored
    GitRepackFailed(PathBuf, std::io::Error),
    /// git seems to be missing from the system
    GitNotInstalled,
    /// a package name inside the cache failed to parse
    MalformedPackageName(String),
    /// could not get the cargo home directory
    GetCargoHomeFailed,
    /// cargo-home exists but is not a directory
    CargoHomeNotDirectory(PathBuf),
    /// one of the parameters of --remove-dir was not recognized
    InvalidDeletableDirs(String),
    /// --remove-dir didn't get any args passed
    RemoveDirNoArg,
    /// failed to find current working directory
    NoCWD,
    /// failed to find Cargo.toml manifest
    NoCargoManifest(PathBuf),
    /// failed to parse query regex
    QueryRegexFailedParsing(String),
    /// tried to "git gc" a file instead of a directory
    GitGCFile(PathBuf),
    // local tried to open a target dir that does not exist
    LocalNoTargetDir(PathBuf),
    // failed to parse date given to younger or older
    DateParseFailure(String, String),
    // cargo metadata failed to parse a cargo manifest
    UnparsableManifest(PathBuf, cargo_metadata::Error),
    // could not find sccache cache dir
    NoSccacheDir,
    // could not get rustup home
    NoRustupHome,
    // trim failed to parse the given unit
    TrimLimitUnitParseFailure(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let valid_deletable_dirs =
            "git-db,git-repos,registry-sources,registry-crate-cache,registry-index,registry,all";

        match &self {
            Self::GitRepoNotOpened(path) => {
                write!(f, "Failed to open git repository at \"{}\"", path.display())
            }

            Self::GitRepoDirNotFound(path) => {
                write!(f, "Git repo \"{}\" not found", path.display())
            }

            Self::GitGCFailed(path, error) => write!(
                f,
                "Failed to git gc repository \"{}\":\n{:?}",
                path.display(),
                error
            ),

            Self::GitPackRefsFailed(path, error) => write!(
                f,
                "Failed to git pack-refs repository \"{}\":\n{:?}",
                path.display(),
                error
            ),

            Self::GitReflogFailed(path, error) => write!(
                f,
                "Failed to git reflog repository \"{}\":\n{:?}",
                path.display(),
                error
            ),

            Self::GitFsckFailed(path, error) => write!(
                f,
                "Failed to git fsck repository \"{}\":\n{:?}",
                path.display(),
                error
            ),

            Self::GitRepackFailed(path, error) => write!(
                f,
                "Failed to git repack repository \"{}\":\n{:?}",
                path.display(),
                error
            ),

            Self::GitNotInstalled => write!(f, "Could not find 'git' binary. Is 'git' installed?",),

            Self::MalformedPackageName(pkgname) => {
                write!(f, "Error:  \"{}\" is not a valid package name", pkgname)
            }

            Self::GetCargoHomeFailed => write!(f, "Failed to get CARGO_HOME!"),

            Self::CargoHomeNotDirectory(path) => write!(
                f,
                "CARGO_HOME \"{}\" is not an existing directory!",
                path.display()
            ),

            Self::InvalidDeletableDirs(dirs) => write!(
                f,
                "\"{}\" are not valid removable directories! Chose one or several from {}",
                dirs, valid_deletable_dirs
            ),

            Self::RemoveDirNoArg => write!(
                f,
                "No argument passed to \"--remove-dir\"! Chose one or several from {}",
                valid_deletable_dirs
            ),
            Self::NoCWD => write!(f, "Failed to find current working directory!",),
            Self::NoCargoManifest(dir) => write!(
                f,
                "Failed to Cargo.toml manifest in {} or downwards.",
                dir.display()
            ),
            Self::QueryRegexFailedParsing(regex) => {
                write!(f, "Failed to parse regular expression \"{}\"", regex)
            }
            Self::GitGCFile(path) => write!(
                f,
                "Tried to \"git gc\" a file instead of a directory: \"{}\"",
                path.display()
            ),
            Self::LocalNoTargetDir(path) => write!(
                f,
                "error: \"local\" subcommand tried to read \"target\" \
                directory that does not exist: \"{}\"",
                path.display()
            ),
            Self::DateParseFailure(date, error) => {
                write!(f, "ERROR failed to parse {} as date {}", date, error)
            }
            Self::UnparsableManifest(path, error) => write!(
                f,
                "Failed to parse Cargo.toml at '{}': '{:?}'",
                path.display(),
                error
            ),

            Self::NoSccacheDir => {
                write!(f,
                "Could not find sccache cache directory at ~/.cache/sccache or ${{SCCACHE_DIR}}")
            }
            Self::NoRustupHome => write!(f, "Failed to determine rustup home directory"),
            Self::TrimLimitUnitParseFailure(limit) => write!(
                f,
                "Failed to parse limit: \"{}\". \
                Should be of the form 123X where X is one of B,K,M,G or T.",
                limit
            ),
        }
    }
}

impl CargoCachePaths {
    /// returns `CargoCachePaths` object which makes all the subpaths accessible to the crate
    pub(crate) fn default() -> Result<Self, Error> {
        let cargo_home = if let Ok(cargo_home) = home::cargo_home() {
            cargo_home
        } else {
            return Err(Error::GetCargoHomeFailed);
        };

        if !cargo_home.is_dir() {
            return Err(Error::CargoHomeNotDirectory(cargo_home));
        }
        // get the paths to the relevant directories
        let bin = cargo_home.join("bin");
        let registry = cargo_home.join("registry");
        let registry_index = registry.join("index");
        let reg_cache = registry.join("cache");
        let reg_src = registry.join("src");
        let git_repos_bare = cargo_home.join("git").join("db");
        let git_checkouts = cargo_home.join("git").join("checkouts");

        Ok(Self {
            cargo_home,
            bin_dir: bin,
            registry,
            registry_index,
            registry_pkg_cache: reg_cache,
            registry_sources: reg_src,
            git_repos_bare,
            git_checkouts,
        })
    }
} // impl CargoCachePaths

// this is the output of `cargo cache --list-dirs`
impl std::fmt::Display for CargoCachePaths {
    fn fmt(&self, f: &'_ mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "\ncargo home:                 {}",
            &self.cargo_home.display()
        )?;
        writeln!(f, "binaries directory:         {}", &self.bin_dir.display())?;
        writeln!(
            f,
            "registry directory:         {}",
            &self.registry.display()
        )?;
        writeln!(
            f,
            "registry index:             {}",
            &self.registry_index.display()
        )?;
        writeln!(
            f,
            "crate source archives:      {}",
            &self.registry_pkg_cache.display()
        )?;
        writeln!(
            f,
            "unpacked crate sources:     {}",
            &self.registry_sources.display()
        )?;
        writeln!(
            f,
            "bare git repos:             {}",
            &self.git_repos_bare.display()
        )?;
        writeln!(
            f,
            "git repo checkouts:         {}",
            &self.git_checkouts.display()
        )?;

        Ok(())
    }
}

// these are everything what we can specify to remove via --remove-dir or similar options
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum RemovableGroup {
    All,
    GitDB,
    GitRepos,
    RegistrySources,
    RegistryCrateCache,
    RegistryIndex,
    Registry,
}

impl std::str::FromStr for RemovableGroup {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "all" => Ok(RemovableGroup::All),
            "git-db" => Ok(RemovableGroup::GitDB),
            "git-repos" => Ok(RemovableGroup::GitRepos),
            "registry-sources" => Ok(RemovableGroup::RegistrySources),
            "registry-crate-cache" => Ok(RemovableGroup::RegistryCrateCache),
            "registry-index" => Ok(RemovableGroup::RegistryIndex),
            "registry" => Ok(RemovableGroup::Registry),
            other => Err(other.to_string()),
        }
    }
}

// these are the actual atomic components of the cache
// we have to map the RemovableGroups to the Components, deduplicate and finally remove them
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Component {
    GitDB,              // git/db
    GitRepos,           // git/checkouts
    RegistrySources,    // registry/src
    RegistryCrateCache, // registry/cache
    RegistryIndex,      // registry/index
}

// map a String to a list of RemovableGroups to actual Components
// returns either a group of successfully converted Components or a list of unrecognized
// RemovableGroups as Error
pub(crate) fn components_from_groups(input: Option<&str>) -> Result<Vec<Component>, Error> {
    let input_string = if let Some(value) = input {
        value
    } else {
        return Err(Error::RemoveDirNoArg);
    };

    // sort failed and successful parses
    #[allow(clippy::type_complexity)]
    let (dirs, errors): (
        Vec<Result<RemovableGroup, String>>,
        Vec<Result<RemovableGroup, String>>,
    ) = input_string
        .split(',')
        .map(str::parse)
        .partition(Result::is_ok);

    // we got errors, abort
    if !errors.is_empty() {
        let invalid_dirs = errors
            .into_iter()
            .map(|e| e.err().unwrap())
            .collect::<Vec<String>>();

        let inv_dirs_joined = invalid_dirs.join(" ");
        let inv_dirs_trimmed = inv_dirs_joined.trim();
        //@TODO fix this enum variant name to be more
        return Err(Error::InvalidDeletableDirs(inv_dirs_trimmed.to_string()));
    }

    // at this point we were able to parse all the user input.

    // map the RemovableGroups to Dirs

    // unwrap the Results
    let dirs = dirs.into_iter().map(|d| d.ok().unwrap());

    let mut mapped_dirs = Vec::new();

    dirs.for_each(|dir| match dir {
        RemovableGroup::All => {
            mapped_dirs.extend(
                // everything
                vec![
                    Component::GitDB,
                    Component::GitRepos,
                    Component::RegistrySources,
                    Component::RegistryCrateCache,
                    Component::RegistryIndex,
                ],
            );
        }
        RemovableGroup::GitDB => {
            mapped_dirs.extend(vec![Component::GitDB, Component::GitRepos]);
        }
        RemovableGroup::GitRepos => {
            mapped_dirs.push(Component::GitRepos);
        }
        RemovableGroup::RegistrySources => {
            mapped_dirs.push(Component::RegistrySources);
        }
        RemovableGroup::RegistryCrateCache => {
            mapped_dirs.extend(vec![
                Component::RegistrySources,
                Component::RegistryCrateCache,
            ]);
        }
        RemovableGroup::RegistryIndex => {
            mapped_dirs.push(Component::RegistryIndex);
        }
        RemovableGroup::Registry => mapped_dirs.extend(vec![
            Component::RegistrySources,
            Component::RegistryCrateCache,
        ]),
    });

    // remove duplicates
    mapped_dirs.sort();
    mapped_dirs.dedup();

    Ok(mapped_dirs)
}

/// get the total size of a directory or a file
pub(crate) fn size_of_path(path: &Path) -> u64 {
    // if the path is a directory, use cumulative_dir_size
    if path.is_dir() {
        cumulative_dir_size(path).dir_size
    } else {
        fs::metadata(path)
            .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &path.display()))
            .len()
    }
}

/// get the total size and number of files of a directory
pub(crate) fn cumulative_dir_size(dir: &Path) -> DirInfo {
    // Note: using a hashmap to cache dirsizes does apparently not pay out performance-wise
    if !dir.is_dir() {
        return DirInfo {
            dir_size: 0,
            file_number: 0,
        };
    }

    // traverse recursively and sum filesizes, parallelized by rayon
    let walkdir_start = dir.display().to_string();

    let dir_size = WalkDir::new(&walkdir_start)
        .into_iter()
        .map(|e| e.unwrap().path().to_owned())
        .filter(|f| f.exists()) // avoid broken symlinks
        .collect::<Vec<_>>() // @TODO perhaps WalkDir will impl ParallelIterator one day
        .par_iter()
        .filter(|f| f.exists()) // check if the file still exists. Since collecting and processing a
        // path, some time may have passed and if we have a "cargo build" operation
        // running in the directory, a temporary file may be gone already and failing to unwrap() (#43)
        .map(|f| {
            fs::metadata(f)
                .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &f.display()))
                .len()
        })
        .sum();

    // for the file number, we don't want the actual number of files but only the number of
    // files in the current directory, limit search depth

    let file_number = if walkdir_start.contains("registry") {
        WalkDir::new(&walkdir_start)
            .max_depth(2)
            .min_depth(2)
            .into_iter()
            .count()
    } else {
        fs::read_dir(dir).unwrap().count()
    } as u64;

    DirInfo {
        dir_size,
        file_number,
    }
}

/// "cargo cache --info" output
pub(crate) fn get_info(c: &CargoCachePaths, s: &DirSizes<'_>) -> String {
    let mut strn = String::with_capacity(1500);

    if let Ok(cache_path) = std::env::var("CARGO_HOME") {
        writeln!(
            strn,
            "${{CARGO_HOME}} env var set to '{}', using that!",
            cache_path
        )
        .unwrap();
    } else {
        writeln!(
            strn,
            "Default cache dir found: '{}', using that!",
            c.cargo_home.display()
        )
        .unwrap();
    };

    strn.push('\n');

    writeln!(
        strn,
        "Total cache size: {}\n",
        s.total_size().file_size(file_size_opts::DECIMAL).unwrap()
    )
    .unwrap();

    strn.push_str(&c.bin_dir.display().to_string());
    strn.push('\n');
    writeln!(
        strn,
        "\t{} binaries installed in binary directory, total size: {}",
        s.numb_bins(),
        s.total_bin_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    )
    .unwrap();
    strn.push_str("\tThese are the binaries installed via 'cargo install'.\n");
    strn.push_str("\tUse 'cargo uninstall' to remove binaries if needed.\n");
    strn.push('\n');

    strn.push_str(&c.registry.display().to_string());
    strn.push('\n');
    writeln!(
        strn,
        "\tRegistry root dir, size: {}",
        s.total_reg_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    )
    .unwrap();
    strn.push_str("\tCrate registries are stored here.\n");
    strn.push('\n');

    strn.push_str(&c.registry_index.display().to_string());
    strn.push('\n');
    writeln!(
        strn,
        "\tRegistry index, size: {}",
        s.total_reg_index_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    )
    .unwrap();
    strn.push_str("\tA git repo holding information on what crates are available.\n");
    strn.push_str("\tWill be recloned as needed.\n");

    strn.push('\n');

    // source archives are extracted here, will be reextracted from the downloaded source if removed
    strn.push_str(&c.registry_pkg_cache.display().to_string());
    strn.push('\n');
    writeln!(
        strn,
        "\tCrate source package archive, size: {}",
        s.total_reg_cache_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    )
    .unwrap();

    strn.push_str("\tCrates source packages of the registries are downloaded into this folder.\n");
    strn.push_str("\tThey will be redownloaded as needed.\n");
    strn.push('\n');

    strn.push_str(&c.registry_sources.display().to_string());
    strn.push('\n');

    writeln!(
        strn,
        "\tCrate sources, size: {}",
        s.total_reg_src_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    )
    .unwrap();
    strn.push_str("\tSource archives are extracted into this dir.\n");
    strn.push_str("\tThey will be reextracted from the package archive as needed.\n");
    strn.push('\n');

    strn.push_str(&c.git_repos_bare.display().to_string());
    strn.push('\n');
    writeln!(
        strn,
        "\tGit database, size: {}",
        s.total_git_repos_bare_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    )
    .unwrap();
    strn.push_str("\tBare repos of git dependencies are stored here.\n");
    strn.push_str("\tRemoved git repositories will be recloned as needed.\n");
    strn.push('\n');

    strn.push_str(&c.git_checkouts.display().to_string());
    strn.push('\n');
    writeln!(
        strn,
        "\tGit repo checkouts, size: {}",
        s.total_git_chk_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    )
    .unwrap();
    strn.push_str("\tSpecific commits of the bare repos will be checked out into here.\n");
    strn.push_str("\tGit checkouts will be rechecked-out from repo database as needed.");
    //println!("{}", strn.len());
    strn
}

//@TODO add tests
/// provides a textual summary of changes (of file sizes)
pub(crate) fn size_diff_format(
    size_before: u64,
    size_after: u64,
    display_size_before: bool,
) -> String {
    #[allow(clippy::cast_possible_wrap)]
    let size_diff: i64 = size_after as i64 - size_before as i64;
    let sign = if size_diff > 0 { "+" } else { "" };
    let size_after_human_readable = size_after.file_size(file_size_opts::DECIMAL).unwrap();
    let humansize_opts = file_size_opts::FileSizeOpts {
        allow_negative: true,
        ..file_size_opts::DECIMAL
    };
    let size_diff_human_readable = size_diff.file_size(humansize_opts).unwrap();
    let size_before_human_readabel = size_before.file_size(file_size_opts::DECIMAL).unwrap();
    // calculate change in percentage
    // when printing, we are going to cut off everything but a few decimal places anyway, so
    // precision is not much of an issue.

    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let perc: f32 =
        (((size_after as f64 / size_before as f64) * f64::from(100)) - f64::from(100)) as f32;
    // truncate to 2 decimal digits
    let percentage: f32 = ((perc * f32::from(100_i8)).trunc()) / (f32::from(100_u8));

    if size_before == size_after {
        if display_size_before {
            format!(
                "{} => {}",
                size_before_human_readabel, size_after_human_readable
            )
        } else {
            size_after_human_readable
        }
    } else if display_size_before {
        format!(
            "{} => {} ({}{}, {}%)",
            size_before_human_readabel,
            size_after_human_readable,
            sign,
            size_diff_human_readable,
            percentage
        )
    } else {
        format!(
            "{} ({}{}, {}%)",
            size_after_human_readable, sign, size_diff_human_readable, percentage
        )
    }
}

// @TODO make this function obsolete
#[allow(clippy::too_many_arguments)]
pub(crate) fn print_size_changed_summary(
    previous_total_size: u64,
    cargo_cache: &CargoCachePaths,
    bin_cache: &mut bin::BinaryCache,
    checkouts_cache: &mut git_checkouts::GitCheckoutCache,
    bare_repos_cache: &mut git_bare_repos::GitRepoCache,
    registry_pkgs_cache: &mut registry_pkg_cache::RegistryPkgCaches,
    registry_index_caches: &mut registry_index::RegistryIndicesCache,
    registry_sources_caches: &mut registry_sources::RegistrySourceCaches,
) {
    // and invalidate the cache
    bin_cache.invalidate();
    checkouts_cache.invalidate();
    bare_repos_cache.invalidate();
    registry_pkgs_cache.invalidate();
    registry_index_caches.invalidate();
    registry_sources_caches.invalidate();

    // and requery it to let it do its thing
    let cache_size_new = DirSizes::new(
        bin_cache,
        checkouts_cache,
        bare_repos_cache,
        registry_pkgs_cache,
        registry_index_caches,
        registry_sources_caches,
        cargo_cache,
    )
    .total_size();

    let size_old_human_readable = previous_total_size
        .file_size(file_size_opts::DECIMAL)
        .unwrap();
    println!(
        "\nSize changed from {} to {}",
        size_old_human_readable,
        size_diff_format(previous_total_size, cache_size_new, false)
    );
}

#[cfg(test)]
mod libtests {
    use super::*;

    use pretty_assertions::assert_eq;
    use regex::Regex;
    use std::env;

    use crate::test_helpers::assert_path_end;

    impl CargoCachePaths {
        pub(crate) fn new(dir: PathBuf) -> Result<Self, Error> {
            if !dir.is_dir() {
                return Err(Error::CargoHomeNotDirectory(dir));
            }

            // get the paths to the relevant directories
            let cargo_home = dir;
            let bin = cargo_home.join("bin");
            let registry = cargo_home.join("registry");
            let registry_index = registry.join("index");
            let reg_cache = registry.join("cache");
            let reg_src = registry.join("src");
            let git = cargo_home.join("git");
            let git_repos_bare = git.join("db");
            let git_checkouts = git.join("checkouts");

            Ok(Self {
                cargo_home,
                bin_dir: bin,
                registry,
                registry_index,
                registry_pkg_cache: reg_cache,
                registry_sources: reg_src,
                git_repos_bare,
                git_checkouts,
            })
        }
    }

    #[allow(non_snake_case)]
    #[test]
    fn test_DirInfo() {
        let x = DirInfo {
            dir_size: 10,
            file_number: 20,
        };
        assert_eq!(x.dir_size, 10);
        assert_eq!(x.file_number, 20);
    }

    #[allow(non_snake_case)]
    #[test]
    fn test_CargoCachePaths_gen() {
        // set cargo cache root dir to /tmp
        let dir_paths = CargoCachePaths::new(env::temp_dir());
        assert!(dir_paths.is_ok(), "dir paths: {:?}", dir_paths);
    }

    #[allow(non_snake_case)]
    #[test]
    fn test_CargoCachePaths_paths() {
        // create a new directory

        // get cargo target dir
        let target_dir = cargo_metadata::MetadataCommand::new()
            .exec()
            .unwrap()
            .target_directory;

        let mut cargo_home = PathBuf::from(target_dir);
        cargo_home.push("cargo_home_cargo_cache_paths");
        // make sure this worked
        let CH_string = format!("{}", cargo_home.display());
        // assert that cargo_home var is cargo-cache/target/cargo_home_cargo_cache_paths
        assert_path_end(
            &cargo_home,
            &["cargo-cache", "target", "cargo_home_cargo_cache_paths"],
        );

        // create the directory: cargo-cache/target/cargo_home_cargo_cache_paths
        if !std::path::PathBuf::from(&CH_string).is_dir() {
            std::fs::DirBuilder::new().create(&CH_string).unwrap();
        }
        assert!(fs::metadata(&CH_string).unwrap().is_dir());
        assert!(std::path::PathBuf::from(&CH_string).is_dir());

        let ccp = CargoCachePaths::new(PathBuf::from(CH_string)).unwrap();

        // test all the paths
        assert_path_end(&ccp.cargo_home, &["cargo_home_cargo_cache_paths"]);

        assert_path_end(&ccp.bin_dir, &["cargo_home_cargo_cache_paths", "bin"]);

        assert_path_end(&ccp.registry, &["cargo_home_cargo_cache_paths", "registry"]);

        assert_path_end(
            &ccp.registry_index,
            &["cargo_home_cargo_cache_paths", "registry", "index"],
        );

        assert_path_end(
            &ccp.registry_pkg_cache,
            &["cargo_home_cargo_cache_paths", "registry", "cache"],
        );

        assert_path_end(
            &ccp.registry_sources,
            &["cargo_home_cargo_cache_paths", "registry", "src"],
        );

        assert_path_end(
            &ccp.git_repos_bare,
            &["cargo_home_cargo_cache_paths", "git", "db"],
        );

        assert_path_end(
            &ccp.git_checkouts,
            &["cargo_home_cargo_cache_paths", "git", "checkouts"],
        );
    }

    #[allow(non_snake_case)]
    #[test]
    fn test_CargoCachePaths_print() {
        // test --list-dirs output

        // get cargo target dir
        let target_dir = cargo_metadata::MetadataCommand::new()
            .exec()
            .unwrap()
            .target_directory;

        let mut cargo_home = PathBuf::from(target_dir);
        cargo_home.push("cargo_home_cargo_cache_paths_print");
        //make sure this worked
        let CH_string = format!("{}", cargo_home.display());
        assert_path_end(
            &cargo_home,
            &[
                "cargo-cache",
                "target",
                "cargo_home_cargo_cache_paths_print",
            ],
        );

        // create the directory
        if !std::path::PathBuf::from(&CH_string).exists() {
            std::fs::DirBuilder::new().create(&CH_string).unwrap();
        }
        assert!(fs::metadata(&CH_string).unwrap().is_dir());
        assert!(std::path::PathBuf::from(&CH_string).is_dir());

        // set cargo home to this directory
        let ccp = CargoCachePaths::new(PathBuf::from(CH_string)).unwrap();

        let output = ccp.to_string();
        let mut iter = output.lines().skip(1); // ??

        let cargo_home2 = iter.next().unwrap();

        assert!(
            Regex::new(if cfg!(windows) {
                r"cargo home:.*\\cargo_home_cargo_cache_paths_print"
            } else {
                r"cargo home:.*/cargo_home_cargo_cache_paths_print"
            })
            .unwrap()
            .is_match(cargo_home2),
            "cargo home: \"{:?}\"",
            cargo_home
        );

        let bins = iter.next().unwrap();
        assert!(Regex::new(if cfg!(windows) {
            r"binaries directory:.*\\cargo_home_cargo_cache_paths_print\\bin"
        } else {
            r"binaries directory:.*/cargo_home_cargo_cache_paths_print/bin"
        })
        .unwrap()
        .is_match(bins));

        let registry = iter.next().unwrap();
        assert!(Regex::new(if cfg!(windows) {
            r"registry directory:.*\\cargo_home_cargo_cache_paths_print\\registry"
        } else {
            r"registry directory:.*/cargo_home_cargo_cache_paths_print/registry"
        })
        .unwrap()
        .is_match(registry));

        let registry_index = iter.next().unwrap();
        assert!(Regex::new(if cfg!(windows) {
            r"registry index:.*\\cargo_home_cargo_cache_paths_print\\registry\\index"
        } else {
            r"registry index:.*/cargo_home_cargo_cache_paths_print/registry/index"
        })
        .unwrap()
        .is_match(registry_index));

        let crate_archives = iter.next().unwrap();
        assert!(Regex::new(if cfg!(windows) {
            r"crate source archives:.*\\cargo_home_cargo_cache_paths_print\\registry\\cache"
        } else {
            r"crate source archives:.*/cargo_home_cargo_cache_paths_print/registry/cache"
        })
        .unwrap()
        .is_match(crate_archives));

        let crate_sources = iter.next().unwrap();
        assert!(Regex::new(if cfg!(windows) {
            r"unpacked crate sources:.*\\cargo_home_cargo_cache_paths_print\\registry\\src"
        } else {
            r"unpacked crate sources:.*/cargo_home_cargo_cache_paths_print/registry/src"
        })
        .unwrap()
        .is_match(crate_sources));

        let bare_repos = iter.next().unwrap();
        assert!(Regex::new(if cfg!(windows) {
            r"bare git repos:.*\\cargo_home_cargo_cache_paths_print\\git\\db"
        } else {
            r"bare git repos:.*/cargo_home_cargo_cache_paths_print/git/db"
        })
        .unwrap()
        .is_match(bare_repos));

        let git_repo_checkouts = iter.next().unwrap();
        assert!(Regex::new(if cfg!(windows) {
            r"git repo checkouts.*\\cargo_home_cargo_cache_paths_print\\git\\checkouts"
        } else {
            r"git repo checkouts.*/cargo_home_cargo_cache_paths_print/git/checkouts"
        })
        .unwrap()
        .is_match(git_repo_checkouts));

        // should be empty now
        let last = iter.next();
        assert!(last.is_none(), "found another directory?!: '{:?}'", last);
    }
}

#[cfg(all(test, feature = "bench"))]
mod benchmarks {
    use super::*;
    use crate::test::black_box;
    use crate::test::Bencher;
    use crate::test_helpers::assert_path_end;

    #[allow(non_snake_case)]
    #[bench]
    fn bench_CargoCachePaths_new(b: &mut Bencher) {
        // get cargo target dir
        let mut target_dir = std::env::current_dir().unwrap();
        target_dir.push("target");
        let mut cargo_home = target_dir;
        cargo_home.push("cargo_home_bench_new");
        //make sure this worked
        let CH_string = format!("{}", cargo_home.display());
        assert_path_end(
            &cargo_home,
            &["cargo-cache", "target", "cargo_home_bench_new"],
        );

        // create the directory
        if !std::path::PathBuf::from(&CH_string).is_dir() {
            std::fs::DirBuilder::new().create(&CH_string).unwrap();
        }
        assert!(fs::metadata(&CH_string).unwrap().is_dir());
        assert!(std::path::PathBuf::from(&CH_string).is_dir());

        #[allow(unused_must_use)]
        b.iter(|| {
            let x = CargoCachePaths::new(PathBuf::from(&CH_string));
            black_box(x);
        });
    }

    #[allow(non_snake_case)]
    #[bench]
    fn bench_CargoCachePaths_print(b: &mut Bencher) {
        // get cargo target dir
        let mut target_dir = std::env::current_dir().unwrap();
        target_dir.push("target");
        let mut cargo_home = target_dir;
        cargo_home.push("cargo_home_bench_print");
        //make sure this worked
        let CH_string = format!("{}", cargo_home.display());
        assert_path_end(
            &cargo_home,
            &["cargo-cache", "target", "cargo_home_bench_print"],
        );

        // create the directory
        if !std::path::PathBuf::from(&CH_string).is_dir() {
            std::fs::DirBuilder::new().create(&CH_string).unwrap();
        }
        assert!(fs::metadata(&CH_string).unwrap().is_dir());
        assert!(std::path::PathBuf::from(&CH_string).is_dir());

        let ccp = CargoCachePaths::new(PathBuf::from(CH_string)).unwrap();
        #[allow(unused_must_use)]
        b.iter(|| {
            let x = ccp.to_string();
            let _ = black_box(x);
        });
    }
}
