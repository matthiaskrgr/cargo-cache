// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/// This file provides the command line interface of the cargo-cache crate
use clap::{App, AppSettings, Arg, ArgMatches};

use crate::library::*;
use rustc_tools_util::*;

/// cargo-cache can perform these operaitons, but only one at a time
#[derive(Debug)]
pub(crate) enum CargoCacheCommands<'a> {
    FSCKRepos,

    GitGCRepos {
        dry_run: bool,
    },
    Info,
    KeepDuplicateCrates {
        dry_run: bool,
        limit: u64,
    },
    ListDirs,
    RemoveDir {
        dry_run: bool,
    },
    AutoClean {
        dry_run: bool,
    },
    AutoCleanExpensive {
        dry_run: bool,
    },
    TopCacheItems {
        limit: u32,
    },
    //Debug,
    Version,
    Verify {
        clean_corrupted: bool,
        dry_run: bool,
    },
    Query {
        query_config: &'a ArgMatches,
    }, // subcommand
    Local,      // subcommand
    Registries, // subcommand
    SCCache,    // subcommand
    CleanUnref {
        dry_run: bool,
        manifest_path: Option<&'a str>,
    }, // subcommand
    Trim {
        dry_run: bool,
        trim_limit: Option<&'a str>,
    }, // subcommand
    Toolchain,  // subcommand
    RemoveIfDate {
        dry_run: bool,
        arg_younger: Option<&'a str>,
        arg_older: Option<&'a str>,
        dirs: Option<&'a str>,
    },
    OnlyDryRun,
    DefaultSummary,
}

