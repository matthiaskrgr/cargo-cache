// enable additional clippy warnings
#![cfg_attr(feature = "cargo-clippy", warn(int_plus_one))]
#![cfg_attr(feature = "cargo-clippy", warn(shadow_reuse))]
#![cfg_attr(feature = "cargo-clippy", warn(shadow_same))]
#![cfg_attr(feature = "cargo-clippy", warn(shadow_unrelated))]
#![cfg_attr(feature = "cargo-clippy", warn(mut_mut))]
#![cfg_attr(feature = "cargo-clippy", warn(nonminimal_bool))]
#![cfg_attr(feature = "cargo-clippy", warn(pub_enum_variant_names))]
#![cfg_attr(feature = "cargo-clippy", warn(range_plus_one))]
#![cfg_attr(feature = "cargo-clippy", warn(string_add))]
#![cfg_attr(feature = "cargo-clippy", warn(string_add_assign))]
#![cfg_attr(feature = "cargo-clippy", warn(stutter))]
//#![cfg_attr(feature = "cargo-clippy", warn(result_unwrap_used))]

extern crate cargo;
extern crate git2;
extern crate humansize;
extern crate walkdir;

use std::fs;
use std::path::PathBuf;

use humansize::{file_size_opts as options, FileSize};
use walkdir::WalkDir;

pub struct DirInfoObj {
    // make sure we do not accidentally confuse dir_size and file_number
    // since both are of the same type
    pub dir_size: u64,
    pub file_number: u64,
}

pub struct DirSizesCollector {
    pub total_size: u64,       // total size of cargo root dir
    numb_bins: u64,            // number of binaries found
    total_bin_size: u64,       // total size of binaries found
    total_reg_size: u64,       // registry size
    total_git_db_size: u64,    // git db size
    total_git_chk_size: u64,   // git checkout size
    total_reg_cache_size: u64, // registry cache size
    total_reg_src_size: u64,   // registry sources size
}

impl DirSizesCollector {
    pub fn new(ccd: &CargoCacheDirs) -> DirSizesCollector {
        let bindir = cumulative_dir_size(&ccd.bin_dir);

        DirSizesCollector {
            total_size: cumulative_dir_size(&ccd.cargo_home).dir_size,
            numb_bins: bindir.file_number,
            total_bin_size: bindir.dir_size,
            total_reg_size: cumulative_dir_size(&ccd.registry).dir_size,
            total_git_db_size: cumulative_dir_size(&ccd.git_db).dir_size,
            total_git_chk_size: cumulative_dir_size(&ccd.git_checkouts).dir_size,
            total_reg_cache_size: cumulative_dir_size(&ccd.registry_cache).dir_size,
            total_reg_src_size: cumulative_dir_size(&ccd.registry_sources).dir_size,
        }
    }
    pub fn print_pretty(&self, ccd: &CargoCacheDirs) {
        println!("Cargo cache '{}':\n", &ccd.cargo_home.display());
        println!(
            "Total size:                   {} ",
            self.total_size.file_size(options::DECIMAL).unwrap()
        );
        println!(
            "Size of {} installed binaries:     {} ",
            self.numb_bins,
            self.total_bin_size.file_size(options::DECIMAL).unwrap()
        );
        println!(
            "Size of registry:                  {} ",
            self.total_reg_size.file_size(options::DECIMAL).unwrap()
        );
        println!(
            "Size of registry crate cache:           {} ",
            self.total_reg_cache_size
                .file_size(options::DECIMAL)
                .unwrap()
        );
        println!(
            "Size of registry source checkouts:      {} ",
            self.total_reg_src_size.file_size(options::DECIMAL).unwrap()
        );
        println!(
            "Size of git db:                    {} ",
            self.total_git_db_size.file_size(options::DECIMAL).unwrap()
        );
        println!(
            "Size of git repo checkouts:        {} ",
            self.total_git_chk_size.file_size(options::DECIMAL).unwrap()
        );
    }
}

pub struct CargoCacheDirs {
    pub cargo_home: PathBuf,
    pub bin_dir: PathBuf,
    pub registry: PathBuf,
    pub registry_cache: PathBuf,
    pub registry_sources: PathBuf,
    pub git_db: PathBuf,
    pub git_checkouts: PathBuf,
}

