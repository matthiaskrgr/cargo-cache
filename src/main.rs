// these [allow()] by default, make them warn:
#![warn(
    ellipsis_inclusive_range_patterns,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unsafe_code,
    unused,
    rust_2018_idioms
)]
// enable additional clippy warnings
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy,
        clippy_correctness,
        clippy_perf,
        clippy_complexity,
        clippy_style,
        clippy_pedantic,
        clippy_nursery
    )
)]
//#![cfg_attr(feature = "cargo-clippy", warn(clippy_cargo))]
// additional warnings from "cippy_restriction" group
#![cfg_attr(
    feature = "cargo-clippy",
    warn(shadow_reuse, shadow_same, shadow_unrelated)
)]
#![cfg_attr(feature = "cargo-clippy", warn(pub_enum_variant_names))]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(string_add, string_add_assign)
)]
#![cfg_attr(feature = "cargo-clippy", warn(needless_borrow))]

mod git;
mod library;
use std::{fs, process};

use clap::{crate_version, value_t, App, Arg, SubCommand};
use humansize::{file_size_opts, FileSize};

use crate::git::*;
use crate::library::*;

fn main() {
    // parse args
    // dummy subcommand:
    // https://github.com/kbknapp/clap-rs/issues/937

    let list_dirs = Arg::with_name("list-dirs")
        .short("l")
        .long("list-dirs")
        .help("List found directory paths.");

    let remove_dir = Arg::with_name("remove-dir").short("r").long("remove-dir")
        .help("Remove directories, accepted values: git-db,git-repos,registry-sources,registry-crate-cache,registry,all")
        .takes_value(true).value_name("dir1,dir2,dir3");

    let gc_repos = Arg::with_name("gc-repos")
        .short("g")
        .long("gc")
        .help("Recompress git repositories (may take some time).");
    let info = Arg::with_name("info")
        .short("i")
        .long("info")
        .conflicts_with("list-dirs")
        .help("Give information on directories");

    let keep_duplicate_crates = Arg::with_name("keep-duplicate-crates")
        .short("k")
        .long("keep-duplicate-crates")
        .help("Remove all but N versions of duplicate crates in the source cache")
        .takes_value(true)
        .value_name("N");

    let dry_run = Arg::with_name("dry-run")
        .short("d")
        .long("dry-run")
        .help("Don't remove anything, just pretend");

    let autoclean = Arg::with_name("autoclean")
        .short("a")
        .long("autoclean")
        .help("Removes registry src checkouts and git repo checkouts");

    let autoclean_expensive = Arg::with_name("autoclean-expensive")
        .short("e")
        .long("autoclean-expensive")
        .help("Removes registry src checkouts, git repo checkouts and gcs repos");

    let config = App::new("cargo-cache")
        .version(crate_version!())
        .bin_name("cargo")
        .about("Manage cargo cache")
        .author("matthiaskrgr")
        .subcommand(
            SubCommand::with_name("cache")
                .version(crate_version!())
                .bin_name("cargo-cache")
                .about("Manage cargo cache")
                .author("matthiaskrgr")
                .arg(&list_dirs)
                .arg(&remove_dir)
                .arg(&gc_repos)
                .arg(&info)
                .arg(&keep_duplicate_crates)
                .arg(&dry_run)
                .arg(&autoclean)
                .arg(&autoclean_expensive),
        ) // subcommand
        .arg(&list_dirs)
        .arg(&remove_dir)
        .arg(&gc_repos)
        .arg(&info)
        .arg(&keep_duplicate_crates)
        .arg(&dry_run)
        .arg(&autoclean)
        .arg(&autoclean_expensive)
        .get_matches();

    // we need this in case we call "cargo-cache" directly
    let config = config.subcommand_matches("cache").unwrap_or(&config);
    // indicates if size changed and whether we should print a before/after size diff
    let mut size_changed: bool = false;

    let cargo_cache = match CargoCacheDirs::new() {
        Ok(cargo_cache) => cargo_cache,
        Err((_, msg)) => {
            eprintln!("{}", msg);
            process::exit(1);
        }
    };

    let dir_sizes = DirSizesCollector::new(&cargo_cache);

    if config.is_present("info") {
        print_info(&cargo_cache, &dir_sizes);
        process::exit(0);
    }

    dir_sizes.print_pretty(&cargo_cache);

    if config.is_present("remove-dir") {
        match remove_dir_via_cmdline(
            config.value_of("remove-dir"),
            config.is_present("dry-run"),
            &cargo_cache,
            &mut size_changed,
        ) {
            Ok(_) => {}
            Err((_, msg)) => {
                eprintln!("{}", msg);
                process::exit(1);
            }
        }
    }

    if config.is_present("list-dirs") {
        cargo_cache.print_dir_paths();
    }
    if config.is_present("gc-repos") || config.is_present("autoclean-expensive") {
        git_gc_everything(
            &cargo_cache.git_db,
            &cargo_cache.registry_cache,
            config.is_present("dry-run"),
        );
        size_changed = true;
    }

    if config.is_present("autoclean") || config.is_present("autoclean-expensive") {
        let reg_srcs = &cargo_cache.registry_sources;
        let git_checkouts = &cargo_cache.git_checkouts;
        for dir in &[reg_srcs, git_checkouts] {
            if dir.is_dir() {
                if config.is_present("dry-run") {
                    println!("would remove directory '{}'", dir.display());
                } else {
                    fs::remove_dir_all(&dir).unwrap();
                    size_changed = true;
                }
            }
        }
    }

    if config.is_present("keep-duplicate-crates") {
        let val =
            value_t!(config.value_of("keep-duplicate-crates"), u64).unwrap_or(10 /* default*/);
        match rm_old_crates(
            val,
            config.is_present("dry-run"),
            &cargo_cache.registry_cache,
            &mut size_changed,
        ) {
            Ok(()) => {}
            Err((error_kind, path)) => {
                match error_kind {
                    ErrorKind::MalformedPackageName => {
                        panic!(format!(
                            "Error: can't parse package string: '{}'",
                            &path.display()
                        ));
                    }
                    _ => unreachable!(),
                };
            }
        }
    }
    if size_changed && !config.is_present("dry-run") {
        let cache_size_old = dir_sizes.total_size;
        // recalculate file sizes by constructing a new DSC object
        let cache_size_new = DirSizesCollector::new(&cargo_cache).total_size;

        let size_old_human_readable = cache_size_old.file_size(file_size_opts::DECIMAL).unwrap();
        println!(
            "\nSize changed from {} to {}",
            size_old_human_readable,
            size_diff_format(cache_size_old, cache_size_new, false)
        );
    }
}
