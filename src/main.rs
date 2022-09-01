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
    //clippy::shadow_reuse,
    clippy::shadow_same,
    clippy::shadow_unrelated,
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
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
//   clippy::wildcard_enum_match_arm // too many FPS for _ => unreachable!()
    clippy::index_refutable_slice,
    clippy::return_self_not_must_use,
    // clippy::string_slice, // fixme!
)]
// suppress these warnings:
// #![allow(clippy::redundant_pub_crate)] // conflicts with unreachable_pub
#![allow(clippy::too_many_lines, clippy::unused_self)] // I don't care
#![allow(clippy::wildcard_imports)] // breaks code, false positives
#![allow(clippy::option_if_let_else)] // too pedantic, not that useful...
#![allow(clippy::upper_case_acronyms)] // questionable
#![allow(clippy::needless_for_each)] // I like my iterators :(
#![allow(clippy::assertions_on_result_states)] // not that useful imo

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
        mod verify;

        // use
        use crate::cache::caches::{Cache, RegistrySuperCache};
        use std::process;
        use std::time::SystemTime;
        use walkdir::WalkDir;
        use crate::cache::*;
        use crate::commands::{local, query, sccache, trim, toolchains};
        use crate::git::*;
        use crate::library::*;
        use crate::remove::*;
        use crate::top_items_summary::*;
        use crate::clean_unref::*;
        use crate::cli::{CargoCacheCommands};
        //use crate::verify;
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

    let config_enum = cli::clap_to_enum(config);

    // handle hidden "version" subcommand
    if config.is_present("version") || matches!(config_enum, CargoCacheCommands::Version) {
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

    match &config_enum {
        CargoCacheCommands::SCCache => sccache::sccache_stats().exit_or_fatal_error(),
        CargoCacheCommands::Toolchain => {
            toolchains::toolchain_stats();
            process::exit(0);
        }
        _ => {}
    }

    // indicates if size changed and whether we should print a before/after size diff
    let mut size_changed: bool = false;

    let cargo_cache = CargoCachePaths::default().unwrap_or_fatal_error();

    if let CargoCacheCommands::ListDirs = config_enum {
        // only print the directories and exit, don't calculate anything else
        println!("{}", cargo_cache);
        process::exit(0);
    }

    // create cache
    let p = CargoCachePaths::default().unwrap();

    let mut bin_cache = bin::BinaryCache::new(p.bin_dir);
    let mut checkouts_cache = git_checkouts::GitCheckoutCache::new(p.git_checkouts);
    let mut bare_repos_cache = git_bare_repos::GitRepoCache::new(p.git_repos_bare);

    let mut registry_pkgs_cache =
        registry_pkg_cache::RegistryPkgCaches::new(p.registry_pkg_cache.clone());

    //let mut registry_index_cache = registry_index::RegistryIndexCache::new(p.registry_index);

    let mut registry_sources_caches =
        registry_sources::RegistrySourceCaches::new(p.registry_sources);

    let p2 = CargoCachePaths::default().unwrap(); //@TODO remove this

    let mut registry_index_caches: registry_index::RegistryIndicesCache =
        registry_index::RegistryIndicesCache::new(p2.registry_index);

    // this should populate the entire cache, not very happy about this, wen we do this more lazily?
    let dir_sizes_original = dirsizes::DirSizes::new(
        &mut bin_cache,
        &mut checkouts_cache,
        &mut bare_repos_cache,
        &mut registry_pkgs_cache,
        &mut registry_index_caches,
        &mut registry_sources_caches,
        &cargo_cache,
    );

    match config_enum {
        CargoCacheCommands::Trim {
            dry_run,
            trim_limit,
        } => {
            let trim_result = trim::trim_cache(
                trim_limit,
                &mut checkouts_cache,
                &mut bare_repos_cache,
                &mut registry_pkgs_cache,
                &mut registry_sources_caches,
                dry_run,
                &mut size_changed,
            );
            dirsizes::DirSizes::print_size_difference(
                &dir_sizes_original,
                &cargo_cache,
                &mut bin_cache,
                &mut checkouts_cache,
                &mut bare_repos_cache,
                &mut registry_pkgs_cache,
                &mut registry_index_caches,
                &mut registry_sources_caches,
            );
            trim_result.exit_or_fatal_error();
        }
        CargoCacheCommands::CleanUnref {
            dry_run,
            manifest_path,
        } => {
            let clean_unref_result = clean_unref(
                &cargo_cache,
                manifest_path,
                &mut bin_cache,
                &mut checkouts_cache,
                &mut bare_repos_cache,
                &mut registry_pkgs_cache,
                &mut registry_index_caches,
                &mut registry_sources_caches,
                dry_run,
                &mut size_changed,
            );
            dirsizes::DirSizes::print_size_difference(
                &dir_sizes_original,
                &cargo_cache,
                &mut bin_cache,
                &mut checkouts_cache,
                &mut bare_repos_cache,
                &mut registry_pkgs_cache,
                &mut registry_index_caches,
                &mut registry_sources_caches,
            );
            clean_unref_result.exit_or_fatal_error();
        }
        CargoCacheCommands::TopCacheItems { limit } => {
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
        }
        CargoCacheCommands::Query { query_config } => {
            query::run_query(
                query_config,
                &mut bin_cache,
                &mut checkouts_cache,
                &mut bare_repos_cache,
                &mut registry_pkgs_cache,
                &mut registry_sources_caches,
            )
            .exit_or_fatal_error();
        }
        CargoCacheCommands::Local => {
            local::local_subcmd().exit_or_fatal_error();
        }
        CargoCacheCommands::RemoveIfDate {
            dry_run,
            arg_younger,
            arg_older,
            dirs,
        } => {
            let res = crate::date::remove_files_by_dates(
                &mut checkouts_cache,
                &mut bare_repos_cache,
                &mut registry_pkgs_cache,
                /* &mut registry_index_cache, */
                &mut registry_sources_caches,
                arg_younger,
                arg_older,
                dry_run,
                dirs,
                &mut size_changed,
            );

            dirsizes::DirSizes::print_size_difference(
                &dir_sizes_original,
                &cargo_cache,
                &mut bin_cache,
                &mut checkouts_cache,
                &mut bare_repos_cache,
                &mut registry_pkgs_cache,
                &mut registry_index_caches,
                &mut registry_sources_caches,
            );
            // don't run --remove-dir stuff (since we also required that parameter)

            res.exit_or_fatal_error();
        }
        CargoCacheCommands::Info => {
            println!("{}", get_info(&cargo_cache, &dir_sizes_original));
            process::exit(0);
        }
        // This one must come BEFORE RemoveIfDate because that one also uses --remove dir
        CargoCacheCommands::RemoveDir { dry_run } => {
            let res = remove_dir_via_cmdline(
                config.value_of("remove-dir"),
                dry_run,
                &cargo_cache,
                &mut size_changed,
                &mut checkouts_cache,
                &mut bare_repos_cache,
                &mut registry_index_caches,
                &mut registry_pkgs_cache,
                &mut registry_sources_caches,
            );

            dirsizes::DirSizes::print_size_difference(
                &dir_sizes_original,
                &cargo_cache,
                &mut bin_cache,
                &mut checkouts_cache,
                &mut bare_repos_cache,
                &mut registry_pkgs_cache,
                &mut registry_index_caches,
                &mut registry_sources_caches,
            );
            res.unwrap_or_fatal_error();
        }
        CargoCacheCommands::FSCKRepos => {
            git_fsck_everything(&cargo_cache.git_repos_bare, &cargo_cache.registry_pkg_cache)
                .exit_or_fatal_error();
        }
        CargoCacheCommands::GitGCRepos { dry_run } => {
            //@TODO deduplicate between autoclean-expensive!
            let res = git_gc_everything(
                &cargo_cache.git_repos_bare,
                &cargo_cache.registry_pkg_cache,
                dry_run,
            );

            if !dry_run {
                bare_repos_cache.invalidate();
                registry_index_caches.invalidate();
                size_changed = true;
            }
            // do not terminate cargo cache since gc is part of autoclean-expensive
            res.unwrap_or_fatal_error();
        }

        CargoCacheCommands::AutoClean { dry_run } => {
            // clean the registry sources and git checkouts
            let reg_srcs = &cargo_cache.registry_sources;
            let git_checkouts = &cargo_cache.git_checkouts;

            // depending on the size of the cache and the system (SSD, HDD...) this can take a few seconds.
            println!("Clearing cache...\n");

            for dir in &[reg_srcs, git_checkouts] {
                let size = cumulative_dir_size(dir);
                if dir.is_dir() {
                    remove_file(
                        dir,
                        dry_run,
                        &mut size_changed,
                        None,
                        &DryRunMessage::Default,
                        Some(size.dir_size),
                    );
                }
            }
            registry_sources_caches.invalidate();
            checkouts_cache.invalidate();

            dirsizes::DirSizes::print_size_difference(
                &dir_sizes_original,
                &cargo_cache,
                &mut bin_cache,
                &mut checkouts_cache,
                &mut bare_repos_cache,
                &mut registry_pkgs_cache,
                &mut registry_index_caches,
                &mut registry_sources_caches,
            );
            std::process::exit(0);
        }
        CargoCacheCommands::AutoCleanExpensive { dry_run } => {
            let res = git_gc_everything(
                &cargo_cache.git_repos_bare,
                &cargo_cache.registry_pkg_cache,
                dry_run,
            );

            if !dry_run {
                bare_repos_cache.invalidate();
                registry_index_caches.invalidate();
            }
            // do not terminate cargo cache since gc is part of autoclean-expensive
            res.unwrap_or_fatal_error();
            size_changed = true;

            // clean the registry sources and git checkouts
            let reg_srcs = &cargo_cache.registry_sources;
            let git_checkouts = &cargo_cache.git_checkouts;

            // depending on the size of the cache and the system (SSD, HDD...) this can take a few seconds.
            println!("Clearing cache...\n");

            for dir in &[reg_srcs, git_checkouts] {
                let size = cumulative_dir_size(dir);
                if dir.is_dir() {
                    remove_file(
                        dir,
                        dry_run,
                        &mut size_changed,
                        None,
                        &DryRunMessage::Default,
                        Some(size.dir_size),
                    );
                }
            }
            registry_sources_caches.invalidate();
            checkouts_cache.invalidate();

            dirsizes::DirSizes::print_size_difference(
                &dir_sizes_original,
                &cargo_cache,
                &mut bin_cache,
                &mut checkouts_cache,
                &mut bare_repos_cache,
                &mut registry_pkgs_cache,
                &mut registry_index_caches,
                &mut registry_sources_caches,
            );
            std::process::exit(0);
        }
        CargoCacheCommands::KeepDuplicateCrates { dry_run, limit } => {
            let res = rm_old_crates(
                limit,
                dry_run,
                &cargo_cache.registry_pkg_cache,
                &mut size_changed,
            );
            registry_pkgs_cache.invalidate();
            registry_sources_caches.invalidate();

            dirsizes::DirSizes::print_size_difference(
                &dir_sizes_original,
                &cargo_cache,
                &mut bin_cache,
                &mut checkouts_cache,
                &mut bare_repos_cache,
                &mut registry_pkgs_cache,
                &mut registry_index_caches,
                &mut registry_sources_caches,
            );

            if let Err(error) = res {
                match error {
                    Error::MalformedPackageName(_) => {
                        // force a stacktrace here
                        panic!("{}", error);
                    }
                    _ => unreachable!(),
                };
            }
        }
        CargoCacheCommands::OnlyDryRun => {
            if !size_changed {
                eprintln!("Warning: there is nothing to be dry run!");
            }
        }
        CargoCacheCommands::Verify {
            clean_corrupted,
            dry_run,
        } => {
            println!("Verifying cache, this may take some time...\n");
            if let Err(failed_verifications) = verify::verify_crates(&mut registry_sources_caches) {
                eprintln!("\n");
                failed_verifications
                    .iter()
                    .for_each(|diff| println!("{}", diff.details()));
                eprintln!(
                    "\nFound {} possible corrupted sources.",
                    failed_verifications.len()
                );

                if clean_corrupted {
                    verify::clean_corrupted(
                        &mut registry_sources_caches,
                        &failed_verifications,
                        dry_run,
                    );
                } else {
                    println!("Hint: use `cargo cache verify --clean-corrupted` to remove them.");
                }

                std::process::exit(1)
            } else {
                std::process::exit(0);
            }
        }
        _ => (),
    }

    if size_changed && !config.is_present("dry-run") {
        // size has changed, print summary of how size has changed

        dirsizes::DirSizes::print_size_difference(
            &dir_sizes_original,
            &cargo_cache,
            &mut bin_cache,
            &mut checkouts_cache,
            &mut bare_repos_cache,
            &mut registry_pkgs_cache,
            &mut registry_index_caches,
            &mut registry_sources_caches,
        );
    }

    // no println!() here!
    // print the default summary
    if matches!(config_enum, CargoCacheCommands::Registries) {
        // print per-registry summary
        let output = dirsizes::per_registry_summary(
            &dir_sizes_original,
            &mut registry_index_caches,
            &mut registry_sources_caches,
            &mut registry_pkgs_cache,
        );
        print!("{}", output);
    } else if matches!(config_enum, CargoCacheCommands::DefaultSummary) {
        // default summary
        print!("{}", dir_sizes_original);
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
    use std::path::{Path, PathBuf};

    #[derive(Debug, Clone)]
    struct CargoCachePaths {
        /// path where registry sources (.rs files / extracted .crate archives) are stored
        registry_sources: PathBuf,

        /// git repository checkouts are stored here
        git_checkouts: PathBuf,
    }

    impl CargoCachePaths {
        /// returns `CargoCachePaths` object which makes all the subpaths accessible to the crate
        pub(crate) fn default() -> Self {
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

            Self {
                registry_sources: reg_src,
                git_checkouts,
            }
        }
    } // impl CargoCachePaths

    pub(crate) fn remove_file(path: &Path) {
        if path.is_file() && std::fs::remove_file(path).is_err() {
            eprintln!("Warning: failed to remove file \"{}\".", path.display());
        }

        if path.is_dir() && remove_dir_all::remove_dir_all(path).is_err() {
            eprintln!(
                "Warning: failed to recursively remove directory \"{}\".",
                path.display()
            );
        }
    }

    let cargo_cache = CargoCachePaths::default();

    println!("cargo-cache: running \"cargo cache --autoclean\"");

    let reg_srcs = &cargo_cache.registry_sources;
    let git_checkouts = &cargo_cache.git_checkouts;
    for dir in &[reg_srcs, git_checkouts] {
        if dir.is_dir() {
            remove_file(dir);
        }
    }
}
