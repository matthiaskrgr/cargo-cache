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
    let walkdir_start = dir.display().to_string();

    let dir_size = WalkDir::new(&walkdir_start)
        .into_iter()
        .map(|e| e.unwrap().path().to_owned())
        .filter(|f| f.exists()) // avoid broken symlinks
        .collect::<Vec<_>>() // @TODO perhaps WalkDir will impl ParallelIterator one day
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

pub(crate) fn pad_strings(
    indent_lvl: u16,
    max_line_width: u16,
    beginning: &str,
    end: &str,
) -> String {
    let left: u16 = max_line_width + (indent_lvl * 2);
    #[allow(clippy::cast_possible_truncation)] // very unlikely
    let right: u16 = beginning.len() as u16;
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

    #[allow(non_snake_case)]
    #[test]
    fn test_pad_strings() {
        let s1 = pad_strings(1, 4, "hello", "world");
        assert_eq!(s1, "hello world\n");

        let s2 = pad_strings(2, 4, "hello", "world");
        assert_eq!(s2, "hello   world\n");

        let s3 = pad_strings(1, 6, "hello", "world");
        assert_eq!(s3, "hello   world\n");

        let s4 = pad_strings(1, 7, "hello", "world");
        assert_eq!(s4, "hello    world\n");

        let s5 = pad_strings(2, 6, "hello", "world");
        assert_eq!(s5, "hello     world\n");

        let s6 = pad_strings(2, 10, "hello", "world");
        assert_eq!(s6, "hello         world\n");
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