pub(crate) fn clap_to_enum(config: &ArgMatches) -> CargoCacheCommands<'_> {
    let dry_run = config.is_present("dry-run");

    /*
    // if no args were passed, or ONLY --debug is passed, print the default summary
    if (config.args.is_empty() && config.subcommand.is_none())
        || (config.subcommand.is_none() && config.is_present("debug") && config.args.len()
    {
        return CargoCacheCommands::DefaultSummary;
    }  */

    // Note: the code above worked in clap2.34 but no longer does in clap3.0 because ArgMatches::args and ::subcommand was made private
    // work around using simple std::env::args()

    // skip executable path which is first item
    let args: Vec<String> = std::env::args().collect();
    let args_slice: Vec<&str> = args.iter().map(|s| &**s).collect(); // :(

    match args_slice[..] {
        // the first item is the executable path, we don't need that
        [_] | [_, "cache" | "--debug"] | [_, "cache", "--debug"] => {
            return CargoCacheCommands::DefaultSummary;
        }
        _ => {}
    }

    // if config.is_present("debug") {
    // do not check for "--debug" since it is independent of all other flags
    if config.is_present("version") || config.subcommand_matches("version").is_some() {
        CargoCacheCommands::Version
    } else if config.subcommand_matches("sccache").is_some()
        || config.subcommand_matches("sc").is_some()
    {
        CargoCacheCommands::SCCache
    } else if config.subcommand_matches("toolchain").is_some() {
        CargoCacheCommands::Toolchain
    } else if let Some(trimconfig) = config.subcommand_matches("trim") {
        let trim_dry_run = dry_run || trimconfig.is_present("dry-run");
        CargoCacheCommands::Trim {
            dry_run: trim_dry_run,
            trim_limit: trimconfig.value_of("trim_limit"),
        } // take config trim_config.value_of("trim_limit")
    } else if let Some(clean_unref_config) = config.subcommand_matches("clean-unref") {
        let arg_dry_run = dry_run || clean_unref_config.is_present("dry-run");
        CargoCacheCommands::CleanUnref {
            dry_run: arg_dry_run,
            manifest_path: clean_unref_config.value_of("manifest-path"),
        } // clean_unref_cfg.value_of("manifest-path"),
    } else if config.is_present("top-cache-items") {
        let limit = config
            .value_of("top-cache-items")
            .unwrap_or("20" /* default*/)
            .parse()
            .unwrap_or(20 /* default*/);
        CargoCacheCommands::TopCacheItems { limit }
    } else if let Some(query_config) = config
        .subcommand_matches("query")
        .or_else(|| config.subcommand_matches("q"))
    {
        CargoCacheCommands::Query { query_config }
    } else if config.subcommand_matches("local").is_some()
        || config.subcommand_matches("l").is_some()
    {
        CargoCacheCommands::Local
    } else if config.is_present("info") {
        CargoCacheCommands::Info
    } else if config.is_present("remove-dir") {
        // This one must come BEFORE RemoveIfDate because that one also uses --remove dir
        CargoCacheCommands::RemoveDir { dry_run } //need more info
    } else if config.is_present("autoclean-expensive")
        || (config.is_present("gc-repos") && config.is_present("autoclean"))
    {
        // if we pass both --gc and --autoclean-expensive, we want autoclean-expensive to run
        // since is already includes --gc
        CargoCacheCommands::AutoCleanExpensive { dry_run }
    } else if config.is_present("fsck-repos") {
        CargoCacheCommands::FSCKRepos
    } else if config.is_present("gc-repos") {
        CargoCacheCommands::GitGCRepos { dry_run }
    } else if config.is_present("autoclean") {
        CargoCacheCommands::AutoClean { dry_run }
    } else if config.is_present("keep-duplicate-crates") {
        let limit: u64 = config
            .value_of_t("keep-duplicate-crates")
            .map_err(|_| "Error: \"--keep-duplicate-crates\" expected an integer argument")
            .unwrap_or_fatal_error();
        CargoCacheCommands::KeepDuplicateCrates { dry_run, limit }
    } else if config.subcommand_matches("registry").is_some()
        || config.subcommand_matches("r").is_some()
        || config.subcommand_matches("registries").is_some()
    {
        CargoCacheCommands::Registries
    } else if config.is_present("list-dirs") {
        CargoCacheCommands::ListDirs
    } else if config.is_present("remove-if-younger-than")
        || config.is_present("remove-if-older-than")
    {
        CargoCacheCommands::RemoveIfDate {
            dry_run,
            arg_older: config.value_of("remove-if-younger-than"),
            arg_younger: config.value_of("remove-if-older-than"),
            dirs: config.value_of("remove-dir"),
        }
    } else if let Some(verify_cfg) = config.subcommand_matches("verify") {
        let dry_run2: bool = verify_cfg.is_present("dry-run") || config.is_present("dry-run");
        let clean_corrupted: bool = verify_cfg.is_present("clean-corrupted");
        CargoCacheCommands::Verify {
            clean_corrupted,
            dry_run: dry_run2,
        }
    } else if dry_run {
        // none of the flags that do on-disk changes are present

        // we got "cargo cache --dry-run"
        CargoCacheCommands::OnlyDryRun
    } else {
        unreachable!("Failed to map all clap options to enum?")
    }
}

/// generates the version info with what we have in the build.rs
pub(crate) fn get_version() -> String {
    // remove the "cargo-cache" since CLAP already adds that by itself
    rustc_tools_util::get_version_info!()
        .to_string()
        .replacen("cargo-cache ", "", 1)
}

