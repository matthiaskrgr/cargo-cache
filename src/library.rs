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
use std::fs;
use std::path::PathBuf;

use crate::dirsizes::*;

use humansize::{file_size_opts, FileSize};
use rayon::iter::*;
use walkdir::WalkDir;

/// `DirInfo` is used so to be able to easily differentiate between size and number of files of a directory
#[derive(Debug, Clone)]
pub(crate) struct DirInfo {
    // make sure we do not accidentally confuse dir_size and file_number
    // since both are of the same type
    /// size of a directory
    pub(crate) dir_size: u64,
    /// number of files of a directory
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
                "Failed to Cargo.toml manifest in {} or upwards.",
                dir.display()
            ),
            Self::QueryRegexFailedParsing(regex) => write!(
                f,
                "Failed to parse regular expression \"{}\"",
                regex.to_string()
            ),
            Self::GitGCFile(path) => write!(
                f,
                "Tried to \"git gc\" a file instead of a directory: \"{}\"",
                path.display()
            ),
            Self::LocalNoTargetDir(path) => write!(
                f, "error: \"local\" subcommand tried to read \"target\" directory that does not exist: \"{}\"",
                path.display()
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
pub(crate) enum RemovableDir {
    All,
    GitDB,
    GitRepos,
    RegistrySources,
    RegistryCrateCache,
    RegistryIndex,
    Registry,
}

impl std::str::FromStr for RemovableDir {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "all" => Ok(RemovableDir::All),
            "git-db" => Ok(RemovableDir::GitDB),
            "git-repos" => Ok(RemovableDir::GitRepos),
            "registry-sources" => Ok(RemovableDir::RegistrySources),
            "registry-crate-cache" => Ok(RemovableDir::RegistryCrateCache),
            "registry-index" => Ok(RemovableDir::RegistryIndex),
            "registry" => Ok(RemovableDir::Registry),
            other => Err(other.to_string()),
        }
    }
}

// these are actually the components of the cache
// we have to mape the RemovableDirs to the CacheComponents
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Component {
    GitDB,              // git/db
    GitRepos,           // git/checkouts
    RegistrySources,    // registry/src
    RegistryCrateCache, // registry/cache
    RegistryIndex,      // registry/index
}

/// get the total size and number of files of a directory
pub(crate) fn cumulative_dir_size(dir: &PathBuf) -> DirInfo {
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
        fs::read_dir(&dir).unwrap().count()
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
        strn.push_str(&format!(
            "${{CARGO_HOME}} env var set to '{}', using that!\n",
            cache_path
        ));
    } else {
        strn.push_str(&format!(
            "Default cache dir found: '{}', using that!\n",
            c.cargo_home.display()
        ));
    };

    strn.push_str("\n");

    strn.push_str(&format!(
        "Total cache size: {}\n\n",
        s.total_size().file_size(file_size_opts::DECIMAL).unwrap()
    ));

    strn.push_str(&c.bin_dir.display().to_string());
    strn.push_str("\n");
    strn.push_str(&format!(
        "\t{} binaries installed in binary directory, total size: {}\n",
        s.numb_bins(),
        s.total_bin_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    ));
    strn.push_str("\tThese are the binaries installed via 'cargo install'.\n");
    strn.push_str("\tUse 'cargo uninstall' to remove binaries if needed.\n");
    strn.push_str("\n");

    strn.push_str(&c.registry.display().to_string());
    strn.push_str("\n");
    strn.push_str(&format!(
        "\tRegistry root dir, size: {}\n",
        s.total_reg_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    ));
    strn.push_str("\tCrate registries are stored here.\n");
    strn.push_str("\n");

    strn.push_str(&c.registry_index.display().to_string());
    strn.push_str("\n");
    strn.push_str(&format!(
        "\tRegistry index, size: {}\n",
        s.total_reg_index_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    ));
    strn.push_str("\tA git repo holding information on what crates are available.\n");
    strn.push_str("\tWill be recloned as needed.\n");

    strn.push_str("\n");

    // source archives are extracted here, will be reextracted from the downloaded source if removed
    strn.push_str(&c.registry_pkg_cache.display().to_string());
    strn.push_str("\n");
    strn.push_str(&format!(
        "\tCrate source package archive, size: {}\n",
        s.total_reg_cache_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    ));

    strn.push_str("\tCrates source packages of the registries are downloaded into this folder.\n");
    strn.push_str("\tThey will be redownloaded as needed.\n");
    strn.push_str("\n");

    strn.push_str(&c.registry_sources.display().to_string());
    strn.push_str("\n");

    strn.push_str(&format!(
        "\tCrate sources, size: {}\n",
        s.total_reg_src_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    ));
    strn.push_str("\tSource archives are extracted into this dir.\n");
    strn.push_str("\tThey will be reextracted from the package archive as needed.\n");
    strn.push_str("\n");

    strn.push_str(&c.git_repos_bare.display().to_string());
    strn.push_str("\n");
    strn.push_str(&format!(
        "\tGit database, size: {}\n",
        s.total_git_repos_bare_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    ));
    strn.push_str("\tBare repos of git dependencies are stored here.\n");
    strn.push_str("\tRemoved git repositories will be recloned as needed.\n");
    strn.push_str("\n");

    strn.push_str(&c.git_checkouts.display().to_string());
    strn.push_str("\n");
    strn.push_str(&format!(
        "\tGit repo checkouts, size: {}\n",
        s.total_git_chk_size()
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    ));
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
        // get cargo target dir
        let mut target_dir = std::env::current_dir().unwrap();
        // note: this may fail if CARGO_TARGET_DIR is set
        target_dir.push("target");
        let mut cargo_home = target_dir;
        cargo_home.push("cargo_home_cargo_cache_paths");
        // make sure this worked
        let CH_string = format!("{}", cargo_home.display());
        assert_path_end(
            &cargo_home,
            &["cargo-cache", "target", "cargo_home_cargo_cache_paths"],
        );

        // create the directory
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
        let mut target_dir = std::env::current_dir().unwrap();
        target_dir.push("target");
        let mut cargo_home = target_dir;
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

        let cargo_home = iter.next().unwrap();

        assert!(
            Regex::new(if cfg!(windows) {
                r"cargo home:.*\\cargo_home_cargo_cache_paths_print"
            } else {
                r"cargo home:.*/cargo_home_cargo_cache_paths_print"
            })
            .unwrap()
            .is_match(cargo_home),
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
        assert!(!last.is_some(), "found another directory?!: '{:?}'", last);
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
