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

use std::{fs, io, process};
use std::io::{stdout, Write};
use std::path::Path;
use std::process::Command;

use clap::{App, Arg, SubCommand};
use humansize::{FileSize, file_size_opts as options};
use walkdir::WalkDir;

struct CacheDir<'a> {
    path: &'a std::path::Path, // path object of the dir
    string: &'a str, // string that represents the dir path
}

struct CacheDirCollector<'a> {
    // an object containing all the relevant cache dirs
    // for easy pasing around to functions
    cargo_home: &'a CacheDir<'a>,
    git_checkouts: &'a CacheDir<'a>,
    git_db: &'a CacheDir<'a>,
    registry: &'a CacheDir<'a>,
    registry_cache: &'a CacheDir<'a>,
    registry_src: &'a CacheDir<'a>,
    bin_dir: &'a CacheDir<'a>,
}

struct DirInfoObj {
    // make sure we do not accidentally confuse dir_size and file_number
    // since both are of the same type
    dir_size: u64,
    file_number: u64,
}

struct DirSizesCollector {
    total_size: u64, // total size of cargo root dir
    numb_bins: u64, // number of binaries found
    total_bin_size: u64, // total size of binaries found
    total_reg_size: u64, // registry size
    total_git_db_size: u64, // git db size
    total_git_chk_size: u64, // git checkout size
    total_reg_cache_size: u64, // registry cache size
    total_reg_src_size: u64, // registry sources size
}

fn gc_repo(pathstr: &str) -> (u64, u64) {
    print!("Recompressing {} : ", pathstr);
    let path = Path::new(pathstr);
    if !path.is_dir() {
        panic!("WARNING: git gc path is not directory: {}", &pathstr);
    }

    // get size before
    let size_before = cumulative_dir_size(pathstr).dir_size;
    let sb_human_readable = size_before.file_size(options::DECIMAL).unwrap();
    print!("{} => ", sb_human_readable);
    // we need to flush stdout manually for incremental print();
    stdout().flush().unwrap();
    let repo = git2::Repository::open(path).unwrap();
    match Command::new("git")
        .arg("gc")
        .arg("--aggressive")
        .arg("--prune=now")
        .current_dir(repo.path())
        .output() {
        Ok(_out) => {}
        /* println!("git gc error\nstatus: {}", out.status);
            println!("stdout:\n {}", String::from_utf8_lossy(&out.stdout));
            println!("stderr:\n {}", String::from_utf8_lossy(&out.stderr));
            //if out.status.success() {}
        } */
        Err(e) => println!("git-gc failed {}", e),
    }
    let size_after = cumulative_dir_size(pathstr).dir_size;
    let sa_human_readable = size_after.file_size(options::DECIMAL).unwrap();
    let mut size_diff = (size_after - size_before) as i64;
    let mut sign = "+";
    if size_diff < 0 {
        sign = "-";
        size_diff *= -1;
    }
    let sd_human_readable = size_diff.file_size(options::DECIMAL).unwrap();
    println!("{} ({}{})", sa_human_readable, sign, sd_human_readable);
    (size_before, size_after)
}

fn cumulative_dir_size(dir: &str) -> DirInfoObj {
    let dir_path = Path::new(dir);
    if !dir_path.is_dir() {
        return DirInfoObj {
            dir_size: 0,
            file_number: 0,
        };
    }
    //@TODO add some clever caching
    let mut cumulative_size = 0;
    let mut number_of_files = 0;
    // traverse recursively and sum filesizes
    for entry in WalkDir::new(dir) {
        let entry = entry.unwrap();
        let path = entry.path();
        //println!("{}", path.display());

        if path.is_file() {
            cumulative_size += fs::metadata(path).unwrap().len();
            number_of_files += 1;
        }
    } // walkdir

    DirInfoObj {
        dir_size: cumulative_size,
        file_number: number_of_files,
    }
}