/// generates the clap config which is used to control the crate
#[allow(clippy::too_many_lines)]
pub(crate) fn gen_clap() -> ArgMatches {
    let version_string = get_version();

    let list_dirs = Arg::new("list-dirs")
        .short('l')
        .long("list-dirs")
        .help("List all found directory paths");

    let remove_dir = Arg::new("remove-dir").short('r').long("remove-dir")
        .help("Remove directories, accepted values: all,git-db,git-repos,\nregistry-sources,registry-crate-cache,registry-index,registry")
        .takes_value(true)
        .value_name("dir1,dir2,dir3");

    let gc_repos = Arg::new("gc-repos")
        .short('g')
        .long("gc")
        .help("Recompress git repositories (may take some time)");

    let fsck_repos = Arg::new("fsck-repos")
        .short('f')
        .long("fsck")
        .help("Fsck git repositories");

    let info = Arg::new("info")
        .short('i')
        .long("info")
        .conflicts_with("list-dirs")
        .help(
            "Print information cache directories, what they are for and what can be safely deleted",
        );

    let keep_duplicate_crates = Arg::new("keep-duplicate-crates")
        .short('k')
        .long("keep-duplicate-crates")
        .help("Remove all but N versions of crate in the source archives directory")
        .takes_value(true)
        .value_name("N");

    let dry_run = Arg::new("dry-run")
        .short('n')
        .long("dry-run")
        .help("Don't remove anything, just pretend");

    let autoclean = Arg::new("autoclean")
        .short('a')
        .long("autoclean")
        .help("Removes crate source checkouts and git repo checkouts");

    let autoclean_expensive = Arg::new("autoclean-expensive")
        .short('e')
        .long("autoclean-expensive")
        .help("As --autoclean, but also recompresses git repositories");

    let list_top_cache_items = Arg::new("top-cache-items")
        .short('t')
        .long("top-cache-items")
        .help("List the top N items taking most space in the cache")
        .takes_value(true)
        .value_name("N");

    let remove_if_older = Arg::new("remove-if-older-than")
        .short('o')
        .long("remove-if-older-than")
        .help("Removes items older than specified date: YYYY.MM.DD or HH:MM:SS")
        .conflicts_with("remove-if-younger-than") // fix later
        .requires("remove-dir")
        .takes_value(true)
        .value_name("date");

    let remove_if_younger = Arg::new("remove-if-younger-than")
        .short('y')
        .long("remove-if-younger-than")
        .help("Removes items younger than the specified date: YYYY.MM.DD or HH:MM:SS")
        .conflicts_with("remove-if-older-than") // fix later
        .requires("remove-dir")
        .takes_value(true)
        .value_name("date");

    let debug = Arg::new("debug")
        .long("debug")
        .help("print some debug stats")
        .hide(true);

    // "version" subcommand which is also hidden, prints crate version
    let version_subcmd = App::new("version").setting(AppSettings::Hidden);

    /***************************
     *       Subcommands        *
     ****************************/

    // <query>
    // arg of query sbcmd
    let query_order = Arg::new("sort")
        .short('s')
        .long("sort-by")
        .help("sort files alphabetically or by file size")
        .takes_value(true)
        .possible_values(["size", "name"]);

    // arg of query sbcmd
    let human_readable = Arg::new("hr")
        .long("human-readable")
        .help("print sizes in human readable format");

    // query subcommand to allow querying
    let query = App::new("query")
        .about("run a query")
        .arg(Arg::new("QUERY"))
        .arg(&query_order)
        .arg(&human_readable);

    // short q (shorter query sbcmd)
    let query_short = App::new("q")
        .about("run a query")
        .arg(Arg::new("QUERY"))
        .arg(&query_order)
        .arg(&human_readable);
    // </query>

    //<local>
    // local subcommand
    let local = App::new("local").about("check local build cache (target) of a rust project");
    // shorter local subcommand (l)
    let local_short = App::new("l").about("check local build cache (target) of a rust project");
    //</local>

    // <registry>
    // registry subcommand
    let registry = App::new("registry").about("query each package registry separately");
    let registry_short = App::new("r").about("query each package registry separately");
    // hidden, but have "cargo cache registries" work too
    let registries_hidden = App::new("registries")
        .about("query each package registry separately")
        .setting(AppSettings::Hidden);
    //</registry>

    //<sccache>
    // local subcommand
    let sccache = App::new("sccache").about("gather stats on a local sccache cache");
    // shorter local subcommand (l)
    let sccache_short = App::new("sc").about("gather stats on a local sccache cache");
    //</sccache>

    //<clean-unref>
    // from cargo
    //
    //fn arg_manifest_path(self) -> Self {
    //    self._arg(opt("manifest-path", "Path to Cargo.toml").value_name("PATH"))
    //}
    //
    // try to emulate this:
    let manifest_path = Arg::new("manifest-path")
        .long("manifest-path")
        .help("Path to Cargo.toml")
        .takes_value(true)
        .value_name("PATH");

    let clean_unref = App::new("clean-unref")
        .about("remove crates that are not referenced in a Cargo.toml from the cache")
        .arg(&manifest_path)
        .arg(&dry_run);
    //</clean-unref>

    //<trim>
    let size_limit = Arg::new("trim_limit")
        .long("limit")
        .short('l')
        .help("size that the cache will be reduced to, for example: '6B', '1K', '4M', '5G' or '1T'")
        .takes_value(true)
        .value_name("LIMIT")
        .required(true);

    let trim = App::new("trim")
        .about("trim old items from the cache until maximum cache size limit is reached")
        .arg(&size_limit)
        .arg(&dry_run);

    // </trim>
    let toolchain = App::new("toolchain").about("print stats on installed toolchains");

    // <verify>

    let clean_corrupted = Arg::new("clean-corrupted")
        .long("clean-corrupted")
        .short('c')
        .help("automatically remove corrupted cache entries");

    let verify = App::new("verify")
        .about("verify crate sources")
        .arg(&dry_run)
        .arg(&clean_corrupted);

    // </verify>

    // now thread all of these together

    // subcommand hack to have "cargo cache --foo" and "cargo-cache --foo" work equally
    // "cargo cache foo" works because cargo, since it does not implement the "cache" subcommand
    // itself will look if there is a "cargo-cache" binary and exec that
    let cache_subcmd = App::new("cache")
        .version(&*version_string)
        .bin_name("cargo-cache")
        .about("Manage cargo cache")
        .author("matthiaskrgr")
        // todo: remove all these clones once clap allows it
        .subcommand(query.clone())
        .subcommand(query_short.clone())
        .subcommand(local.clone())
        .subcommand(local_short.clone())
        .subcommand(version_subcmd.clone())
        .subcommand(registry.clone())
        .subcommand(registry_short.clone())
        .subcommand(registries_hidden.clone())
        .subcommand(sccache.clone())
        .subcommand(sccache_short.clone())
        .subcommand(clean_unref.clone())
        .subcommand(toolchain.clone())
        .subcommand(trim.clone())
        .subcommand(verify.clone())
        .arg(&list_dirs)
        .arg(&remove_dir)
        .arg(&gc_repos)
        .arg(&fsck_repos)
        .arg(&info)
        .arg(&keep_duplicate_crates)
        .arg(&dry_run)
        .arg(&autoclean)
        .arg(&autoclean_expensive)
        .arg(&list_top_cache_items)
        .arg(&remove_if_younger)
        .arg(&remove_if_older)
        .arg(&debug)
        .setting(AppSettings::Hidden);

    App::new("cargo-cache")
        .version(&*version_string)
        .bin_name("cargo")
        .about("Manage cargo cache")
        .author("matthiaskrgr")
        .subcommand(cache_subcmd)
        .subcommand(query)
        .subcommand(query_short)
        .subcommand(local)
        .subcommand(local_short)
        .subcommand(version_subcmd)
        .subcommand(registry)
        .subcommand(registry_short)
        .subcommand(registries_hidden)
        .subcommand(sccache)
        .subcommand(sccache_short)
        .subcommand(clean_unref)
        .subcommand(toolchain)
        .subcommand(trim)
        .subcommand(verify)
        .arg(&list_dirs)
        .arg(&remove_dir)
        .arg(&gc_repos)
        .arg(&fsck_repos)
        .arg(&info)
        .arg(&keep_duplicate_crates)
        .arg(&dry_run)
        .arg(&autoclean)
        .arg(&autoclean_expensive)
        .arg(&list_top_cache_items)
        .arg(&remove_if_younger)
        .arg(&remove_if_older)
        .arg(&debug)
        .get_matches()
}

