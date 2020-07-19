// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
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
#![deny(unsafe_code, clippy::unimplemented)]
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
    clippy::redundant_clone,
    clippy::empty_enum,
    clippy::explicit_iter_loop,
    clippy::match_same_arms,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::path_buf_push_overwrite,
    clippy::inefficient_to_string,
    clippy::trivially_copy_pass_by_ref,
    clippy::let_unit_value,
    clippy::option_option,
//   clippy::wildcard_enum_match_arm // too many FPS for _ => unreachable!()
)]
// suppress these warnings:
// #![allow(clippy::redundant_pub_crate)] // conflicts with unreachable_pub
#![allow(clippy::too_many_lines, clippy::unused_self)] // I don't care
#![allow(clippy::wildcard_imports)] // breaks code, false positives

// for the "ci-autoclean" feature, we don't need all these modules so ignore them
cfg_if::cfg_if! {
    if #[cfg(not(feature = "ci-autoclean"))] {
        // mods
        mod cache;
        mod cli;
        mod commands;
        mod dirsizes;
        mod tables;
        mod git;
        mod library;
        mod remove;
        mod top_items;
        mod top_items_summary;
        mod date;
        mod clean_unref;

        // use
        use crate::cache::caches::{Cache, RegistrySuperCache};
        use clap::value_t;
        use std::process;
        use std::time::SystemTime;
        use walkdir::WalkDir;
        use crate::cache::*;
        use crate::commands::{local, query, sccache};
        use crate::git::*;
        use crate::library::*;
        use crate::remove::*;
        use crate::top_items_summary::*;
        use crate::clean_unref::*;
    }
}

#[cfg(all(any(test, feature = "bench", not(feature = "ci-autoclean"))))]
mod test_helpers;

#[cfg(all(test, feature = "bench", not(feature = "ci-autoclean")))]
extern crate test; //hack

