use std::fs;
use std::path::PathBuf;

use crate::dirsizes::*;

use humansize::{file_size_opts, FileSize};
use rayon::iter::*;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub(crate) struct DirInfo {
    // make sure we do not accidentally confuse dir_size and file_number
    // since both are of the same type
    pub(crate) dir_size: u64,
    pub(crate) file_number: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct CargoCachePaths {
    pub(crate) cargo_home: PathBuf,
    pub(crate) bin_dir: PathBuf,
    pub(crate) registry: PathBuf,
    pub(crate) registry_cache: PathBuf,
    pub(crate) registry_sources: PathBuf,
    pub(crate) registry_index: PathBuf,
    pub(crate) git_repos_bare: PathBuf,
    pub(crate) git_checkouts: PathBuf,
}

#[derive(Debug, Clone)]
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

impl CargoCachePaths {
    // holds the PathBufs to the different componens of the cargo cache
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
        let registry_index = registry.join("index/");
        let reg_cache = registry.join("cache/");
        let reg_src = registry.join("src/");
        let git_repos_bare = cargo_home.join("git/db/");
        let git_checkouts = cargo_home_path_clone.join("git/checkouts/");

        Ok(Self {
            cargo_home,
            bin_dir: bin,
            registry,
            registry_index,
            registry_cache: reg_cache,
            registry_sources: reg_src,
            git_repos_bare,
            git_checkouts,
        })
    }

    pub(crate) fn get_dir_paths(&self) -> String {
        let mut s = String::with_capacity(500);
        s.push_str("\n");
        s.push_str(&format!(
            "cargo home:                 {}\n",
            &self.cargo_home.display()
        ));

        s.push_str(&format!(
            "binaries directory:         {}\n",
            &self.bin_dir.display()
        ));
        s.push_str(&format!(
            "registry directory:         {}\n",
            &self.registry.display()
        ));
        s.push_str(&format!(
            "registry index:             {}\n",
            &self.registry_index.display()
        ));
        s.push_str(&format!(
            "crate source archives:      {}\n",
            &self.registry_cache.display()
        ));
        s.push_str(&format!(
            "unpacked crate sources:     {}\n",
            &self.registry_sources.display()
        ));
        s.push_str(&format!(
            "bare git repos:             {}\n",
            &self.git_repos_bare.display()
        ));
        s.push_str(&format!(
            "git repo checkouts:         {}\n",
            &self.git_checkouts.display()
        ));
        s
    }
} // impl CargoCachePaths

pub(crate) fn cumulative_dir_size(dir: &PathBuf) -> DirInfo {
    //@TODO: can we Walkdir only once?

    // Note: using a hashmap to cache dirsizes does apparently not pay out performance-wise
    if !dir.is_dir() {
        return DirInfo {
            dir_size: 0,
            file_number: 0,
        };
    }

    // traverse recursively and sum filesizes, parallelized by rayon

    // I would like to get rid of the vector here but not sure how to convert
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
        }).sum();

    // for the file number, we don't want the actual number of files but only the number of
    // files in the current directory.

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

pub(crate) fn get_info(c: &CargoCachePaths, s: &DirSizes) -> String {
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
        &c.registry_cache.display(),
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
    #[allow(clippy::cast_possible_wrap)] // FP due to derives
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

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation
    )]
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
        invalid_dirs.trim();
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