#[cfg(test)]
mod clitests {
    use crate::test_helpers::bin_path;
    use pretty_assertions::assert_eq;
    use rustc_tools_util::*;
    use std::process::Command;

    #[test]
    fn run_help() {
        let cc_help = Command::new(bin_path()).arg("--help").output();
        assert!(
            cc_help.is_ok(),
            "cargo-cache --help failed: '{:?}'",
            cc_help
        );
        let help_real = String::from_utf8_lossy(&cc_help.unwrap().stdout).into_owned();
        // DEBUG:
        //eprintln!("{}", help_real);

        let mut help_desired = rustc_tools_util::get_version_info!().to_string();
        help_desired.push_str(
            "\nmatthiaskrgr
Manage cargo cache

USAGE:
    cargo [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -a, --autoclean
            Removes crate source checkouts and git repo checkouts

    -e, --autoclean-expensive
            As --autoclean, but also recompresses git repositories

    -f, --fsck
            Fsck git repositories

    -g, --gc
            Recompress git repositories (may take some time)

    -h, --help
            Print help information

    -i, --info
            Print information cache directories, what they are for and what can be safely deleted

    -k, --keep-duplicate-crates <N>
            Remove all but N versions of crate in the source archives directory

    -l, --list-dirs
            List all found directory paths

    -n, --dry-run
            Don't remove anything, just pretend

    -o, --remove-if-older-than <date>
            Removes items older than specified date: YYYY.MM.DD or HH:MM:SS

    -r, --remove-dir <dir1,dir2,dir3>
            Remove directories, accepted values: all,git-db,git-repos,
            registry-sources,registry-crate-cache,registry-index,registry

    -t, --top-cache-items <N>
            List the top N items taking most space in the cache

    -V, --version
            Print version information

    -y, --remove-if-younger-than <date>
            Removes items younger than the specified date: YYYY.MM.DD or HH:MM:SS

SUBCOMMANDS:
    clean-unref    remove crates that are not referenced in a Cargo.toml from the cache
    help           Print this message or the help of the given subcommand(s)
    l              check local build cache (target) of a rust project
    local          check local build cache (target) of a rust project
    q              run a query
    query          run a query
    r              query each package registry separately
    registry       query each package registry separately
    sc             gather stats on a local sccache cache
    sccache        gather stats on a local sccache cache
    toolchain      print stats on installed toolchains
    trim           trim old items from the cache until maximum cache size limit is reached
    verify         verify crate sources\n",
        );
        assert_eq!(help_desired, help_real);
    }
    #[test]
    fn run_help_subcommand() {
        let cc_help = Command::new(bin_path()).arg("cache").arg("--help").output();
        assert!(
            cc_help.is_ok(),
            "cargo-cache --help failed: '{:?}'",
            cc_help
        );
        let help_real = String::from_utf8_lossy(&cc_help.unwrap().stdout).into_owned();
        // eprintln!("{}", help_real);

        let mut help_desired = rustc_tools_util::get_version_info!().to_string();
        help_desired.push_str(
            "\nmatthiaskrgr
Manage cargo cache

USAGE:
    cargo cache [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -a, --autoclean
            Removes crate source checkouts and git repo checkouts

    -e, --autoclean-expensive
            As --autoclean, but also recompresses git repositories

    -f, --fsck
            Fsck git repositories

    -g, --gc
            Recompress git repositories (may take some time)

    -h, --help
            Print help information

    -i, --info
            Print information cache directories, what they are for and what can be safely deleted

    -k, --keep-duplicate-crates <N>
            Remove all but N versions of crate in the source archives directory

    -l, --list-dirs
            List all found directory paths

    -n, --dry-run
            Don't remove anything, just pretend

    -o, --remove-if-older-than <date>
            Removes items older than specified date: YYYY.MM.DD or HH:MM:SS

    -r, --remove-dir <dir1,dir2,dir3>
            Remove directories, accepted values: all,git-db,git-repos,
            registry-sources,registry-crate-cache,registry-index,registry

    -t, --top-cache-items <N>
            List the top N items taking most space in the cache

    -V, --version
            Print version information

    -y, --remove-if-younger-than <date>
            Removes items younger than the specified date: YYYY.MM.DD or HH:MM:SS

SUBCOMMANDS:
    clean-unref    remove crates that are not referenced in a Cargo.toml from the cache
    help           Print this message or the help of the given subcommand(s)
    l              check local build cache (target) of a rust project
    local          check local build cache (target) of a rust project
    q              run a query
    query          run a query
    r              query each package registry separately
    registry       query each package registry separately
    sc             gather stats on a local sccache cache
    sccache        gather stats on a local sccache cache
    toolchain      print stats on installed toolchains
    trim           trim old items from the cache until maximum cache size limit is reached
    verify         verify crate sources\n",
        );

        assert_eq!(help_desired, help_real);
    }

    #[test]
    fn run_help_query() {
        let ccq_help = Command::new(bin_path())
            .arg("cache")
            .arg("query")
            .arg("--help")
            .output();
        assert!(
            ccq_help.is_ok(),
            "cargo-cache query --help failed: '{:?}'",
            ccq_help
        );
        let help_real = String::from_utf8_lossy(&ccq_help.unwrap().stdout).into_owned();

        //eprintln!("{}", help_real);

        let mut help_desired = String::new();
        help_desired.push_str(
"cargo-cache-query 
run a query

USAGE:
    cargo cache query [OPTIONS] [QUERY]

ARGS:
    <QUERY>    

OPTIONS:
    -h, --help              Print help information
        --human-readable    print sizes in human readable format
    -s, --sort-by <sort>    sort files alphabetically or by file size [possible values: size, name]\n"
        );

        assert_eq!(help_desired, help_real);
    }

    #[test]
    fn all_versions_are_equal() {
        let v1 = Command::new(bin_path()).arg("-V").output().unwrap().stdout;
        let v2 = Command::new(bin_path())
            .arg("cache")
            .arg("-V")
            .output()
            .unwrap()
            .stdout;
        let v3 = Command::new(bin_path())
            .arg("--version")
            .output()
            .unwrap()
            .stdout;
        let v4 = Command::new(bin_path())
            .arg("version")
            .output()
            .unwrap()
            .stdout;

        let v1_s = String::from_utf8_lossy(&v1).into_owned();
        let v2_s = String::from_utf8_lossy(&v2).into_owned();
        let v3_s = String::from_utf8_lossy(&v3).into_owned();
        let v4_s = String::from_utf8_lossy(&v4).into_owned();

        assert!(
            v1_s == v2_s && v2_s == v3_s && v3_s == v4_s,
            "version outputs do not match!\n v1 {}\nv2 {}\nv3 {}\nv4 {}",
            v1_s,
            v2_s,
            v3_s,
            v4_s
        );
    }

    #[test]
    fn bare_dry_run_warns() {
        let cc_dryrun = Command::new(bin_path())
            .arg("cache")
            .arg("--dry-run")
            .output();
        assert!(
            cc_dryrun.is_ok(),
            "cargo-cache --dry-run failed: '{:?}'",
            cc_dryrun
        );

        let stderr = String::from_utf8_lossy(&cc_dryrun.unwrap().stderr).into_owned();
        let last_line = stderr.lines().last();
        // last line must be this warning:
        assert_eq!(last_line, Some("Warning: there is nothing to be dry run!"));
    }
}

#[cfg(all(test, feature = "bench"))]
mod benchmarks {
    use crate::test::black_box;
    use crate::test::Bencher;
    use crate::test_helpers::bin_path;
    use std::process::Command;

    #[bench]
    fn bench_clap_help(b: &mut Bencher) {
        #[allow(unused_must_use)]
        b.iter(|| {
            let x = Command::new(bin_path()).arg("--help").output();
            black_box(x);
        });
    }

    #[bench]
    fn bench_clap_help_subcommand(b: &mut Bencher) {
        #[allow(unused_must_use)]
        b.iter(|| {
            let x = Command::new(bin_path()).arg("cache").arg("--help").output();
            black_box(x);
        });
    }
}