fn rm_dir(cache: &CacheDirCollector) {
    // remove a directory from cargo cache

    fn print_dirs_to_delete() {
        println!("Possile directories to delete:");
        println!("'git-checkouts', 'git', 'registry'");
        println!("'registry-source-checkouts', 'registry-crate-archives'.");
        println!("'abort' to abort.");
    }

    //print the paths and sizes to give user some context
    println!();
    print_dir_paths(cache);
    println!();

    let mut dirs_to_delete: Vec<&CacheDir> = Vec::new();

    print_dirs_to_delete();

    'inputStrLoop: loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect(
            "Couldn't read input",
        );

        // check what dir we are supposed to delete now
        match input.trim() {
            "git-checkouts" => {
                dirs_to_delete.push(cache.git_checkouts);
                break;
            }
            "git" => {
                // @TODO make sure we print that we are rming bare repos AND checkouts
                dirs_to_delete.push(cache.git_db);
                dirs_to_delete.push(cache.git_checkouts);
                break;
            }
            "registry" => {
                dirs_to_delete.push(cache.registry);
                break;
            }
            "registry-source-checkouts" => {
                dirs_to_delete.push(cache.registry_src);
                break;
            }
            "registry-crate-archives" => {
                dirs_to_delete.push(cache.registry_cache);
                break;
            }
            "bin-dir" => println!("Please use 'cargo uninstall'."),
            "abort" => {
                println!("Terminating...");
                process::exit(0);
            }
            _ => {
                println!("Invalid input.");
                print_dirs_to_delete();


                continue 'inputStrLoop;
            } // _
        } // match input
    } // 'inputStrLoop



    println!(
        "Trying to delete {}",
        dirs_to_delete.first().unwrap().string
    );
    println!(
        "Really delete dir {} ? (yes/no)",
        dirs_to_delete.first().unwrap().string
    );

    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect(
            "Couldn't read line",
        );
        if input.trim() == "yes" {
            println!("deleting {}", dirs_to_delete.first().unwrap().string);
            for dir in dirs_to_delete {
                if dir.path.is_file() {
                    println!("ERROR: {} is not a directory but a file??", dir.string);
                    println!("Doing nothing.");
                } else if dir.path.is_dir() {
                    fs::remove_dir_all(dir.string).unwrap();
                } else {
                    println!("Directory {} does not exist, skipping", dir.string);
                }
            }
            break;
        } else if input == "no" {
            println!("keeping {}", dirs_to_delete.first().unwrap().string);
            break;
        } else {
            println!("Invalid input: {}", input);
            println!("please use 'yes' or 'no'");
        }
    } // loop
} // fn rm_dir

fn print_dir_sizes(s: &DirSizesCollector) {
    println!("\nCargo cache:\n");
    println!(
        "Total size:                        {} ",
        s.total_size.file_size(options::DECIMAL).unwrap()
    );
    println!(
        "Size of {} installed binaries:     {} ",
        s.numb_bins,
        s.total_bin_size.file_size(options::DECIMAL).unwrap()
    );
    println!(
        "Size of registry:                  {} ",
        s.total_reg_size.file_size(options::DECIMAL).unwrap()
    );
    println!(
        "Size of registry crate cache:           {} ",
        s.total_reg_cache_size.file_size(options::DECIMAL).unwrap()
    );
    println!(
        "Size of registry source checkouts:      {} ",
        s.total_reg_src_size.file_size(options::DECIMAL).unwrap()
    );
    println!(
        "Size of git db:                    {} ",
        s.total_git_db_size.file_size(options::DECIMAL).unwrap()
    );
    println!(
        "Size of git repo checkouts:        {} ",
        s.total_git_chk_size.file_size(options::DECIMAL).unwrap()
    );
}