pub enum ErrorKind {
    GitRepoNotOpened,
    GitRepoDirNotFound,
    GitGCFailed,
    GitPackRefsFailed,
    GitReflogFailed,
    MalformedPackageName,
    CargoFailedGetConfig,
    CargoHomeNotDirectory,
    InvalidDeletableDir,
    #[allow(non_camel_case_types)]
    rmFailed,
    #[allow(non_camel_case_types)]
    rmDirNoArg,
}

impl CargoCacheDirs {
    pub fn new() -> Result<CargoCacheDirs, (ErrorKind, String)> {
        let cargo_cfg = match cargo::util::config::Config::default() {
            Ok(cargo_cfg) => cargo_cfg,
            Err(_) => {
                return Err((ErrorKind::CargoFailedGetConfig, "Failed to get cargo config!".to_string()))
            }
        };

        let cargo_home_path = cargo_cfg.home().clone().into_path_unlocked();
        let cargo_home_str = format!("{}", cargo_home_path.display());
        let cargo_home_path_clone = cargo_home_path.clone();

        if !cargo_home_path.is_dir() {
            let msg = format!("Error, no cargo home path directory '{}' found.", &cargo_home_str);
            return Err((ErrorKind::CargoHomeNotDirectory, msg));
        }
        // get the paths to the relevant directories
        let cargo_home = cargo_home_path;
        let bin = cargo_home.join("bin/");
        let registry = cargo_home.join("registry/");
        let reg_cache = registry.join("cache/");
        let reg_src = registry.join("src/");
        let git_db = cargo_home.join("git/db/");
        let git_checkouts = cargo_home_path_clone.join("git/checkouts/");

        Ok(CargoCacheDirs {
            cargo_home: cargo_home,
            bin_dir: bin,
            registry: registry,
            registry_cache: reg_cache,
            registry_sources: reg_src,
            git_db: git_db,
            git_checkouts: git_checkouts,
        })
    }

    pub fn print_dir_paths(&self) {
        println!();
        println!("binaries directory:           {}", &self.bin_dir.display());
        println!("registry directory:           {}", &self.registry.display());
        println!(
            "registry crate source cache:  {}",
            &self.registry_cache.display()
        );
        println!(
            "registry unpacked sources:    {}",
            &self.registry_sources.display()
        );
        println!("git db directory:             {}", &self.git_db.display());
        println!(
            "git checkouts dir:            {}",
            &self.git_checkouts.display()
        );
    }
}

pub fn cumulative_dir_size(dir: &PathBuf) -> DirInfoObj {
    if !dir.is_dir() {
        return DirInfoObj {
            dir_size: 0,
            file_number: 0,
        };
    }
    // Note: using a hashmap to cache dirsizes does apparently not pay out performance-wise
    let mut cumulative_size = 0;
    let mut number_of_files = 0;
    // traverse recursively and sum filesizes
    for entry in WalkDir::new(format!("{}", dir.display())) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            cumulative_size += fs::metadata(path)
                .expect(&format!(
                    "Failed to get metadata of file '{}'",
                    &dir.display()
                ))
                .len();
            number_of_files += 1;
        }
    } // walkdir

    DirInfoObj {
        dir_size: cumulative_size,
        file_number: number_of_files,
    }
}

