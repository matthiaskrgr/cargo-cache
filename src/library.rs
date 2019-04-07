// Copyright 2017-2019 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;
use std::fs;
use std::path::PathBuf;

use crate::dirsizes::*;

use humansize::{file_size_opts, FileSize};
use rayon::iter::*;
use walkdir::WalkDir;

#[derive(Debug)]
pub(crate) struct DirInfo {
    // make sure we do not accidentally confuse dir_size and file_number
    // since both are of the same type
    pub(crate) dir_size: u64,
    pub(crate) file_number: u64,
}

#[derive(Debug)]
pub(crate) struct CargoCachePaths {
    pub(crate) cargo_home: PathBuf,
    pub(crate) bin_dir: PathBuf,
    pub(crate) registry: PathBuf,
    pub(crate) registry_pkg_cache: PathBuf,
    pub(crate) registry_sources: PathBuf,
    pub(crate) registry_index: PathBuf,
    pub(crate) git_repos_bare: PathBuf,
    pub(crate) git_checkouts: PathBuf,
}

#[derive(Debug)]
pub(crate) enum ErrorKind {
    GitRepoNotOpened,
    GitRepoDirNotFound,
    GitGCFailed,
    GitPackRefsFailed,
    GitReflogFailed,
    MalformedPackageName,
    CargoFailedGetConfig,
    CargoHomeNotDirectory,
    InvalidDeletableDir,
    RemoveDirNoArg,
}

