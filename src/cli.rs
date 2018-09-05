use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

pub(crate) fn gen_clap<'a>() -> ArgMatches<'a> {
    let version = include_str!(concat!(env!("OUT_DIR"), "/commit-info.txt"));

    let list_dirs = Arg::with_name("list-dirs")
        .short("l")
        .long("list-dirs")
        .help("List all found directory paths");

    let remove_dir = Arg::with_name("remove-dir").short("r").long("remove-dir")
        .help("Remove directories, accepted values: git-db,git-repos,registry-sources,registry-crate-cache,registry,all")
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

    App::new("cargo-cache")
        .version(&*version)
        .bin_name("cargo")
        .about("Manage cargo cache")
        .author("matthiaskrgr")
        .subcommand(
            SubCommand::with_name("cache")
                .version(&*version)
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
                .arg(&autoclean_expensive)
                .arg(&list_top_cache_items)
                .setting(AppSettings::Hidden),
        ) // subcommand
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
    use crate::version;
    use pretty_assertions::assert_eq;
    use std::process::Command;
    use test::Bencher;

    fn cargo_build_release() {
        // build crate in release mode
        let cmd = Command::new("cargo").arg("build").arg("--release").output();
        assert!(cmd.unwrap().status.success());
    }

    fn cargo_build_debug() {
        // build crate in debug mode
        let cmd = Command::new("cargo").arg("build").output();
        assert!(cmd.unwrap().status.success());
    }

    #[test]
    fn run_help() {
        cargo_build_debug();

        let cc_help = Command::new("target/debug/cargo-cache")
            .arg("--help")
            .output();
        assert!(cc_help.is_ok(), "cargo-cache --help failed");
        let help_real = String::from_utf8_lossy(&cc_help.unwrap().stdout).into_owned();
        let mut help_desired = format!("cargo-cache {}", version::VersionInfo::new());
        help_desired.push_str("
matthiaskrgr
Manage cargo cache\n
USAGE:
    cargo [FLAGS] [OPTIONS]\n
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
    -r, --remove-dir <dir1,dir2,dir3>    Remove directories, accepted values: git-db,git-repos,registry-
                                         sources,registry-crate-cache,registry,all
    -t, --top-cache-items <N>            List the top N items taking most space in the cache\n");

        assert_eq!(help_desired, help_real);
    }
    #[test]
    fn run_help_subcommand() {
        cargo_build_debug();

        let cc_help = Command::new("target/debug/cargo-cache")
            .arg("cache")
            .arg("--help")
            .output();
        assert!(cc_help.is_ok(), "cargo-cache --help failed");
        let help_real = String::from_utf8_lossy(&cc_help.unwrap().stdout).into_owned();
        let mut help_desired = format!("cargo-cache {}", version::VersionInfo::new());
        help_desired.push_str("
matthiaskrgr
Manage cargo cache\n
USAGE:
    cargo cache [FLAGS] [OPTIONS]\n
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
    -r, --remove-dir <dir1,dir2,dir3>    Remove directories, accepted values: git-db,git-repos,registry-
                                         sources,registry-crate-cache,registry,all
    -t, --top-cache-items <N>            List the top N items taking most space in the cache\n");

        assert_eq!(help_desired, help_real);
    }

    #[bench]
    fn bench_clap_help(b: &mut Bencher) {
        cargo_build_release();
        b.iter(|| {
            Command::new("target/release/cargo-cache")
                .arg("--help")
                .output()
        })
    }

    #[bench]
    fn bench_clap_help_subcommand(b: &mut Bencher) {
        cargo_build_release();
        b.iter(|| {
            Command::new("target/release/cargo-cache")
                .arg("cache")
                .arg("--help")
                .output()
        })
    }

}
