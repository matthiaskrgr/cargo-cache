// Copyright 2017-2019 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// bench feat. cannot be used in beta or stable so hide them behind a feature
#![cfg_attr(all(test, feature = "bench"), feature(test))]
// deny unsafe code
#![deny(unsafe_code)]
// these [allow()] by default, make them warn:
#![warn(
    ellipsis_inclusive_range_patterns,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused,
    unused_qualifications,
    unused_results,
    rust_2018_idioms
)]
// enable additional clippy warnings
#![warn(
    clippy::all,
    clippy::correctness,
    clippy::perf,
    clippy::complexity,
    clippy::style,
    clippy::pedantic,
    clippy::shadow_reuse,
    clippy::shadow_same,
    clippy::shadow_unrelated,
    clippy::pub_enum_variant_names,
    clippy::string_add,
    clippy::string_add_assign,
    clippy::redundant_clone
)]
mod cache;
mod cli;
mod commands;
mod dirsizes;
mod display;
mod git;
mod library;
mod remove;
#[cfg(any(test, feature = "bench"))]
mod test_helpers;
mod top_items;
mod top_items_summary;

#[cfg(all(test, feature = "bench"))]
extern crate test; //hack

use std::process;
use std::time::SystemTime;

use crate::cache::dircache::{Cache, RegistrySuperCache};
use clap::value_t;
use humansize::{file_size_opts, FileSize};
use walkdir::WalkDir;

use crate::cache::*;
use crate::commands::{local, query};
use crate::git::*;
use crate::library::*;
use crate::remove::*;
use crate::top_items_summary::*;

