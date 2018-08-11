// these [allow()] by default, make them warn:
#![warn(
    ellipsis_inclusive_range_patterns,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unsafe_code,
    unused,
    rust_2018_compatibility,
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
        clippy_nursery,
        shadow_reuse,
        shadow_same,
        shadow_unrelated,
        pub_enum_variant_names,
        string_add,
        string_add_assign,
        needless_borrow
    )
)]
#![feature(test)]

mod cli;
mod git;
mod library;

use std::{fs, process};

use clap::value_t;
use humansize::{file_size_opts, FileSize};

use crate::git::*;
use crate::library::*;

fn main() {
    // parse args
    // dummy subcommand:
    // https://github.com/kbknapp/clap-rs/issues/937

    let config = cli::gen_clap();
    // we need this in case we call "cargo-cache" directly
    let config = config.subcommand_matches("cache").unwrap_or(&config);
    // indicates if size changed and whether we should print a before/after size diff

    let mut size_changed: bool = false;

    let cargo_cache = match CargoCachePaths::new() {
        Ok(cargo_cache) => cargo_cache,
        Err((_, msg)) => {
            eprintln!("{}", msg);
            process::exit(1);
        }
    };

    if config.is_present("list-dirs") {
        // only print the directories and exit, don't calculate anything else
        println!("{}", cargo_cache.get_dir_paths());
        process::exit(0);
    }

    let dir_sizes = DirSizes::new(&cargo_cache);

    if config.is_present("info") {
        println!("{}", get_info(&cargo_cache, &dir_sizes));
        process::exit(0);
    }

    println!("{}", dir_sizes.print_pretty(&cargo_cache));

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

    if config.is_present("gc-repos") || config.is_present("autoclean-expensive") {
        git_gc_everything(
            &cargo_cache.git_repos_bare,
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
        let cache_size_new = DirSizes::new(&cargo_cache).total_size;

        let size_old_human_readable = cache_size_old.file_size(file_size_opts::DECIMAL).unwrap();
        println!(
            "\nSize changed from {} to {}",
            size_old_human_readable,
            size_diff_format(cache_size_old, cache_size_new, false)
        );
    }
}
