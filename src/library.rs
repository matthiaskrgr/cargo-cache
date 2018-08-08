use std::fs;
use std::path::PathBuf;

use humansize::{file_size_opts, FileSize};
use rayon::iter::*;
use walkdir::WalkDir;

pub(crate) struct DirInfoObj {
    // make sure we do not accidentally confuse dir_size and file_number
    // since both are of the same type
    pub(crate) dir_size: u64,
    pub(crate) file_number: u64,
}

pub(crate) struct DirSizesCollector {
    pub(crate) total_size: u64,     // total size of cargo root dir
    numb_bins: u64,                 // number of binaries found
    total_bin_size: u64,            // total size of binaries found
    total_reg_size: u64,            // registry size
    total_git_repos_bare_size: u64, // git db size
    numb_git_repos_bare_repos: u64, // number of cloned repos
    numb_git_checkouts: u64,        // number of checked out repos
    total_git_chk_size: u64,        // git checkout size
    total_reg_cache_size: u64,      // registry cache size
    total_reg_src_size: u64,        // registry sources size
    numb_reg_cache_entries: u64,    // number of source archives
    numb_reg_src_checkouts: u64,    // number of source checkouts
}

impl DirSizesCollector {
    pub(crate) fn new(ccd: &CargoCacheDirs) -> Self {
        let bindir = cumulative_dir_size(&ccd.bin_dir);
        let git_repos_bare = cumulative_dir_size(&ccd.git_repos_bare);
        let git_checkouts = cumulative_dir_size(&ccd.git_checkouts);
        let reg_cache = cumulative_dir_size(&ccd.registry_cache);
        let reg_src = cumulative_dir_size(&ccd.registry_sources);

        Self {
            total_size: cumulative_dir_size(&ccd.cargo_home).dir_size,
            numb_bins: bindir.file_number,
            total_bin_size: bindir.dir_size,
            total_reg_size: cumulative_dir_size(&ccd.registry).dir_size,

            total_git_repos_bare_size: git_repos_bare.dir_size,
            numb_git_repos_bare_repos: git_repos_bare.file_number,

            total_git_chk_size: git_checkouts.dir_size,
            numb_git_checkouts: git_checkouts.file_number,

            total_reg_cache_size: reg_cache.dir_size,
            numb_reg_cache_entries: reg_cache.file_number,

            total_reg_src_size: reg_src.dir_size,
            numb_reg_src_checkouts: reg_src.file_number,
        }
    }
    pub(crate) fn print_pretty(&self, ccd: &CargoCacheDirs) {
        // create a string and concatenate all the things we want to print with it
        // and only print it in the end, this should save a few syscalls and be faster than
        // printing every line one by one
        let mut s = String::new();

        s.push_str(&format!("Cargo cache '{}':\n\n", &ccd.cargo_home.display()));

        s.push_str(&format!(
            "Total size: {: >35}\n",
            self.total_size.file_size(file_size_opts::DECIMAL).unwrap()
        ));

        // the nested format!()s are a hack to get nice alignment of the numbers
        // any ideas on how to not uses nested format here is appreciate...
        s.push_str(&format!(
            "{: <41} {}\n",
            &format!("Size of {} installed binaries:", self.numb_bins,),
            self.total_bin_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap()
        ));

        s.push_str(&format!(
            "Size of registry: {: >33}\n",
            self.total_reg_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap()
        ));

        s.push_str(&format!(
            "{: <44}{}\n",
            &format!("Size of {} crate archives:", self.numb_reg_cache_entries),
            self.total_reg_cache_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap()
        ));

        s.push_str(&format!(
            "{: <43} {}\n",
            &format!(
                "Size of {} crate source checkouts:",
                self.numb_reg_src_checkouts
            ),
            self.total_reg_src_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap()
        ));

        s.push_str(&format!(
            "{: <41} {}\n",
            &format!("Size of {} git repos:", self.numb_git_repos_bare_repos),
            self.total_git_repos_bare_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap()
        ));

        s.push_str(&format!(
            "{: <41} {}\n",
            &format!("Size of {} git repo checkouts:", self.numb_git_checkouts),
            self.total_git_chk_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap()
        ));
        println!("{}", s);
    }
}

pub(crate) struct CargoCacheDirs {
    pub(crate) cargo_home: PathBuf,
    pub(crate) bin_dir: PathBuf,
    pub(crate) registry: PathBuf,
    pub(crate) registry_cache: PathBuf,
    pub(crate) registry_sources: PathBuf,
    pub(crate) git_repos_bare: PathBuf,
    pub(crate) git_checkouts: PathBuf,
}

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
    RemoveFailed,
    RemoveDirNoArg,
}

