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

// https://github.com/LeopoldArkham/humansize
extern crate humansize; // convert bytes to whatever

// https://github.com/BurntSushi/walkdir
extern crate walkdir; // walk CARGO_DIR recursively

// https://github.com/kbknapp/clap-rs
extern crate clap; // cmdline arg parsing

// https://github.com/rust-lang/cargo
extern crate cargo; // obtain CARGO_DIR

// https://github.com/alexcrichton/git2-rs
extern crate git2; // compress git repos

use std::{fs, process};
use std::io::{stdout, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use humansize::{file_size_opts as options, FileSize};
use walkdir::WalkDir;

pub struct DirInfoObj {
    // make sure we do not accidentally confuse dir_size and file_number
    // since both are of the same type
    dir_size: u64,
    file_number: u64,
}

pub struct DirSizesCollector {
    pub total_size: u64,       // total size of cargo root dir
    numb_bins: u64,            // number of binaries foundq
    total_bin_size: u64,       // total size of binaries found
    total_reg_size: u64,       // registry size
    total_git_db_size: u64,    // git db size
    total_git_chk_size: u64,   // git checkout size
    total_reg_cache_size: u64, // registry cache size
    total_reg_src_size: u64,   // registry sources size
}

impl DirSizesCollector {
    pub fn new(ccd: &CargoCacheDirs) -> DirSizesCollector {
        let bindir = cumulative_dir_size(&ccd.bin_dir.string);

        DirSizesCollector {
            total_size: cumulative_dir_size(&ccd.cargo_home.string).dir_size,
            numb_bins: bindir.file_number,
            total_bin_size: bindir.dir_size,
            total_reg_size: cumulative_dir_size(&ccd.registry.string).dir_size,
            total_git_db_size: cumulative_dir_size(&ccd.git_db.string).dir_size,
            total_git_chk_size: cumulative_dir_size(&ccd.git_checkouts.string).dir_size,
            total_reg_cache_size: cumulative_dir_size(&ccd.registry_cache.string).dir_size,
            total_reg_src_size: cumulative_dir_size(&ccd.registry_sources.string).dir_size,
        }
    }
    pub fn print_pretty(&self, ccd: &CargoCacheDirs) {
        println!("Cargo cache '{}':\n", ccd.cargo_home.string);
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

pub struct DirCache {
    pub path: PathBuf,
    pub string: String,
}

impl DirCache {
    fn new(string: String, pathbuf: PathBuf) -> DirCache {
        DirCache {
            path: pathbuf,
            string: string,
        }
    }
}

pub struct CargoCacheDirs {
    pub cargo_home: DirCache,
    pub bin_dir: DirCache,
    pub registry: DirCache,
    pub registry_cache: DirCache,
    pub registry_sources: DirCache,
    pub git_db: DirCache,
    pub git_checkouts: DirCache,
}

pub enum ErrorKind {
    GitRepoNotOpened,
    GitRepoDirNotFound,
    GitGCFailed,
    MalformedPackageName,
}

impl Default for CargoCacheDirs {
    fn default() -> Self {
        Self::new()
    }
}

impl CargoCacheDirs {
    pub fn new() -> CargoCacheDirs {
        let cargo_cfg = cargo::util::config::Config::default()
            .expect("Failed to get cargo config/CARGO_HOME from cargo!");
        let cargo_home_str = format!("{}", cargo_cfg.home().display());
        let cargo_home_path = PathBuf::from(&cargo_home_str);
        let cargo_home_path_clone = cargo_home_path.clone();

        if !cargo_home_path.is_dir() {
            panic!(
                "Error, no cargo home path directory '{}' found.",
                &cargo_home_str
            );
        }

        let cargo_home = DirCache::new(cargo_home_str, cargo_home_path);
        // bin
        let bin_path = cargo_home.path.join("bin/");
        let bin_str = str_from_pb(&bin_path);
        let bin = DirCache::new(bin_str, bin_path);
        // registry
        let registry_dir_path = cargo_home.path.join("registry/");
        let registry_dir_str = str_from_pb(&registry_dir_path);
        let registry = DirCache::new(registry_dir_str, registry_dir_path);

        let registry_cache = registry.path.join("cache/");
        let registry_cache_str = str_from_pb(&registry_cache);
        let reg_cache = DirCache::new(registry_cache_str, registry_cache);

        let registry_sources = registry.path.join("src/");
        let registry_sources_str = str_from_pb(&registry_sources);
        let reg_src = DirCache::new(registry_sources_str, registry_sources);
        // git
        let git_db_path = cargo_home.path.join("git/db/");
        let git_db_str = str_from_pb(&git_db_path);
        let git_db = DirCache::new(git_db_str, git_db_path);

        let git_checkouts_path = cargo_home_path_clone.join("git/checkouts/");
        let git_checkouts_str = str_from_pb(&git_checkouts_path);
        let git_checkouts = DirCache::new(git_checkouts_str, git_checkouts_path);

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
        println!("binaries directory:           {}", self.bin_dir.string);
        println!("registry directory:           {}", self.registry.string);
        println!(
            "registry crate source cache:  {}",
            self.registry_cache.string
        );
        println!(
            "registry unpacked sources:    {}",
            self.registry_sources.string
        );
        println!("git db directory:             {}", self.git_db.string);
        println!(
            "git checkouts dir:            {}",
            self.git_checkouts.string
        );
    }
}

fn gc_repo(pathstr: &str, config: &clap::ArgMatches) -> Result<(u64, u64), (ErrorKind, String)> {
    let vec = pathstr.split('/').collect::<Vec<&str>>();
    let reponame = match vec.last() {
        Some(reponame) => reponame,
        None => "<unknown>",
    };
    print!("Recompressing '{}': ", reponame);
    let path = Path::new(pathstr);
    if !path.is_dir() {
        return Err((ErrorKind::GitRepoDirNotFound, pathstr.to_string()));
    }

    // get size before
    let repo_size_before = cumulative_dir_size(pathstr).dir_size;
    let sb_human_readable = repo_size_before.file_size(options::DECIMAL).unwrap();
    print!("{} => ", sb_human_readable);
    // we need to flush stdout manually for incremental print();
    match stdout().flush() {
        Ok(_ok) => {}
        Err(_e) => {} // ignore errors
    }
    if config.is_present("dry-run") {
        println!("{} ({}{})", sb_human_readable, "+", 0);
        Ok((0, 0))
    } else {
        let repo = match git2::Repository::open(path) {
            Ok(repo) => repo,
            Err(e) => return Err(((ErrorKind::GitRepoNotOpened), format!("{:?}", e))),
        };
        match Command::new("git")
            .arg("gc")
            .arg("--aggressive")
            .arg("--prune=now")
            .current_dir(repo.path())
            .output()
        {
            Ok(_out) => {}
            /* println!("git gc error\nstatus: {}", out.status);
            println!("stdout:\n {}", String::from_utf8_lossy(&out.stdout));
            println!("stderr:\n {}", String::from_utf8_lossy(&out.stderr));
            //if out.status.success() {}
            } */
            Err(e) => return Err((ErrorKind::GitGCFailed, format!("{:?}", e))),
        }
        let repo_size_after = cumulative_dir_size(pathstr).dir_size;
        println!(
            "{}",
            size_diff_format(repo_size_before, repo_size_after, false)
        );

        Ok((repo_size_before, repo_size_after))
    }
}

fn cumulative_dir_size(dir: &str) -> DirInfoObj {
    let dir_path = Path::new(dir);
    if !dir_path.is_dir() {
        return DirInfoObj {
            dir_size: 0,
            file_number: 0,
        };
    }
    // Note: using a hashmap to cache dirsizes does apparently not pay out performance-wise
    let mut cumulative_size = 0;
    let mut number_of_files = 0;
    // traverse recursively and sum filesizes
    for entry in WalkDir::new(dir) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            cumulative_size += fs::metadata(path)
                .expect(&format!("Failed to get metadata of file '{}'", &dir))
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
    registry_str: &str,
    size_changed: &mut bool,
) -> Result<bool, (ErrorKind, String)> {
    println!();

    // remove crate sources from cache
    // src can be completely nuked since we can always rebuilt it from cache
    let registry_src_path = Path::new(&registry_str);

    let mut removed_size = 0;
    // walk registry repos
    for repo in fs::read_dir(&registry_src_path).unwrap() {
        let mut crate_list = Vec::new();
        let string = repo.unwrap().path().into_os_string().into_string().unwrap();
        for cratesrc in fs::read_dir(&string).unwrap() {
            let cratestr = cratesrc
                .expect("failed to read directory")
                .path()
                .into_os_string()
                .into_string()
                .expect("failed to convert path to string");
            crate_list.push(cratestr.clone());
        }
        crate_list.sort();
        crate_list.reverse();

        let mut versions_of_this_package = 0;
        let mut last_pkgname = String::from("");
        // iterate over all crates and extract name and version
        for pkgpath in &crate_list {
            let string = match pkgpath.split('/').last() {
                Some(string) => string,
                None => return Err((ErrorKind::MalformedPackageName, (pkgpath.clone()))),
            };

            let mut vec = string.split('-').collect::<Vec<&str>>();
            let pkgver = match vec.pop() {
                Some(pkgver) => pkgver,
                None => return Err((ErrorKind::MalformedPackageName, (pkgpath.clone()))),
            };
            let pkgname = vec.join("-");

            if amount_to_keep == 0 {
                removed_size += fs::metadata(pkgpath)
                    .expect(&format!("Failed to get metadata of file '{}'", &pkgpath))
                    .len();
                if config.is_present("dry-run") {
                    println!(
                        "dry run: not actually deleting {} {} at {}",
                        pkgname, pkgver, pkgpath
                    );
                } else {
                    println!("deleting: {} {} at {}", pkgname, pkgver, pkgpath);
                    fs::remove_file(pkgpath)
                        .expect(&format!("Failed to remove file '{}'", pkgpath));
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
                        .expect(&format!("Failed to get metadata of file '{}'", &pkgpath))
                        .len();
                    if config.is_present("dry-run") {
                        println!(
                            "dry run: not actually deleting {} {} at {}",
                            pkgname, pkgver, pkgpath
                        );
                    } else {
                        println!("deleting: {} {} at {}", pkgname, pkgver, pkgpath);
                        fs::remove_file(pkgpath)
                            .expect(&format!("Failed to remove file '{}'", pkgpath));
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
    Ok(true)
}

pub fn print_info(c: &CargoCacheDirs, s: &DirSizesCollector) {
    println!("Found CARGO_HOME / cargo cache base dir");
    println!(
        "\t\t\t'{}' of size: {}",
        c.cargo_home.string,
        s.total_size.file_size(options::DECIMAL).unwrap()
    );

    println!("Found {} binaries installed in", s.numb_bins);
    println!(
        "\t\t\t'{}', size: {}",
        c.bin_dir.string,
        s.total_bin_size.file_size(options::DECIMAL).unwrap()
    );
    println!("\t\t\tNote: use 'cargo uninstall' to remove binaries, if needed.");

    println!("Found registry base dir:");
    println!(
        "\t\t\t'{}', size: {}",
        c.registry.string,
        s.total_reg_size.file_size(options::DECIMAL).unwrap()
    );
    println!("Found registry crate source cache:");
    println!(
        "\t\t\t'{}', size: {}",
        c.registry_cache.string,
        s.total_reg_cache_size.file_size(options::DECIMAL).unwrap()
    );
    println!("\t\t\tNote: removed crate sources will be redownloaded if neccessary");
    println!("Found registry unpacked sources");
    println!(
        "\t\t\t'{}', size: {}",
        c.registry_sources.string,
        s.total_reg_src_size.file_size(options::DECIMAL).unwrap()
    );
    println!("\t\t\tNote: removed unpacked sources will be reextracted from local cache (no net access needed).");

    println!("Found git repo database:");
    println!(
        "\t\t\t'{}', size: {}",
        c.git_db.string,
        s.total_git_db_size.file_size(options::DECIMAL).unwrap()
    );
    println!("\t\t\tNote: removed git repositories will be recloned if neccessary");
    println!("Found git repo checkouts:");
    println!(
        "\t\t\t'{}', size: {}",
        c.git_checkouts.string,
        s.total_git_chk_size.file_size(options::DECIMAL).unwrap()
    );
    println!(
        "\t\t\tNote: removed git checkouts will be rechecked-out from repo database if neccessary (no net access needed, if repos are up-to-date)."
    );
}

fn str_from_pb(path: &PathBuf) -> String {
    path.clone().into_os_string().into_string().unwrap()
}

pub fn run_gc(cargo_cache: &CargoCacheDirs, config: &clap::ArgMatches) {
    let git_db = &cargo_cache.git_db.path;
    // gc cloned git repos of crates or whatever
    if !git_db.is_dir() {
        println!("WARNING:   {} is not a dir", str_from_pb(git_db));
        return;
    }
    let mut total_size_before: u64 = 0;
    let mut total_size_after: u64 = 0;

    println!("\nRecompressing repositories. Please be patient...");
    // gc git repos of crates
    for entry in fs::read_dir(&git_db).unwrap() {
        let repo = entry.unwrap().path();
        let repostr = str_from_pb(&repo);
        let (before, after) = match gc_repo(&repostr, config) {
            // run gc
            Ok((before, after)) => (before, after),
            Err((errorkind, msg)) => match errorkind {
                ErrorKind::GitGCFailed => {
                    println!("Warning, git gc failed, skipping '{}'", repostr);
                    println!("git error: '{}'", msg);
                    continue;
                }
                ErrorKind::GitRepoDirNotFound => {
                    println!("Git repo not found: '{}'", msg);
                    continue;
                }
                ErrorKind::GitRepoNotOpened => {
                    println!("Failed to parse git repo: '{}'", msg);
                    continue;
                }
                _ => unreachable!(),
            },
        };
        total_size_before += before;
        total_size_after += after;
    }
    println!("Recompressing registries....");
    let mut repo_index = (&cargo_cache.registry_cache.path).clone();
    repo_index.pop();
    repo_index.push("index/");
    for repo in fs::read_dir(repo_index).unwrap() {
        let repo_str = str_from_pb(&repo.unwrap().path());
        let (before, after) = match gc_repo(&repo_str, config) {
            // run gc
            Ok((before, after)) => (before, after),
            Err((errorkind, msg)) => match errorkind {
                ErrorKind::GitGCFailed => {
                    println!("Warning, git gc failed, skipping '{}'", repo_str);
                    println!("git error: '{}'", msg);
                    continue;
                }
                ErrorKind::GitRepoDirNotFound => {
                    println!("Git repo not found: {}", msg);
                    continue;
                }
                ErrorKind::GitRepoNotOpened => {
                    println!("Failed to parse git repo: '{}'", msg);
                    continue;
                }
                _ => unreachable!(),
            },
        };

        total_size_before += before;
        total_size_after += after;
    } // iterate over registries and gc

    println!(
        "Compressed {} to {}",
        total_size_before.file_size(options::DECIMAL).unwrap(),
        size_diff_format(total_size_before, total_size_after, false)
    );
}

pub fn size_diff_format(size_before: u64, size_after: u64, dspl_sze_before: bool) -> String {
    let size_after_signed = size_after as i64;
    let size_before_signed = size_before as i64;
    let size_diff: i64 = size_after_signed - size_before_signed;

    // humansize does not work with negative numbers currently so we have to work around
    let sign = if size_diff < 0 { "-" } else { "+" };

    let size_after_human_readable = size_after.file_size(options::DECIMAL).unwrap();
    let size_diff_human_readable = size_diff.abs().file_size(options::DECIMAL).unwrap();
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
    if !config.is_present("remove-dir") {
        // do nothing if arg not used
        return;
    }

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
    let mut deduped_dirs = Vec::new();
    for elm in dirs_to_delete {
        if !deduped_dirs.contains(&elm) {
            deduped_dirs.push(elm);
        }
    }

    // translate enum to actual paths to be deleted
    let mut dirs = Vec::new();
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
        if config.is_present("dry-run") {
            println!("dry-run: would delete: '{}'", dir.string);
        } else if dir.path.is_dir() {
            println!("removing: '{}'", dir.string);
            fs::remove_dir_all(&dir.path).expect(&format!(
                "failed to remove dir '{}' as part of --remove-dir",
                dir.string
            ));
            *size_changed = true;
        } else {
            println!(
                "dir not existing or already removed; skipping: '{}'",
                dir.string
            );
        }
    }
}