fn print_dir_paths(c: &CacheDirCollector) {
    //println!("cargo base path (CARGO_HOME): {}", cargo_home_str);
    println!("binaries directory:           {}", c.bin_dir.string);
    println!("registry directory:           {}", c.registry.string);
    println!("registry crate source cache:  {}", c.registry_cache.string);
    println!("registry unpacked sources:    {}", c.registry_src.string);
    println!("git db directory:             {}", c.git_db.string);
    println!("git checkouts dir:            {}", c.git_checkouts.string);
}

fn print_info(c: &CacheDirCollector, s: &DirSizesCollector) {
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
        c.registry_src.string,
        s.total_reg_src_size.file_size(options::DECIMAL).unwrap()
    );
    println!(
        "\t\t\tNote: removed unpacked sources will be reextracted from local cache (no net access needed)."
    );

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


fn main() {

    // parse args
    // dummy subcommand:
    // https://github.com/kbknapp/clap-rs/issues/937
    let cargo_cache_cfg = App::new("cargo-cache")
        .version("0.1")
        .bin_name("cargo")
        .about("Manage cargo cache")
        .author("matthiaskrgr")
        .subcommand(
            SubCommand::with_name("cache")
                .arg(
                    Arg::with_name("list-dirs")
                        .short("l")
                        .long("list-dirs")
                        .help("List found directory paths."),
                )
                .arg(
                    Arg::with_name("remove-dirs")
                        .short("r")
                        .long("remove")
                        .help("Select directories in the cache to be removed."),
                )
                .arg(Arg::with_name("gc-repos").short("g").long("gc").help(
                    "Recompress git repositories (may take some time).",
                ))
                .arg(
                    Arg::with_name("info")
                        .short("i")
                        .long("info")
                        .conflicts_with("list-dirs")
                        .help("give information on directories"),
                ),
        ) /*subcmd*/
        .arg(
            Arg::with_name("list-dirs")
                .short("l")
                .long("list-dirs")
                .help("List found directory paths."),
        )
        .arg(
            Arg::with_name("remove-dirs")
                .short("r")
                .long("remove")
                .help("Select directories in the cache to be removed."),
        )
        .arg(Arg::with_name("gc-repos").short("g").long("gc").help(
            "Recompress git repositories (may take some time).",
        ))
        .arg(
            Arg::with_name("info")
                .short("i")
                .long("info")
                .conflicts_with("list-dirs")
                .help("give information on directories"),
        )
        .get_matches();




    // we need this in case we call "cargo-cache" directly
    let cargo_cache_cfg = cargo_cache_cfg.subcommand_matches("cache").unwrap_or(
        &cargo_cache_cfg,
    );
    // get the cargo home dir path from cargo
    let cargo_cfg = cargo::util::config::Config::default().unwrap();
    let cargo_home_str = format!("{}", cargo_cfg.home().display());
    let cargo_home_path = Path::new(&cargo_home_str);

    // make sure we actually have a cargo dir
    if !cargo_home_path.is_dir() {
        println!("Error, no '{} dir found", &cargo_home_str);
        process::exit(1);
    }

    if !cargo_cache_cfg.is_present("list-dirs") {
        println!("Found CARGO_HOME: {}\n", cargo_home_str);
    }

    let bin_dir = (cargo_home_path.clone()).join("bin/");
    let bin_dir_str = bin_dir.clone().into_os_string().into_string().unwrap();


    let registry_dir = (cargo_home_path.clone()).join("registry/");
    let registry_dir_str = (registry_dir.clone())
        .into_os_string()
        .into_string()
        .unwrap();

    let registry_cache = (registry_dir.clone()).join("cache/");
    let registry_cache_str = (registry_cache.clone())
        .into_os_string()
        .into_string()
        .unwrap();
    let registry_sources = (registry_dir.clone()).join("src/");
    let registry_sources_str = (registry_sources.clone())
        .into_os_string()
        .into_string()
        .unwrap();

    let git_db = (cargo_home_path.clone()).join("git/db/");
    let git_db_str = git_db.clone().into_os_string().into_string().unwrap();

    let git_checkouts = (cargo_home_path.clone()).join("git/checkouts/");
    let git_checkouts_str = (git_checkouts.clone())
        .into_os_string()
        .into_string()
        .unwrap();

    let bindir = cumulative_dir_size(&bin_dir_str);
    let dir_sizes = DirSizesCollector {
        total_size: cumulative_dir_size(&cargo_home_str).dir_size,
        numb_bins: bindir.file_number,
        total_bin_size: bindir.dir_size,
        total_reg_size: cumulative_dir_size(&registry_dir_str).dir_size,
        total_git_db_size: cumulative_dir_size(&git_db_str).dir_size,
        total_git_chk_size: cumulative_dir_size(&git_checkouts_str).dir_size,
        total_reg_cache_size: cumulative_dir_size(&registry_cache_str).dir_size,
        total_reg_src_size: cumulative_dir_size(&registry_sources_str).dir_size,
    };


    // link everything into the CacheDirCollector which we can easily pass around to functions
    let cargo_cache = CacheDirCollector {
        cargo_home: &CacheDir {
            path: &cargo_home_path,
            string: &cargo_home_str,
        },
        git_checkouts: &CacheDir {
            path: &git_checkouts,
            string: &git_checkouts_str,
        },
        git_db: &CacheDir {
            path: &git_db,
            string: &git_db_str,
        },
        registry: &CacheDir {
            path: &registry_dir,
            string: &registry_dir_str,
        },
        registry_cache: &CacheDir {
            path: &registry_cache,
            string: &registry_cache_str,
        },
        registry_src: &CacheDir {
            path: &registry_sources,
            string: &registry_sources_str,
        },
        bin_dir: &CacheDir {
            path: &bin_dir,
            string: &bin_dir_str,
        },
    };

    if cargo_cache_cfg.is_present("info") {
        print_info(&cargo_cache, &dir_sizes);
        process::exit(0);
    }


    print_dir_sizes(&dir_sizes);

    if cargo_cache_cfg.is_present("remove-dirs") {
        rm_dir(&cargo_cache);
    } else if cargo_cache_cfg.is_present("list-dirs") {
        print_dir_paths(&cargo_cache);

    }

    // gc cloned git repos of crates or whatever
    if cargo_cache_cfg.is_present("gc-repos") && git_db.is_dir() {
        let mut total_size_before: u64 = 0;
        let mut total_size_after: u64 = 0;

        println!("Recompressing repositories. Please be patient...");
        // gc git repos of crates
        for entry in fs::read_dir(&git_db).unwrap() {
            let entry = entry.unwrap();
            let repo = entry.path();
            let repostr = repo.into_os_string().into_string().unwrap();
            let (before, after) = gc_repo(&repostr);
            total_size_before += before;
            total_size_after += after;
        }

        // gc registries
        let registry_repos_str = format!(
            "{}",
            cargo::util::config::Config::default()
                .unwrap()
                .home()
                .display()
        );
        let registry_repos_path = Path::new(&registry_repos_str).join("registry/").join(
            "index/",
        );
        for repo in fs::read_dir(&registry_repos_path).unwrap() {
            let repo = repo.unwrap().path().join(".git/");
            let repo_str = repo.into_os_string().into_string().unwrap();
            let (before, after) = gc_repo(&repo_str);
            total_size_before += before;
            total_size_after += after;
        } // iterate over registries and gc
        let mut size_diff = (total_size_after - total_size_before) as i64;
        let mut sign = "+";
        if size_diff < 0 {
            sign = "-";
            size_diff *= -1;
        }
        let sd_human_readable = size_diff.file_size(options::DECIMAL).unwrap();

        println!(
            "Compressed {} to {}, ({}{})",
            total_size_before.file_size(options::DECIMAL).unwrap(),
            total_size_after.file_size(options::DECIMAL).unwrap(),
            sign,
            sd_human_readable
        );
    } // gc
}