impl CargoCacheDirs {
    pub(crate) fn new() -> Result<Self, (ErrorKind, String)> {
        let cargo_cfg = match cargo::util::config::Config::default() {
            Ok(cargo_cfg) => cargo_cfg,
            Err(_) => {
                return Err((
                    ErrorKind::CargoFailedGetConfig,
                    "Failed to get cargo config!".to_string(),
                ))
            }
        };

        let cargo_home_path = cargo_cfg.home().clone().into_path_unlocked();
        let cargo_home_str = cargo_home_path.display();
        let cargo_home_path_clone = cargo_home_path.clone();

        if !cargo_home_path.is_dir() {
            let msg = format!(
                "Error, no cargo home path directory '{}' found.",
                &cargo_home_str
            );
            return Err((ErrorKind::CargoHomeNotDirectory, msg));
        }
        // get the paths to the relevant directories
        let cargo_home = cargo_home_path;
        let bin = cargo_home.join("bin/");
        let registry = cargo_home.join("registry/");
        let reg_cache = registry.join("cache/");
        let reg_src = registry.join("src/");
        let git_repos_bare = cargo_home.join("git/db/");
        let git_checkouts = cargo_home_path_clone.join("git/checkouts/");

        Ok(Self {
            cargo_home,
            bin_dir: bin,
            registry,
            registry_cache: reg_cache,
            registry_sources: reg_src,
            git_repos_bare,
            git_checkouts,
        })
    }

    pub(crate) fn print_dir_paths(&self) {
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
        println!(
            "git db directory:             {}",
            &self.git_repos_bare.display()
        );
        println!(
            "git checkouts dir:            {}",
            &self.git_checkouts.display()
        );
    }
}