impl CargoCachePaths {
    // holds the PathBufs to the different components of the cargo cache
    pub(crate) fn default() -> Result<Self, (ErrorKind, String)> {
        let cargo_cfg = if let Ok(cargo_cfg) = cargo::util::config::Config::default() {
            cargo_cfg
        } else {
            return Err((
                ErrorKind::CargoFailedGetConfig,
                "Failed to get cargo config!".to_string(),
            ));
        };

        let cargo_home_path = cargo_cfg.home().clone().into_path_unlocked();

        if !cargo_home_path.is_dir() {
            let msg = format!(
                "Error, no cargo home path directory '{}' found.",
                cargo_home_path.display()
            );
            return Err((ErrorKind::CargoHomeNotDirectory, msg));
        }
        // get the paths to the relevant directories
        let cargo_home = cargo_home_path;
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

pub(crate) fn cumulative_dir_size(dir: &PathBuf) -> DirInfo {
    // Note: using a hashmap to cache dirsizes does apparently not pay out performance-wise
    if !dir.is_dir() {
        return DirInfo {
            dir_size: 0,
            file_number: 0,
        };
    }

    // traverse recursively and sum filesizes, parallelized by rayon

    // @TODO I would like to get rid of the vector here but not sure how to convert
    // WalkDir iterator into rayon par_iter

    let walkdir_start = dir.display().to_string();

    let dir_size = WalkDir::new(&walkdir_start)
        .into_iter()
        .map(|e| e.unwrap().path().to_owned())
        .filter(|f| f.exists()) // avoid broken symlinks
        .collect::<Vec<_>>()
        .par_iter()
        .map(|f| {
            fs::metadata(f)
                .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &dir.display()))
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

pub(crate) fn rm_old_crates(
    amount_to_keep: u64,
    dry_run: bool,
    registry_src_path: &PathBuf,
    size_changed: &mut bool,
) -> Result<(), (ErrorKind, PathBuf)> {
    println!();

    // remove crate sources from cache
    // src can be completely removed since we can always rebuilt it from cache (by extracting packages)
    let mut removed_size = 0;
    // walk registry repos
    for repo in fs::read_dir(&registry_src_path).unwrap() {
        let mut crate_list = fs::read_dir(&repo.unwrap().path())
            .unwrap()
            .map(|cratepath| cratepath.unwrap().path())
            .collect::<Vec<PathBuf>>();
        crate_list.sort();
        crate_list.reverse();

        let mut versions_of_this_package = 0;
        let mut last_pkgname = String::new();
        // iterate over all crates and extract name and version
        for pkgpath in &crate_list {
            let path_end = match pkgpath.iter().last() {
                Some(path_end) => path_end,
                None => return Err((ErrorKind::MalformedPackageName, (pkgpath.to_owned()))),
            };

            let mut vec = path_end.to_str().unwrap().split('-').collect::<Vec<&str>>();
            let pkgver = match vec.pop() {
                Some(pkgver) => pkgver,
                None => return Err((ErrorKind::MalformedPackageName, (pkgpath.to_owned()))),
            };
            let pkgname = vec.join("-");

            if amount_to_keep == 0 {
                removed_size += fs::metadata(pkgpath)
                    .unwrap_or_else(|_| {
                        panic!("Failed to get metadata of file '{}'", &pkgpath.display())
                    })
                    .len();

                let dryrun_msg = format!(
                    "dry run: not actually deleting {} {} at {}",
                    pkgname,
                    pkgver,
                    pkgpath.display()
                );
                remove_file(&pkgpath, dry_run, size_changed, None, Some(dryrun_msg));

                continue;
            }
            // println!("pkgname: {:?}, pkgver: {:?}", pkgname, pkgver);

            if last_pkgname == pkgname {
                versions_of_this_package += 1;
                if versions_of_this_package == amount_to_keep {
                    // we have seen this package too many times, queue for deletion
                    removed_size += fs::metadata(pkgpath)
                        .unwrap_or_else(|_| {
                            panic!("Failed to get metadata of file '{}'", &pkgpath.display())
                        })
                        .len();

                    let dryrun_msg = format!(
                        "dry run: not actually deleting {} {} at {}",
                        pkgname,
                        pkgver,
                        pkgpath.display()
                    );
                    remove_file(&pkgpath, dry_run, size_changed, None, Some(dryrun_msg));
                }
            } else {
                // last_pkgname != pkgname, we got to a new package, reset counter
                versions_of_this_package = 0;
                last_pkgname = pkgname;
            } // if last_pkgname == pkgname
        } // for pkgpath in &crate_list
    }
    println!(
        "Removed {} of compressed crate sources.",
        removed_size.file_size(file_size_opts::DECIMAL).unwrap()
    );
    Ok(())
}

pub(crate) fn get_info(c: &CargoCachePaths, s: &DirSizes<'_>) -> String {
    let mut strn = String::with_capacity(1020);
    strn.push_str("Found CARGO_HOME / cargo cache base dir\n");
    strn.push_str(&format!(
        "\t\t\t'{}' of size: {}\n",
        &c.cargo_home.display(),
        s.total_size.file_size(file_size_opts::DECIMAL).unwrap()
    ));

    strn.push_str(&format!("Found {} binaries installed in\n", s.numb_bins));
    strn.push_str(&format!(
        "\t\t\t'{}', size: {}\n",
        &c.bin_dir.display(),
        s.total_bin_size.file_size(file_size_opts::DECIMAL).unwrap()
    ));
    strn.push_str("\t\t\tNote: use 'cargo uninstall' to remove binaries, if needed.\n");

    strn.push_str("Found registry base dir:\n");
    strn.push_str(&format!(
        "\t\t\t'{}', size: {}\n",
        &c.registry.display(),
        s.total_reg_size.file_size(file_size_opts::DECIMAL).unwrap()
    ));
    strn.push_str("Found registry crate source cache:\n");
    strn.push_str(&format!(
        "\t\t\t'{}', size: {}\n",
        &c.registry_pkg_cache.display(),
        s.total_reg_cache_size
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    ));
    strn.push_str("\t\t\tNote: removed crate sources will be redownloaded if necessary\n");
    strn.push_str("Found registry unpacked sources\n");
    strn.push_str(&format!(
        "\t\t\t'{}', size: {}\n",
        &c.registry_sources.display(),
        s.total_reg_src_size
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    ));
    strn.push_str("\t\t\tNote: removed unpacked sources will be reextracted from local cache (no net access needed).\n");

    strn.push_str("Found git repo database:\n");
    strn.push_str(&format!(
        "\t\t\t'{}', size: {}\n",
        &c.git_repos_bare.display(),
        s.total_git_repos_bare_size
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    ));
    strn.push_str("\t\t\tNote: removed git repositories will be recloned if necessary\n");
    strn.push_str("Found git repo checkouts:\n");
    strn.push_str(&format!(
        "\t\t\t'{}', size: {}\n",
        &c.git_checkouts.display(),
        s.total_git_chk_size
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    ));
    strn.push_str(
        "\t\t\tNote: removed git checkouts will be rechecked-out from repo database if necessary (no net access needed, if repos are up-to-date).\n"
    );
    strn
}

pub(crate) fn size_diff_format(size_before: u64, size_after: u64, dspl_sze_before: bool) -> String {
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
        if dspl_sze_before {
            format!(
                "{} => {}",
                size_before_human_readabel, size_after_human_readable
            )
        } else {
            size_after_human_readable
        }
    } else if dspl_sze_before {
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

pub(crate) fn remove_dir_via_cmdline(
    directory: Option<&str>,
    dry_run: bool,
    ccd: &CargoCachePaths,
    size_changed: &mut bool,
) -> Result<(), (ErrorKind, String)> {
    fn rm(
        dir: &PathBuf,
        dry_run: bool,
        size_changed: &mut bool,
    ) -> Result<(), (ErrorKind, String)> {
        // remove a specified subdirectory from cargo cache
        let msg = Some(format!("removing: '{}'", dir.display()));
        remove_file(&dir, dry_run, size_changed, msg, None);

        Ok(())
    }

    let input = if let Some(value) = directory {
        value
    } else {
        return Err((
            ErrorKind::RemoveDirNoArg,
            "No argument assigned to --remove-dir, example: 'git-repos,registry-sources'"
                .to_string(),
        ));
    };

    let inputs = input.split(',');
    let valid_dirs = vec![
        "git-db",
        "git-repos",
        "registry-sources",
        "registry-crate-cache",
        "registry",
        "all",
    ];

    // keep track of what we want to remove
    let mut rm_git_repos = false;
    let mut rm_git_checkouts = false;
    let mut rm_registry_sources = false;
    let mut rm_registry_crate_cache = false;

    // validate input
    let mut invalid_dirs = String::new();
    let mut terminate: bool = false;

    for word in inputs {
        if valid_dirs.contains(&word) {
            // dir is recognized
            // dedupe
            match word {
                "all" => {
                    rm_git_repos = true;
                    rm_git_checkouts = true;
                    rm_registry_sources = true;
                    rm_registry_crate_cache = true;
                    // we clean the entire cache anyway,
                    // no need to look further, break out of loop
                    break; // for word in inputs
                }
                "registry" | "registry-crate-cache" => {
                    rm_registry_sources = true;
                    rm_registry_crate_cache = true;
                }
                "registry-sources" => {
                    rm_registry_sources = true;
                }
                "git-repos" => {
                    rm_git_checkouts = true;
                }
                "git-db" => {
                    rm_git_repos = true;
                    rm_git_checkouts = true;
                }
                _ => unreachable!(),
            } // match *word
        } else {
            // collect all invalid dirs and print all of them as merged string later
            invalid_dirs.push_str(word);
            invalid_dirs.push_str(" ");
            terminate = true;
        }
    } // for word in inputs
    if terminate {
        // remove trailing whitespace
        let inv_dirs = invalid_dirs.trim();
        return Err((
            ErrorKind::InvalidDeletableDir,
            format!("Invalid deletable dir(s): {}", inv_dirs),
        ));
    }
    // finally delete
    if rm_git_checkouts {
        rm(&ccd.git_checkouts, dry_run, size_changed)?
    }
    if rm_git_repos {
        rm(&ccd.git_repos_bare, dry_run, size_changed)?
    }
    if rm_registry_sources {
        rm(&ccd.registry_sources, dry_run, size_changed)?
    }
    if rm_registry_crate_cache {
        rm(&ccd.registry_pkg_cache, dry_run, size_changed)?
    }
    Ok(())
}

pub(crate) fn remove_file(
    path: &PathBuf,
    dry_run: bool,
    size_changed: &mut bool,
    deletion_msg: Option<String>,
    dry_run_msg: Option<String>,
) {
    if dry_run {
        if let Some(dr_msg) = dry_run_msg {
            println!("{}", dr_msg)
        } else {
            println!("dry-run: would remove: '{}'", path.display());
        }
        return;
    }
    // print deletion message if we have one
    if let Some(msg) = deletion_msg {
        println!("{}", msg);
    }

    if path.is_file() && fs::remove_file(&path).is_err() {
        eprintln!("Warning: failed to remove file \"{}\".", path.display());
    } else {
        *size_changed = true;
    }

    if path.is_dir() && fs::remove_dir_all(&path).is_err() {
        eprintln!(
            "Warning: failed to recursively remove directory \"{}\".",
            path.display()
        );
    } else {
        *size_changed = true;
    }
}

pub(crate) fn pad_strings(indent_lvl: i64, beginning: &str, end: &str) -> String {
    // max line width
    const MAX_WIDTH: i64 = 40;

    let left = MAX_WIDTH + (indent_lvl * 2);
    let right = beginning.len() as i64;
    let len_padding = left - right;
    assert!(
        len_padding > 0,
        format!(
            "len_padding is negative: '{} - {} = {}' ",
            left, right, len_padding
        )
    );

    let mut formatted_line = beginning.to_string();

    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    // I tried mittigating via previous assert()
    formatted_line.push_str(&" ".repeat(len_padding as usize));
    formatted_line.push_str(end);
    formatted_line.push_str("\n");
    formatted_line
}

#[cfg(test)]
mod libtests {
    use super::*;

    use pretty_assertions::assert_eq;
    use regex::Regex;
    use std::env;

    use crate::test_helpers::assert_path_end;

    impl CargoCachePaths {
        pub(crate) fn new(dir: PathBuf) -> Result<Self, (ErrorKind, String)> {
            if !dir.is_dir() {
                let msg = format!(
                    "Error, no cargo home path directory '{}' found.",
                    dir.display()
                );
                return Err((ErrorKind::CargoHomeNotDirectory, msg));
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
        // @TODO take $CARGO_TARGET_DIR into account
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
            black_box(x);
        });
    }

}
