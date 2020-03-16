// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/// This file provides the command line interface of the cargo-cache crate
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

use rustc_tools_util::*;

/// generates the version info with what we have in the build.rs
pub(crate) fn get_version() -> String {
    // remove the "cargo-cache" since CLAP already adds that by itself
    rustc_tools_util::get_version_info!()
        .to_string()
        .replacen("cargo-cache ", "", 1)
}

/// generates the clap config which is used to control the crate
#[allow(clippy::too_many_lines)]
pub(crate) fn gen_clap<'a>() -> ArgMatches<'a> {
    let version_string = get_version();

    let list_dirs = Arg::with_name("list-dirs")
        .short("l")
        .long("list-dirs")
        .help("List all found directory paths");

    let remove_dir = Arg::with_name("remove-dir").short("r").long("remove-dir")
        .help("Remove directories, accepted values: all,git-db,git-repos,\nregistry-sources,registry-crate-cache,registry-index,registry")
        .takes_value(true)
        .value_name("dir1,dir2,dir3");

    let gc_repos = Arg::with_name("gc-repos")
        .short("g")
        .long("gc")
        .help("Recompress git repositories (may take some time)");

    let fsck_repos = Arg::with_name("fsck-repos")
        .short("f")
        .long("fsck")
        .help("Fsck git repositories");

    let info = Arg::with_name("info")
        .short("i")
        .long("info")
        .conflicts_with("list-dirs")
        .help(
            "Print information cache directories, what they are for and what can be safely deleted",
        );

    let keep_duplicate_crates = Arg::with_name("keep-duplicate-crates")
        .short("k")
        .long("keep-duplicate-crates")
        .help("Remove all but N versions of crate in the source archives directory")
        .takes_value(true)
        .value_name("N");

    let dry_run = Arg::with_name("dry-run")
        .short("d")
        .long("dry-run")
        .help("Don't remove anything, just pretend");

    let autoclean = Arg::with_name("autoclean")
        .short("a")
        .long("autoclean")
        .help("Removes crate source checkouts and git repo checkouts");

    let autoclean_expensive = Arg::with_name("autoclean-expensive")
        .short("e")
        .long("autoclean-expensive")
        .help("As --autoclean, but also recompresses git repositories");

    let list_top_cache_items = Arg::with_name("top-cache-items")
        .short("t")
        .long("top-cache-items")
        .help("List the top N items taking most space in the cache")
        .takes_value(true)
        .value_name("N");

    let remove_if_older = Arg::with_name("remove-if-older-than").short("o").long("remove-if-older-than")
        .help("Removes items older than specified date: YYYY.MM.DD or HH:MM:SS or YYYY.MM.DD HH::MM::SS")
        .conflicts_with("remove-if-younger-than")  // fix later
        .requires("remove-dir")
        .takes_value(true)
        .value_name("date");

    let remove_if_younger = Arg::with_name("remove-if-younger-than").short("y").long("remove-if-younger-than")
        .help("Removes items younger than the specified date: YYYY.MM.DD or HH:MM:SS or YYYY.MM.DD HH::MM::SS")
        .conflicts_with("remove-if-older-than") // fix later
        .requires("remove-dir")
        .takes_value(true)
        .value_name("date");

    let debug = Arg::with_name("debug")
        .long("debug")
        .help("print some debug stats")
        .hidden(true);

    // <query>
    // arg of query sbcmd
    let query_order = Arg::with_name("sort")
        .short("s")
        .long("sort-by")
        .help("sort files alphabetically or by file size")
        .takes_value(true)
        .possible_values(&["size", "name"]);

    // arg of query sbcmd
    let human_readable = Arg::with_name("hr")
        .short("h")
        .long("human-readable")
        .help("print sizes in human readable format");

    // query subcommand to allow querying
    let query = SubCommand::with_name("query")
        .about("run a query")
        .arg(Arg::with_name("QUERY"))
        .arg(&query_order)
        .arg(&human_readable);

    // short q (shorter query sbcmd)
    let query_short = SubCommand::with_name("q")
        .about("run a query")
        .arg(Arg::with_name("QUERY"))
        .arg(&query_order)
        .arg(&human_readable);
    // </query>

    //<local>
    // local subcommand
    let local =
        SubCommand::with_name("local").about("check local build cache (target) of a rust project");
    // shorter local subcommand (l)
    let local_short =
        SubCommand::with_name("l").about("check local build cache (target) of a rust project");
    //</local>

    // <registry>
    // registry subcommand
    let registry =
        SubCommand::with_name("registry").about("query each package registry separately");
    let registry_short = SubCommand::with_name("r").about("query each package registry separately");
    // hidden, but have "cargo cache registries" work too
    let registries_hidden = SubCommand::with_name("registries")
        .about("query each package registry separately")
        .settings(&[AppSettings::Hidden]);
    //</registry>

    // "version" subcommand which is also hidden, prints crate version
    let version_subcmd = SubCommand::with_name("version").settings(&[AppSettings::Hidden]);

    // now thread all of these together

    // subcommand hack to have "cargo cache --foo" and "cargo-cache --foo" work equally
    // "cargo cache foo" works because cargo, since it does not implement the "cache" subcommand
    // itself will look if there is a "cargo-cache" binary and exec that
    let cache_subcmd = SubCommand::with_name("cache")
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

        let mut help_desired = rustc_tools_util::get_version_info!().to_string();
        help_desired.push_str("
matthiaskrgr
Manage cargo cache\n
USAGE:
    cargo [FLAGS] [OPTIONS] [SUBCOMMAND]\n
FLAGS:
    -a, --autoclean              Removes crate source checkouts and git repo checkouts
    -e, --autoclean-expensive    As --autoclean, but also recompresses git repositories
    -d, --dry-run                Don't remove anything, just pretend
    -f, --fsck                   Fsck git repositories
    -g, --gc                     Recompress git repositories (may take some time)
    -h, --help                   Prints help information
    -i, --info                   Print information cache directories, what they are for and what can be safely deleted
    -l, --list-dirs              List all found directory paths
    -V, --version                Prints version information\n
OPTIONS:
    -k, --keep-duplicate-crates <N>        Remove all but N versions of crate in the source archives directory
    -r, --remove-dir <dir1,dir2,dir3>      Remove directories, accepted values: all,git-db,git-repos,
                                           registry-sources,registry-crate-cache,registry-index,registry
    -o, --remove-if-older-than <date>      Removes items older than specified date: YYYY.MM.DD or HH:MM:SS or YYYY.MM.DD
                                           HH::MM::SS
    -y, --remove-if-younger-than <date>    Removes items younger than the specified date: YYYY.MM.DD or HH:MM:SS or
                                           YYYY.MM.DD HH::MM::SS
    -t, --top-cache-items <N>              List the top N items taking most space in the cache\n
SUBCOMMANDS:
    help        Prints this message or the help of the given subcommand(s)
    l           check local build cache (target) of a rust project
    local       check local build cache (target) of a rust project
    q           run a query
    query       run a query
    r           query each package registry separately
    registry    query each package registry separately\n");

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

        let mut help_desired = rustc_tools_util::get_version_info!().to_string();
        help_desired.push_str("
matthiaskrgr
Manage cargo cache\n
USAGE:
    cargo cache [FLAGS] [OPTIONS] [SUBCOMMAND]\n
FLAGS:
    -a, --autoclean              Removes crate source checkouts and git repo checkouts
    -e, --autoclean-expensive    As --autoclean, but also recompresses git repositories
    -d, --dry-run                Don't remove anything, just pretend
    -f, --fsck                   Fsck git repositories
    -g, --gc                     Recompress git repositories (may take some time)
    -h, --help                   Prints help information
    -i, --info                   Print information cache directories, what they are for and what can be safely deleted
    -l, --list-dirs              List all found directory paths
    -V, --version                Prints version information\n
OPTIONS:
    -k, --keep-duplicate-crates <N>        Remove all but N versions of crate in the source archives directory
    -r, --remove-dir <dir1,dir2,dir3>      Remove directories, accepted values: all,git-db,git-repos,
                                           registry-sources,registry-crate-cache,registry-index,registry
    -o, --remove-if-older-than <date>      Removes items older than specified date: YYYY.MM.DD or HH:MM:SS or YYYY.MM.DD
                                           HH::MM::SS
    -y, --remove-if-younger-than <date>    Removes items younger than the specified date: YYYY.MM.DD or HH:MM:SS or
                                           YYYY.MM.DD HH::MM::SS
    -t, --top-cache-items <N>              List the top N items taking most space in the cache\n
SUBCOMMANDS:
    help        Prints this message or the help of the given subcommand(s)
    l           check local build cache (target) of a rust project
    local       check local build cache (target) of a rust project
    q           run a query
    query       run a query
    r           query each package registry separately
    registry    query each package registry separately\n");

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

        let mut help_desired = String::new();
        help_desired.push_str(
            "cargo-cache-query 
run a query

USAGE:
    cargo cache query [FLAGS] [OPTIONS] [QUERY]

FLAGS:
        --help              Prints help information
    -h, --human-readable    print sizes in human readable format
    -V, --version           Prints version information

OPTIONS:
    -s, --sort-by <sort>    sort files alphabetically or by file size [possible values: size, name]

ARGS:
    <QUERY>    \n",
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