// the default main function
#[allow(clippy::cognitive_complexity)]
#[cfg(not(feature = "ci-autoclean"))]
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

    if config.is_present("sc") || config.is_present("sccache") {
        sccache::sccache_stats();
        process::exit(0);
    }

    // indicates if size changed and whether we should print a before/after size diff
    let mut size_changed: bool = false;

    let cargo_cache = match CargoCachePaths::default() {
        Ok(cargo_cache) => cargo_cache,
        Err(e) => {
            eprintln!("{}", e);
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

    if let Some(clean_unref_cfg) = config.subcommand_matches("clean-unref") {
        match clean_unref(
            &cargo_cache,
            &clean_unref_cfg.value_of("manifest-path"),
            &mut checkouts_cache,
            &mut bare_repos_cache,
            &mut registry_pkgs_cache,
            &mut registry_sources_caches,
            config.is_present("dry-run") || clean_unref_cfg.is_present("dry-run"),
            &mut size_changed,
        ) {
            Ok(_) => {
                process::exit(0);
            }
            Err(e) => {
                eprintln!("{:?}", e);
                process::exit(1);
            }
        }
    }

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

        let query = query::run_query(
            query_config,
            &mut bin_cache,
            &mut checkouts_cache,
            &mut bare_repos_cache,
            &mut registry_pkgs_cache,
            &mut registry_sources_caches,
        );
        if let Err(e) = query {
            eprintln!("{}", e);
            process::exit(1)
        } else {
            process::exit(0);
        }
    } else if config.is_present("local") || config.is_present("l") {
        // this is not actually not needed and was previously passed into local_subcmd()
        /*
        let local_config = if config.is_present("local") {
            config
                .subcommand_matches("local")
                .expect("unwrap failed here")
        } else {
            config.subcommand_matches("l").expect("unwrap failed there")
        }; */

        match local::local_subcmd() {
            Ok(_) => {
                process::exit(0);
            }
            Err(error) => {
                eprintln!("{}", error);
                process::exit(1);
            }
        }
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
    let dir_sizes_total = dir_sizes.total_size();

    if config.is_present("remove-if-younger-than") || config.is_present("remove-if-older-than") {
        let res = crate::date::remove_files_by_dates(
            &mut checkouts_cache,
            &mut bare_repos_cache,
            &mut registry_pkgs_cache,
            /* &mut registry_index_cache, */
            &mut registry_sources_caches,
            &config.value_of("remove-if-younger-than"),
            &config.value_of("remove-if-older-than"),
            config.is_present("dry-run"),
            &config.value_of("remove-dir"),
            &mut size_changed,
        );
        match res {
            Err(error) => {
                eprintln!("{}", error);
                std::process::exit(1);
            }
            Ok(()) => {
                //@TODO we could perhaps optimize this by only querying the caches that changed
                if !config.is_present("dry-run") {
                    print_size_changed_summary(
                        dir_sizes_total,
                        &cargo_cache,
                        &mut bin_cache,
                        &mut checkouts_cache,
                        &mut bare_repos_cache,
                        &mut registry_pkgs_cache,
                        &mut registry_index_caches,
                        &mut registry_sources_caches,
                    );
                }
                // don't run --remove-dir stuff (since we also required that parameter)
                std::process::exit(0);
            }
        }
    }

    if config.is_present("info") {
        println!("{}", get_info(&cargo_cache, &dir_sizes));
        process::exit(0);
    }

    // no println!() here!
    // print the default summary
    let output = if config.subcommand_matches("registry").is_some()
        || config.subcommand_matches("r").is_some()
        || config.subcommand_matches("registries").is_some()
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

    if config.is_present("remove-dir")
        && !(config.is_present("remove-if-younger-than")
            || config.is_present("remove-if-older-than"))
    {
        if let Err(e) = remove_dir_via_cmdline(
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
            eprintln!("{}", e);
            process::exit(1);
        }
    }

    if config.is_present("fsck-repos") {
        git_fsck_everything(&cargo_cache.git_repos_bare, &cargo_cache.registry_pkg_cache);
        std::process::exit(0);
    }

    if config.is_present("gc-repos") || config.is_present("autoclean-expensive") {
        if let Err(e) = git_gc_everything(
            &cargo_cache.git_repos_bare,
            &cargo_cache.registry_pkg_cache,
            config.is_present("dry-run"),
        ) {
            eprintln!("{}", e);
            process::exit(2);
        }
        size_changed = true;
    }

    if config.is_present("autoclean") || config.is_present("autoclean-expensive") {
        // clean the registry sources and git checkouts
        let reg_srcs = &cargo_cache.registry_sources;
        let git_checkouts = &cargo_cache.git_checkouts;

        // depending on the size of the cache and the system (SSD, HDD...) this can take a few seconds.
        println!("\nClearing cache...");

        for dir in &[reg_srcs, git_checkouts] {
            let size = cumulative_dir_size(dir);
            if dir.is_dir() {
                remove_file(
                    dir,
                    config.is_present("dry-run"),
                    &mut size_changed,
                    None,
                    &DryRunMessage::Default,
                    Some(size.dir_size),
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
            Err(error) => {
                match error {
                    Error::MalformedPackageName(_) => {
                        panic!("{}", error);
                    }
                    _ => unreachable!(),
                };
            }
        }
    }

    if size_changed && !config.is_present("dry-run") {
        // size has changed, print summary of how size has changed

        print_size_changed_summary(
            dir_sizes_total,
            &cargo_cache,
            &mut bin_cache,
            &mut checkouts_cache,
            &mut bare_repos_cache,
            &mut registry_pkgs_cache,
            &mut registry_index_caches,
            &mut registry_sources_caches,
        );
    }

    if !size_changed
        && config.is_present("dry-run")
        // none of the flags that do on-disk changes are present
        && !(config.is_present("keep-duplicate-crates")
            || config.is_present("autoclean")
            || config.is_present("autoclean-expensive")
            || config.is_present("gc-repos")
            || config.is_present("fsck-repos")
            || config.is_present("remove-dir"))
    {
        eprintln!("Warning: there is nothing to be dry run!");
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

// the main function when using the ci-autoclean feature
// this is a very stripped-down version of cargo-cache which has minimal external dependencies and should
// compile within a couple of seconds in order to be used on CI to clean the cargo-home for caching on CI-cache (travis/azure etc)
#[cfg(feature = "ci-autoclean")]
fn main() {
    use std::path::PathBuf;

    #[derive(Debug, Clone)]
    struct CargoCachePaths {
        /// path where registry sources (.rs files / extracted .crate archives) are stored
        registry_sources: PathBuf,

        /// git repository checkouts are stored here
        git_checkouts: PathBuf,
    }

    impl CargoCachePaths {
        /// returns `CargoCachePaths` object which makes all the subpaths accessible to the crate
        pub(crate) fn default() -> Result<Self, ()> {
            let cargo_home = if let Ok(cargo_home) = home::cargo_home() {
                cargo_home
            } else {
                std::process::exit(1);
            };

            if !cargo_home.is_dir() {
                std::process::exit(1);
            }
            // get the paths to the relevant directories
            let registry = cargo_home.join("registry");
            let reg_src = registry.join("src");
            let git_checkouts = cargo_home.join("git").join("checkouts");

            Ok(Self {
                registry_sources: reg_src,
                git_checkouts,
            })
        }
    } // impl CargoCachePaths

    pub(crate) fn remove_file(path: &PathBuf) {
        if path.is_file() && std::fs::remove_file(&path).is_err() {
            eprintln!("Warning: failed to remove file \"{}\".", path.display());
        }

        if path.is_dir() && remove_dir_all::remove_dir_all(&path).is_err() {
            eprintln!(
                "Warning: failed to recursively remove directory \"{}\".",
                path.display()
            );
        }
    }

    let cargo_cache = match CargoCachePaths::default() {
        Ok(cargo_cache) => cargo_cache,
        Err(_e) => {
            std::process::exit(1);
        }
    };

    println!("cargo-cache: running \"cargo cache --autoclean\"");

    let reg_srcs = &cargo_cache.registry_sources;
    let git_checkouts = &cargo_cache.git_checkouts;
    for dir in &[reg_srcs, git_checkouts] {
        if dir.is_dir() {
            remove_file(dir);
        }
    }
}
