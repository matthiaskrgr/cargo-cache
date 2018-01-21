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
extern crate clap;
extern crate git2;
extern crate humansize;
extern crate walkdir;

use std::{fs, process};
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
}

impl Default for CargoCacheDirs {
    fn default() -> Self {
        Self::new()
    }
}

impl CargoCacheDirs {
    pub fn new() -> CargoCacheDirs {
        let cargo_cfg = match cargo::util::config::Config::default() {
            Ok(cargo_cfg) => cargo_cfg,
            Err(_) => {
                println!("Error: failed to get cargo config!");
                process::exit(1)
            }
        };

        let cargo_home_str = format!("{}", cargo_cfg.home().display());
        let cargo_home_path = PathBuf::from(&cargo_home_str);
        let cargo_home_path_clone = cargo_home_path.clone();

        if !cargo_home_path.is_dir() {
            panic!(
                "Error, no cargo home path directory '{}' found.",
                &cargo_home_str
            );
        }
        // get the paths to the relevant directories
        let cargo_home = cargo_home_path;
        let bin = cargo_home.join("bin/");
        let registry = cargo_home.join("registry/");
        let reg_cache = registry.join("cache/");
        let reg_src = registry.join("src/");
        let git_db = cargo_home.join("git/db/");
        let git_checkouts = cargo_home_path_clone.join("git/checkouts/");

        CargoCacheDirs {
            cargo_home: cargo_home,
            bin_dir: bin,
            registry: registry,
            registry_cache: reg_cache,
            registry_sources: reg_src,
            git_db: git_db,
            git_checkouts: git_checkouts,
        }
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
    for entry in WalkDir::new(str_from_pb(dir)) {
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
    config: &clap::ArgMatches,
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
        let string = str_from_pb(&repo.unwrap().path());
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
                if config.is_present("dry-run") {
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
                    if config.is_present("dry-run") {
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

pub fn str_from_pb(path: &PathBuf) -> String {
    path.clone().into_os_string().into_string().unwrap()
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
    config: &clap::ArgMatches,
    ccd: &CargoCacheDirs,
    size_changed: &mut bool,
) {
    let input = match config.value_of("remove-dir") {
        Some(value) => value,
        None => {
            println!("No argument assigned to --remove-dir, example: 'git-repos,registry-sources'");
            process::exit(2)
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
    enum DelDir {
        GitRepos,
        GitCheckouts,
        RegistrySources,
        RegistryCrateCache,
    }
    let mut dirs_to_delete = Vec::new();
    let mut terminate: bool = false;
    for word in &inputs {
        if !valid_dirs.contains(word) {
            println!("Error: invalid deletable dir: '{}'.", word);
            terminate = true;
        } else {
            // dir is recognized, translate into enum
            match *word {
                "all" => {
                    dirs_to_delete.push(DelDir::GitRepos);
                    dirs_to_delete.push(DelDir::GitCheckouts);
                    dirs_to_delete.push(DelDir::RegistrySources);
                    dirs_to_delete.push(DelDir::RegistryCrateCache);
                }
                "registry" | "registry-crate-cache" => {
                    dirs_to_delete.push(DelDir::RegistrySources);
                    dirs_to_delete.push(DelDir::RegistryCrateCache);
                }
                "registry-sources" => {
                    dirs_to_delete.push(DelDir::RegistrySources);
                }
                "git-repos" => {
                    dirs_to_delete.push(DelDir::GitCheckouts);
                }
                "git-db" => {
                    dirs_to_delete.push(DelDir::GitRepos);
                    dirs_to_delete.push(DelDir::GitCheckouts);
                }
                _ => unreachable!(),
            }
        }
    }
    if terminate {
        // invalid deletable dir given
        process::exit(5);
    }

    // remove duplicates
    let mut deduped_dirs = Vec::with_capacity(dirs_to_delete.len());
    for elm in dirs_to_delete {
        if !deduped_dirs.contains(&elm) {
            deduped_dirs.push(elm);
        }
    }

    // translate enum to actual paths to be deleted
    let mut dirs = Vec::with_capacity(deduped_dirs.len());
    for dir in deduped_dirs {
        match dir {
            DelDir::GitCheckouts => {
                dirs.push(&ccd.git_checkouts);
            }
            DelDir::GitRepos => {
                dirs.push(&ccd.git_db);
            }
            DelDir::RegistrySources => {
                dirs.push(&ccd.registry_sources);
            }
            DelDir::RegistryCrateCache => {
                dirs.push(&ccd.registry_cache);
            }
        }
    }
    // finally delete
    for dir in dirs {
        let dirstr = dir.display();
        if config.is_present("dry-run") {
            println!("dry-run: would delete: '{}'", dirstr);
        } else if dir.is_dir() {
            println!("removing: '{}'", dirstr);
            fs::remove_dir_all(&dir).expect(&format!(
                "failed to remove dir '{}' as part of --remove-dir",
                dirstr
            ));
            *size_changed = true;
        } else {
            println!(
                "dir not existing or already removed; skipping: '{}'",
                dirstr
            );
        }
    }
}

#[test]
fn test_str_from_pb() {
    let string = String::from("/home/one/two/three");
    let path = PathBuf::from(&string);

    assert_eq!(str_from_pb(&path), string);
    assert_eq!(str_from_pb(&path), "/home/one/two/three");
}