#[allow(clippy::cognitive_complexity)]
fn main() {
    // parse args
    // dummy subcommand:  https://github.com/clap-rs/clap/issues/937
    let config = cli::gen_clap();
    // we need this in case we call "cargo-cache" binary directly
    let config = config.subcommand_matches("cache").unwrap_or(&config);

    // handle hidden "version" subcommand
    if config.is_present("version") {
        println!("cargo-cache {}", cli::get_version());
        process::exit(0);
    }

    let debug_mode: bool = config.is_present("debug");

    // if we are in "debug" mode, get the current time
    let time_started = if debug_mode {
        Some(SystemTime::now())
    } else {
        None
    };

    // indicates if size changed and whether we should print a before/after size diff
    let mut size_changed: bool = false;

    let cargo_cache = match CargoCachePaths::default() {
        Ok(cargo_cache) => cargo_cache,
        Err((_, msg)) => {
            eprintln!("{}", msg);
            process::exit(1);
        }
    };

    if config.is_present("list-dirs") {
        // only print the directories and exit, don't calculate anything else
        println!("{}", cargo_cache);
        process::exit(0);
    }

    // create cache
    let p = CargoCachePaths::default().unwrap();

    let mut bin_cache = bin::BinaryCache::new(p.bin_dir);
    let mut checkouts_cache = git_checkouts::GitCheckoutCache::new(p.git_checkouts);
    let mut bare_repos_cache = git_repos_bare::GitRepoCache::new(p.git_repos_bare);

    let mut registry_pkgs_cache =
        registry_pkg_cache::RegistryPkgCaches::new(p.registry_pkg_cache.clone());

    //let mut registry_index_cache = registry_index::RegistryIndexCache::new(p.registry_index);

    let mut registry_sources_caches =
        registry_sources::RegistrySourceCaches::new(p.registry_sources);

    let p2 = CargoCachePaths::default().unwrap(); //@TODO remove this

    let mut registry_index_caches: registry_index::RegistryIndicesCache =
        registry_index::RegistryIndicesCache::new(p2.registry_index);

    if config.is_present("top-cache-items") {
        let limit =
            value_t!(config.value_of("top-cache-items"), u32).unwrap_or(20 /* default*/);
        if limit > 0 {
            println!(
                "{}",
                get_top_crates(
                    limit,
                    &cargo_cache,
                    &mut bin_cache,
                    &mut checkouts_cache,
                    &mut bare_repos_cache,
                    &mut registry_pkgs_cache,
                    /* &mut registry_index_cache, */
                    &mut registry_sources_caches,
                )
            );
        }
        process::exit(0);
    } else if config.is_present("query") || config.is_present("q") {
        let query_config = if config.is_present("query") {
            config
                .subcommand_matches("query")
                .expect("unwrap failed here")
        } else {
            config.subcommand_matches("q").expect("unwrap failed there")
        };

        query::run_query(
            &query_config,
            &mut bin_cache,
            &mut checkouts_cache,
            &mut bare_repos_cache,
            &mut registry_pkgs_cache,
            &mut registry_sources_caches,
        );

        process::exit(0);
    } else if config.is_present("local") || config.is_present("l") {
        let local_config = if config.is_present("local") {
            config
                .subcommand_matches("local")
                .expect("unwrap failed here")
        } else {
            config.subcommand_matches("l").expect("unwrap failed there")
        };

        local::local_run(&local_config);

        process::exit(0);
    }

    let dir_sizes = dirsizes::DirSizes::new(
        &mut bin_cache,
        &mut checkouts_cache,
        &mut bare_repos_cache,
        &mut registry_pkgs_cache,
        &mut registry_index_caches,
        &mut registry_sources_caches,
        &cargo_cache,
    );

    if config.is_present("info") {
        println!("{}", get_info(&cargo_cache, &dir_sizes));
        process::exit(0);
    }

    // no println!() here!
    // print the default summary
    let output = if config.subcommand_matches("registry").is_some()
        || config.subcommand_matches("r").is_some()
    {
        // print per-registry summary
        dirsizes::per_registry_summary(
            &dir_sizes,
            &mut registry_index_caches,
            &mut registry_sources_caches,
            &mut registry_pkgs_cache,
        )
    } else {
        // print the default cache summary
        dir_sizes.to_string()
    };
    print!("{}", output);

    if config.is_present("remove-dir") {
        if let Err((_, msg)) = remove_dir_via_cmdline(
            config.value_of("remove-dir"),
            config.is_present("dry-run"),
            &cargo_cache,
            &mut size_changed,
            &mut checkouts_cache,
            &mut bare_repos_cache,
            &mut registry_index_caches,
            &mut registry_pkgs_cache,
            &mut registry_sources_caches,
        ) {
            eprintln!("{}", msg);
            process::exit(1);
        }
    }

    if config.is_present("fsck-repos") {
        git_fsck_everything(&cargo_cache.git_repos_bare, &cargo_cache.registry_pkg_cache);
        std::process::exit(0);
    }

    if config.is_present("gc-repos") || config.is_present("autoclean-expensive") {
        git_gc_everything(
            &cargo_cache.git_repos_bare,
            &cargo_cache.registry_pkg_cache,
            config.is_present("dry-run"),
        );
        size_changed = true;
    }

    if config.is_present("autoclean") || config.is_present("autoclean-expensive") {
        let reg_srcs = &cargo_cache.registry_sources;
        let git_checkouts = &cargo_cache.git_checkouts;
        for dir in &[reg_srcs, git_checkouts] {
            if dir.is_dir() {
                remove_file(
                    &dir,
                    config.is_present("dry-run"),
                    &mut size_changed,
                    None,
                    None,
                    None,
                );
            }
        }
    }

    if config.is_present("keep-duplicate-crates") {
        let clap_val = value_t!(config.value_of("keep-duplicate-crates"), u64);
        let limit = match clap_val {
            Ok(x) => x,
            Err(e) => {
                eprintln!(
                    "Error: \"--keep-duplicate-crates\" expected an integer argument.\n{}\"",
                    e
                );
                process::exit(1);
            }
        };
        match rm_old_crates(
            limit,
            config.is_present("dry-run"),
            &cargo_cache.registry_pkg_cache,
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
        // size has changed
        // in order to get a diff, save the old sizes
        let cache_size_old = dir_sizes.total_size;

        // and invalidate the cache
        bin_cache.invalidate();
        checkouts_cache.invalidate();
        bare_repos_cache.invalidate();
        registry_pkgs_cache.invalidate();
        registry_index_caches.invalidate();
        //registry_index_cache.invalidate();
        registry_sources_caches.invalidate();

        // and requery it to let it do its thing
        let cache_size_new = dirsizes::DirSizes::new(
            &mut bin_cache,
            &mut checkouts_cache,
            &mut bare_repos_cache,
            &mut registry_pkgs_cache,
            &mut registry_index_caches,
            &mut registry_sources_caches,
            &cargo_cache,
        )
        .total_size;

        let size_old_human_readable = cache_size_old.file_size(file_size_opts::DECIMAL).unwrap();
        println!(
            "\nSize changed from {} to {}",
            size_old_human_readable,
            size_diff_format(cache_size_old, cache_size_new, false)
        );
    }

    if debug_mode {
        println!("\ndebug:");

        let time_elasped = time_started.unwrap().elapsed().unwrap();

        let cache_root = CargoCachePaths::default().unwrap().cargo_home;

        let wd = WalkDir::new(cache_root.display().to_string());
        let file_count = wd.into_iter().count();
        let time_as_milis = time_elasped.as_millis();
        let time_as_nanos = time_elasped.as_nanos();
        println!("processed {} files in {} ms", file_count, time_as_milis);
        let files_per_ms = file_count as u128 / time_as_milis;
        let ns_per_file = time_as_nanos / file_count as u128;
        println!("{} files per ms", files_per_ms);
        println!("{} ns per file", ns_per_file);
    }
}