pub(crate) fn cumulative_dir_size(dir: &PathBuf) -> DirInfoObj {
    // Note: using a hashmap to cache dirsizes does apparently not pay out performance-wise
    if !dir.is_dir() {
        return DirInfoObj {
            dir_size: 0,
            file_number: 0,
        };
    }
    // traverse recursively and sum filesizes
    let mut files = Vec::new();
    for entry in WalkDir::new(dir.display().to_string()) {
        files.push(entry.unwrap().path().to_owned());
    }
    // would like to get rid of the vector here but not sure how to convert
    // WalkDir iterator into rayon par_iter

    // parallelize using rayon
    let sizes_sum = files
        .par_iter()
        .map(|f| {
            fs::metadata(f)
                .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &dir.display()))
                .len()
        }).sum();

    // for the file number, we don't want the actual number of files but only the number of
    // files in the current directory.
    let mut numb_files = 0_u64;
    if dir.display().to_string().contains("registry") {
        for _ in WalkDir::new(dir.display().to_string())
            .max_depth(2)
            .min_depth(2)
        {
            numb_files += 1;
        }
    } else {
        for _ in WalkDir::new(dir.display().to_string()).max_depth(1) {
            numb_files += 1;
        }
    }

    DirInfoObj {
        dir_size: sizes_sum,
        file_number: numb_files,
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
    // src can be completely nuked since we can always rebuilt it from cache
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
                    .unwrap_or_else(|_| {
                        panic!("Failed to get metadata of file '{}'", &pkgpath.display())
                    }).len();
                if dry_run {
                    println!(
                        "dry run: not actually deleting {} {} at {}",
                        pkgname,
                        pkgver,
                        pkgpath.display()
                    );
                } else {
                    println!("deleting: {} {} at {}", pkgname, pkgver, pkgpath.display());
                    fs::remove_file(pkgpath).unwrap_or_else(|_| {
                        panic!("Failed to remove file '{}'", pkgpath.display())
                    });
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
                        .unwrap_or_else(|_| {
                            panic!("Failed to get metadata of file '{}'", &pkgpath.display())
                        }).len();
                    if dry_run {
                        println!(
                            "dry run: not actually deleting {} {} at {}",
                            pkgname,
                            pkgver,
                            pkgpath.display()
                        );
                    } else {
                        println!("deleting: {} {} at {}", pkgname, pkgver, pkgpath.display());
                        fs::remove_file(pkgpath).unwrap_or_else(|_| {
                            panic!("Failed to remove file '{}'", pkgpath.display())
                        });
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
        removed_size.file_size(file_size_opts::DECIMAL).unwrap()
    );
    Ok(())
}

pub(crate) fn print_info(c: &CargoCacheDirs, s: &DirSizesCollector) {
    println!("Found CARGO_HOME / cargo cache base dir");
    println!(
        "\t\t\t'{}' of size: {}",
        &c.cargo_home.display(),
        s.total_size.file_size(file_size_opts::DECIMAL).unwrap()
    );

    println!("Found {} binaries installed in", s.numb_bins);
    println!(
        "\t\t\t'{}', size: {}",
        &c.bin_dir.display(),
        s.total_bin_size.file_size(file_size_opts::DECIMAL).unwrap()
    );
    println!("\t\t\tNote: use 'cargo uninstall' to remove binaries, if needed.");

    println!("Found registry base dir:");
    println!(
        "\t\t\t'{}', size: {}",
        &c.registry.display(),
        s.total_reg_size.file_size(file_size_opts::DECIMAL).unwrap()
    );
    println!("Found registry crate source cache:");
    println!(
        "\t\t\t'{}', size: {}",
        &c.registry_cache.display(),
        s.total_reg_cache_size
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    );
    println!("\t\t\tNote: removed crate sources will be redownloaded if necessary");
    println!("Found registry unpacked sources");
    println!(
        "\t\t\t'{}', size: {}",
        &c.registry_sources.display(),
        s.total_reg_src_size
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    );
    println!("\t\t\tNote: removed unpacked sources will be reextracted from local cache (no net access needed).");

    println!("Found git repo database:");
    println!(
        "\t\t\t'{}', size: {}",
        &c.git_repos_bare.display(),
        s.total_git_repos_bare_size
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    );
    println!("\t\t\tNote: removed git repositories will be recloned if necessary");
    println!("Found git repo checkouts:");
    println!(
        "\t\t\t'{}', size: {}",
        &c.git_checkouts.display(),
        s.total_git_chk_size
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    );
    println!(
        "\t\t\tNote: removed git checkouts will be rechecked-out from repo database if necessary (no net access needed, if repos are up-to-date)."
    );
}

pub(crate) fn size_diff_format(size_before: u64, size_after: u64, dspl_sze_before: bool) -> String {
    #[cfg_attr(feature = "cargo-clippy", allow(cast_possible_wrap))]
    let size_diff: i64 = size_after as i64 - size_before as i64;
    let sign = if size_diff > 0 { "+" } else { "" };
    let size_after_human_readable = size_after.file_size(file_size_opts::DECIMAL).unwrap();
    let humansize_opts = file_size_opts::FileSizeOpts {
        allow_negative: true,
        ..file_size_opts::DECIMAL
    };
    let size_diff_human_readable = size_diff.file_size(humansize_opts).unwrap();
    let size_before_human_readabel = size_before.file_size(file_size_opts::DECIMAL).unwrap();
    // percentage
    #[cfg_attr(feature = "cargo-clippy", allow(cast_precision_loss))]
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
    ccd: &CargoCacheDirs,
    size_changed: &mut bool,
) -> Result<(), (ErrorKind, String)> {
    fn rm(
        dir: &PathBuf,
        dry_run: bool,
        size_changed: &mut bool,
    ) -> Result<(), (ErrorKind, String)> {
        // remove a specified subdirectory from cargo cache
        if !dir.is_dir() {
        } else if dry_run {
            println!("dry-run: would delete: '{}'", dir.display());
        } else {
            println!("removing: '{}'", dir.display());
            match fs::remove_dir_all(&dir) {
                Ok(_) => {}
                Err(_) => {
                    return Err((
                        ErrorKind::RemoveFailed,
                        format!("failed to remove directory {}", dir.display()),
                    ))
                }
            }
            *size_changed = true;
        }
        Ok(())
    }

    let input = match directory {
        Some(value) => value,
        None => {
            return Err((
                ErrorKind::RemoveDirNoArg,
                "No argument assigned to --remove-dir, example: 'git-repos,registry-sources'"
                    .to_string(),
            ))
        }
    };

    //@TODO remove vec
    let inputs = input.split(',').collect::<Vec<&str>>();
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

    for word in &inputs {
        if valid_dirs.contains(word) {
            // dir is recognized
            // dedupe
            match *word {
                "all" => {
                    rm_git_repos = true;
                    rm_git_checkouts = true;
                    rm_registry_sources = true;
                    rm_registry_crate_cache = true;
                    // we rm everything, no need to look further, break out of loop
                    break; // for word in &inputs
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
    } // for word in &inputs
    if terminate {
        // remove the last character which is a trailing whitespace
        invalid_dirs.pop();
        return Err((
            ErrorKind::InvalidDeletableDir,
            format!("Invalid deletable dirs: {}", invalid_dirs),
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
        rm(&ccd.registry_cache, dry_run, size_changed)?
    }
    Ok(())
}
