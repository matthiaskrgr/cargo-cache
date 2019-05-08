// Copyright 2017-2019 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

use rustc_tools_util::*;

#[allow(clippy::too_many_lines)]
pub(crate) fn gen_clap<'a>() -> ArgMatches<'a> {
    let version = rustc_tools_util::get_version_info!()
        .to_string()
        .replacen("cargo-cache ", "", 1);

    let list_dirs = Arg::with_name("list-dirs")
        .short("l")
        .long("list-dirs")
        .help("List all found directory paths");

    let remove_dir = Arg::with_name("remove-dir").short("r").long("remove-dir")
        .help("Remove directories, accepted values: git-db,git-repos,\nregistry-sources,registry-crate-cache,registry-index,registry,all")
        .takes_value(true)
        .value_name("dir1,dir2,dir3");

    let gc_repos = Arg::with_name("gc-repos")
        .short("g")
        .long("gc")
        .help("Recompress git repositories (may take some time)");

    let info = Arg::with_name("info")
        .short("i")
        .long("info")
        .conflicts_with("list-dirs")
        .help("Print information on found cache directories");

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

    // <query>
    let query_order = Arg::with_name("sort")
        .short("s")
        .long("sort-by")
        .help("sort files alphabetically or by file size")
        .takes_value(true)
        .possible_values(&["size", "name"]);

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

    // short q
    let query_short = SubCommand::with_name("q")
        .about("run a query")
        .arg(Arg::with_name("QUERY"))
        .arg(&query_order)
        .arg(&human_readable);
    // </query>

    //<local>

    // subcommand
    let local =
        SubCommand::with_name("local").about("check local build cache (target) of a rust project");

    let local_short =
        SubCommand::with_name("l").about("check local build cache (target) of a rust project");
    //</local>

    // subcommand hack to have "cargo cache --foo" and "cargo-cache --foo" work equally
    let cache_subcmd = SubCommand::with_name("cache")
        .version(&*version)
        .bin_name("cargo-cache")
        .about("Manage cargo cache")
        .author("matthiaskrgr")
        .subcommand(query.clone()) // todo: don't clone
        .subcommand(query_short.clone()) // todo: don't clone
        .subcommand(local.clone()) // don't clone
        .subcommand(local_short.clone()) // don't clone
        .arg(&list_dirs)
        .arg(&remove_dir)
        .arg(&gc_repos)
        .arg(&info)
        .arg(&keep_duplicate_crates)
        .arg(&dry_run)
        .arg(&autoclean)
        .arg(&autoclean_expensive)
        .arg(&list_top_cache_items)
        .setting(AppSettings::Hidden);

    App::new("cargo-cache")
        .version(&*version)
        .bin_name("cargo")
        .about("Manage cargo cache")
        .author("matthiaskrgr")
        .subcommand(cache_subcmd)
        .subcommand(query)
        .subcommand(query_short)
        .subcommand(local)
        .subcommand(local_short)
        .arg(&list_dirs)
        .arg(&remove_dir)
        .arg(&gc_repos)
        .arg(&info)
        .arg(&keep_duplicate_crates)
        .arg(&dry_run)
        .arg(&autoclean)
        .arg(&autoclean_expensive)
        .arg(&list_top_cache_items)
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
    -g, --gc                     Recompress git repositories (may take some time)
    -h, --help                   Prints help information
    -i, --info                   Print information on found cache directories
    -l, --list-dirs              List all found directory paths
    -V, --version                Prints version information\n
OPTIONS:
    -k, --keep-duplicate-crates <N>      Remove all but N versions of crate in the source archives directory
    -r, --remove-dir <dir1,dir2,dir3>    Remove directories, accepted values: git-db,git-repos,
                                         registry-sources,registry-crate-cache,registry-index,registry,all
    -t, --top-cache-items <N>            List the top N items taking most space in the cache\n
SUBCOMMANDS:
    help     Prints this message or the help of the given subcommand(s)
    l        check local build cache (target) of a rust project
    local    check local build cache (target) of a rust project
    q        run a query
    query    run a query\n");

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
    -g, --gc                     Recompress git repositories (may take some time)
    -h, --help                   Prints help information
    -i, --info                   Print information on found cache directories
    -l, --list-dirs              List all found directory paths
    -V, --version                Prints version information\n
OPTIONS:
    -k, --keep-duplicate-crates <N>      Remove all but N versions of crate in the source archives directory
    -r, --remove-dir <dir1,dir2,dir3>    Remove directories, accepted values: git-db,git-repos,
                                         registry-sources,registry-crate-cache,registry-index,registry,all
    -t, --top-cache-items <N>            List the top N items taking most space in the cache\n
SUBCOMMANDS:
    help     Prints this message or the help of the given subcommand(s)
    l        check local build cache (target) of a rust project
    local    check local build cache (target) of a rust project
    q        run a query
    query    run a query\n");

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