pub(crate) fn get_top_crates(limit: u32, ccd: &CargoCachePaths) -> String {
    // we now have all the sizes and names and version sorted
    #[derive(Debug, Clone)]
    struct FileDesc {
        name: String,
        version: String,
        size: u64,
    }

    impl FileDesc {
        fn new(path: &PathBuf, recursive: bool) -> Self {
            let last = path.to_str().unwrap().split('/').last().unwrap();

            let mut i = last.split('-').collect::<Vec<_>>();
            let version = i.pop().unwrap().trim_right_matches(".crate").to_string();
            let name = i.join("-");

            let size = if recursive {
                let walkdir = WalkDir::new(path.display().to_string());

                walkdir
                    .into_iter()
                    .map(|e| e.unwrap().path().to_owned())
                    .filter(|f| f.exists())
                    .collect::<Vec<_>>()
                    .par_iter()
                    .map(|f| {
                        fs::metadata(f)
                            .unwrap_or_else(|_| {
                                panic!("Failed to get metadata of file '{}'", &path.display())
                            }).len()
                    }).sum()
            } else {
                //  recursive ?
                fs::metadata(&path)
                    .unwrap_or_else(|_| {
                        panic!("Failed to get metadata of file '{}'", &path.display())
                    }).len()
            };

            Self {
                name,
                version,
                size,
            }
        } // fn new()
    } // impl FileDesc

    let mut output = String::new();

    let sources = [
        &ccd.registry_sources,
        &ccd.registry_cache,
        &ccd.git_repos_bare,
        &ccd.git_checkouts,
    ];

    for cache_dir in &sources {
        // do not try to read nonexisting directory (issue #9)
        if !cache_dir.exists() {
            eprintln!(
                "Skipping '{}' because it doesn't exist.",
                cache_dir.display()
            );
            continue;
        }

        output.push_str(&format!("\nSummary for: {:?}\n", cache_dir));

        let recursive: bool = *cache_dir != &ccd.registry_cache;

        // if we check bare git repos or checkouts, we need to calculate sizes slightly different
        let is_git: bool = *cache_dir == &ccd.git_checkouts || *cache_dir == &ccd.git_repos_bare;

        // get list of package all "...\.crate$" files and sort it
        let mut collection = Vec::new();
        if is_git {
            let crate_list = fs::read_dir(&cache_dir)
                .unwrap()
                .map(|cratepath| cratepath.unwrap().path())
                .collect::<Vec<PathBuf>>();
            collection.extend_from_slice(&crate_list);
        } else {
            for repo in fs::read_dir(cache_dir).unwrap() {
                let crate_list = fs::read_dir(&repo.unwrap().path())
                    .unwrap()
                    .map(|cratepath| cratepath.unwrap().path())
                    .collect::<Vec<PathBuf>>();

                collection.extend_from_slice(&crate_list);
            }
        }
        collection.sort();

        let collections_vec = collection
            .iter()
            .map(|path| FileDesc::new(path, recursive))
            .collect::<Vec<_>>();

        let mut summary: Vec<String> = Vec::new();
        let mut current_name = String::new();
        let mut counter: u32 = 0;
        let mut total_size: u64 = 0;
        //let mut max_cratename_len = 0;

        // first find out max_cratename_len
        let max_cratename_len = collections_vec.iter().map(|p| p.name.len()).max().unwrap();

        #[allow(clippy::if_not_else)]
        collections_vec.into_iter().for_each(|pkg| {
            {
                if pkg.name != current_name {
                    // don't push the first empty string
                    if !current_name.is_empty() {
                        let total_size_hr = total_size.file_size(file_size_opts::DECIMAL).unwrap();
                        let average_crate_size = (total_size / u64::from(counter))
                            .file_size(file_size_opts::DECIMAL)
                            .unwrap();

                        if *cache_dir == &ccd.registry_sources {
                            summary.push(format!(
                                "{:0>20} {: <width$} src ckt: {: <3} {: <20}  total: {}\n",
                                total_size,
                                current_name,
                                counter,
                                format!("src avg: {: >9}", average_crate_size),
                                total_size_hr,
                                width = max_cratename_len
                            ));
                        } else if *cache_dir == &ccd.registry_cache {
                            summary.push(format!(
                                "{:0>20} {: <width$} archives: {: <3} {: <20}  total: {}\n",
                                total_size,
                                current_name,
                                counter,
                                format!("crate avg: {: >9}", average_crate_size),
                                total_size_hr,
                                width = max_cratename_len
                            ));
                        } else if *cache_dir == &ccd.git_repos_bare {
                            summary.push(format!(
                                "{:0>20} {: <width$} repo: {: <3} {: <20}  total: {}\n",
                                total_size,
                                current_name,
                                counter,
                                format!("repo avg: {: >9}", average_crate_size),
                                total_size_hr,
                                width = max_cratename_len
                            ));
                        } else if *cache_dir == &ccd.git_checkouts {
                            summary.push(format!(
                                "{:0>20} {: <width$} repo ckt: {: <3} {: <20}  total: {}\n",
                                total_size,
                                current_name,
                                counter,
                                format!("ckt avg: {: >9}", average_crate_size),
                                total_size_hr,
                                width = max_cratename_len
                            ));
                        } else {
                            unreachable!("unknown cache source dir summary requested!");
                        }
                    } // !current_name.is_empty()
                      // new package, reset counting
                    current_name = pkg.name;
                    counter = 1;
                    total_size = pkg.size;
                } else {
                    counter += 1;
                    total_size += pkg.size;
                }
            }
        });

        summary.sort();
        summary.reverse();

        for (c, i) in summary.into_iter().enumerate() {
            if c == limit as usize {
                break;
            }
            let i = &i[21..]; // remove first word used for sorting
            output.push_str(i);
        }
    }
    output
}

#[cfg(test)]
mod libtests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::env;

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
        env::set_var("/tmp/", "CARGO_HOME");
        let dir_paths = CargoCachePaths::new();
        assert!(dir_paths.is_ok());
    }

    #[allow(non_snake_case)]
    #[test]
    fn test_CargoCachePaths_paths() {
        // get cargo target dir
        let mut target_dir = std::env::current_dir().unwrap();
        target_dir.push("target");
        let mut cargo_home = target_dir;
        cargo_home.push("cargo_home");
        //make sure this worked
        let CH_string = format!("{}", cargo_home.display());
        assert!(CH_string.ends_with("cargo-cache/target/cargo_home"));

        // create the directory
        if !std::path::PathBuf::from(&CH_string).is_dir() {
            std::fs::DirBuilder::new().create(&CH_string).unwrap();
        }
        assert!(fs::metadata(&CH_string).unwrap().is_dir());
        assert!(std::path::PathBuf::from(&CH_string).is_dir());

        // set cargo home to this directory
        std::env::set_var("CARGO_HOME", CH_string);
        let ccp = CargoCachePaths::new().unwrap();

        // test all the paths
        assert!(ccp.cargo_home.display().to_string().ends_with("cargo_home"));
        assert!(
            ccp.bin_dir
                .display()
                .to_string()
                .ends_with("cargo_home/bin/")
        );
        assert!(
            ccp.registry
                .display()
                .to_string()
                .ends_with("cargo_home/registry/")
        );
        assert!(
            ccp.registry_index
                .display()
                .to_string()
                .ends_with("cargo_home/registry/index/")
        );
        assert!(
            ccp.registry_cache
                .display()
                .to_string()
                .ends_with("cargo_home/registry/cache/")
        );
        assert!(
            ccp.registry_sources
                .display()
                .to_string()
                .ends_with("cargo_home/registry/src/")
        );
        assert!(
            ccp.git_repos_bare
                .display()
                .to_string()
                .ends_with("cargo_home/git/db/")
        );
        assert!(
            ccp.git_checkouts
                .display()
                .to_string()
                .ends_with("cargo_home/git/checkouts/")
        );
    }

}