pub fn rm_old_crates(
    amount_to_keep: u64,
    dry_run: bool,
    registry_src_path: &PathBuf,
    size_changed: &mut bool,
) -> Result<(), (ErrorKind, PathBuf)> {
    println!();

    // remove crate sources from cache
    // src can be completely nuked since we can always rebuilt it from cache
    let mut removed_size = 0;
    // walk registry repos
    for repo in fs::read_dir(&registry_src_path).unwrap() {
        let mut crate_list = Vec::new();
        let string = format!("{}", &repo.unwrap().path().display());
        for cratepath in fs::read_dir(&string).unwrap() {
            crate_list.push(cratepath.expect("failed to read directory").path());
        }
        crate_list.sort();
        crate_list.reverse();

        let mut versions_of_this_package = 0;
        let mut last_pkgname = String::from("");
        // iterate over all crates and extract name and version
        for pkgpath in &crate_list {
            let path_end = match pkgpath.into_iter().last() {
                Some(path_end) => path_end,
                None => return Err((ErrorKind::MalformedPackageName, (pkgpath.clone()))),
            };

            let mut vec = path_end.to_str().unwrap().split('-').collect::<Vec<&str>>();
            let pkgver = match vec.pop() {
                Some(pkgver) => pkgver,
                None => return Err((ErrorKind::MalformedPackageName, (pkgpath.clone()))),
            };
            let pkgname = vec.join("-");

            if amount_to_keep == 0 {
                removed_size += fs::metadata(pkgpath)
                    .expect(&format!(
                        "Failed to get metadata of file '{}'",
                        &pkgpath.display()
                    ))
                    .len();
                if dry_run {
                    println!(
                        "dry run: not actually deleting {} {} at {}",
                        pkgname,
                        pkgver,
                        pkgpath.display()
                    );
                } else {
                    println!("deleting: {} {} at {}", pkgname, pkgver, pkgpath.display());
                    fs::remove_file(pkgpath)
                        .expect(&format!("Failed to remove file '{}'", pkgpath.display()));
                    *size_changed = true;
                }
                continue;
            }
            //println!("pkgname: {:?}, pkgver: {:?}", pkgname, pkgver);

            if last_pkgname == pkgname {
                versions_of_this_package += 1;
                if versions_of_this_package == amount_to_keep {
                    // we have seen this package too many times, queue for deletion
                    removed_size += fs::metadata(pkgpath)
                        .expect(&format!(
                            "Failed to get metadata of file '{}'",
                            &pkgpath.display()
                        ))
                        .len();
                    if dry_run {
                        println!(
                            "dry run: not actually deleting {} {} at {}",
                            pkgname,
                            pkgver,
                            pkgpath.display()
                        );
                    } else {
                        println!("deleting: {} {} at {}", pkgname, pkgver, pkgpath.display());
                        fs::remove_file(pkgpath)
                            .expect(&format!("Failed to remove file '{}'", pkgpath.display()));
                        *size_changed = true;
                    }
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
        removed_size.file_size(options::DECIMAL).unwrap()
    );
    Ok(())
}

pub fn print_info(c: &CargoCacheDirs, s: &DirSizesCollector) {
    println!("Found CARGO_HOME / cargo cache base dir");
    println!(
        "\t\t\t'{}' of size: {}",
        &c.cargo_home.display(),
        s.total_size.file_size(options::DECIMAL).unwrap()
    );

    println!("Found {} binaries installed in", s.numb_bins);
    println!(
        "\t\t\t'{}', size: {}",
        &c.bin_dir.display(),
        s.total_bin_size.file_size(options::DECIMAL).unwrap()
    );
    println!("\t\t\tNote: use 'cargo uninstall' to remove binaries, if needed.");

    println!("Found registry base dir:");
    println!(
        "\t\t\t'{}', size: {}",
        &c.registry.display(),
        s.total_reg_size.file_size(options::DECIMAL).unwrap()
    );
    println!("Found registry crate source cache:");
    println!(
        "\t\t\t'{}', size: {}",
        &c.registry_cache.display(),
        s.total_reg_cache_size.file_size(options::DECIMAL).unwrap()
    );
    println!("\t\t\tNote: removed crate sources will be redownloaded if neccessary");
    println!("Found registry unpacked sources");
    println!(
        "\t\t\t'{}', size: {}",
        &c.registry_sources.display(),
        s.total_reg_src_size.file_size(options::DECIMAL).unwrap()
    );
    println!("\t\t\tNote: removed unpacked sources will be reextracted from local cache (no net access needed).");

    println!("Found git repo database:");
    println!(
        "\t\t\t'{}', size: {}",
        &c.git_db.display(),
        s.total_git_db_size.file_size(options::DECIMAL).unwrap()
    );
    println!("\t\t\tNote: removed git repositories will be recloned if neccessary");
    println!("Found git repo checkouts:");
    println!(
        "\t\t\t'{}', size: {}",
        &c.git_checkouts.display(),
        s.total_git_chk_size.file_size(options::DECIMAL).unwrap()
    );
    println!(
        "\t\t\tNote: removed git checkouts will be rechecked-out from repo database if neccessary (no net access needed, if repos are up-to-date)."
    );
}

pub fn size_diff_format(size_before: u64, size_after: u64, dspl_sze_before: bool) -> String {
    let size_diff: i64 = size_after as i64 - size_before as i64;
    let sign = if size_diff > 0 { "+" } else { "" };
    let size_after_human_readable = size_after.file_size(options::DECIMAL).unwrap();
    let humansize_opts = options::FileSizeOpts {
        allow_negative: true,
        ..options::DECIMAL
    };
    let size_diff_human_readable = size_diff.file_size(humansize_opts).unwrap();
    let size_before_human_readabel = size_before.file_size(options::DECIMAL).unwrap();
    // percentage
    let percentage: f64 =
        ((size_after as f64 / size_before as f64) * f64::from(100)) - f64::from(100);
    // format
    let percentage = format!("{:.*}", 2, percentage);

    if size_before == size_after {
        if dspl_sze_before {
            format!(
                "{} => {}",
                size_before_human_readabel, size_after_human_readable
            )
        } else {
            format!("{}", size_after_human_readable)
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

pub fn remove_dir_via_cmdline(
    directory: Option<&str>,
    dry_run: bool,
    ccd: &CargoCacheDirs,
    size_changed: &mut bool,
) -> Result<(),(ErrorKind, String)> {

    fn rm(dir: &PathBuf, dry_run: bool, size_changed: &mut bool) -> Result<(), (ErrorKind, String)> {
        // remove a specified subdirectory from cargo cache
        if !dir.is_dir() {
        } else if dry_run {
            println!("dry-run: would delete: '{}'", dir.display());
        } else {
            println!("removing: '{}'", dir.display());
            match fs::remove_dir_all(&dir) {
                Ok(_) => {},
                Err(_) => {
                        return Err((ErrorKind::rmFailed, format!("failed to remove directory {}", dir.display())))
                },
            }
            *size_changed = true;
        }
        Ok(())
    }

    let input = match directory {
        Some(value) => value,
        None => {
            return Err((ErrorKind::rmDirNoArg, "No argument assigned to --remove-dir, example: 'git-repos,registry-sources'".to_string()))
        }
    };

    let inputs = input.split(',').collect::<Vec<&str>>();
    let valid_dirs = vec![
        "git-db",
        "git-repos",
        "registry-sources",
        "registry-crate-cache",
        "registry",
        "all",
    ];
    // validate input

    #[derive(Clone, Debug, PartialEq)]
    struct DelDirs {
        git_repos: bool,
        git_checkouts: bool,
        registry_sources: bool,
        registry_crate_cache: bool,
    }
    let mut terminate: bool = false;
    let mut del_dirs = DelDirs {
        git_repos: false,
        git_checkouts: false,
        registry_sources: false,
        registry_crate_cache: false,
    };

    let mut invalid_dirs = "".to_string();
    for word in &inputs {
        if !valid_dirs.contains(word) {
            // collect all invalid dirs and print all of them as merged string later
            invalid_dirs = format!("{} {}", invalid_dirs.to_string(), word.to_string()).to_string();
            terminate = true;
        } else {
            // dir is recognized
            // dedupe
            match *word {
                "all" => {
                    del_dirs.git_repos = true;
                    del_dirs.git_checkouts = true;
                    del_dirs.registry_sources = true;
                    del_dirs.registry_crate_cache = true;
                    // we rm everything, no need to look further, break out of loop
                    break; // for word in &inputs
                }
                "registry" | "registry-crate-cache" => {
                    del_dirs.registry_sources = true;
                    del_dirs.registry_crate_cache = true;
                }
                "registry-sources" => {
                    del_dirs.registry_sources = true;
                }
                "git-repos" => {
                    del_dirs.git_checkouts = true;
                }
                "git-db" => {
                    del_dirs.git_repos = true;
                    del_dirs.git_checkouts = true;
                }
                _ => unreachable!(),
            } // match *word
        }
    } // for word in &inputs
    if terminate {
        return Err((ErrorKind::InvalidDeletableDir, format!("Invalid deletable dirs: {}", invalid_dirs)))
    }
    // finally delete
    if del_dirs.git_checkouts {
        match rm(&ccd.git_checkouts, dry_run, size_changed) {
            Ok(_) => {},
            Err(e) => return Err(e),
        }
    }
    if del_dirs.git_repos {
        match rm(&ccd.git_db, dry_run, size_changed) {
            Ok(_) => {},
            Err(e) => return Err(e),
        }
    }
    if del_dirs.registry_sources {
        match rm(&ccd.registry_sources, dry_run, size_changed) {
            Ok(_) => {},
            Err(e) => return Err(e),
        }
    }
    if del_dirs.registry_crate_cache {
        match rm(&ccd.registry_cache, dry_run, size_changed) {
            Ok(_) => {},
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
