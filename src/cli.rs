use clap::{crate_version, App, Arg, ArgMatches, SubCommand};

pub(crate) fn gen_clap<'a>() -> ArgMatches<'a> {
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

    App::new("cargo-cache")
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
                .arg(&autoclean_expensive)
        ) // subcommand
        .arg(&list_dirs)
        .arg(&remove_dir)
        .arg(&gc_repos)
        .arg(&info)
        .arg(&keep_duplicate_crates)
        .arg(&dry_run)
        .arg(&autoclean)
        .arg(&autoclean_expensive)
        .get_matches()
}

#[cfg(test)]
mod clitests {
    use super::*;
    use std::process::Command;

    #[test]
    fn run_help() {
        // build
        let status = Command::new("cargo").arg("build").output();
        // make sure the build succeeded
        assert!(status.is_ok(), "cargo build did not succeed");
        let cc_help = Command::new("target/debug/cargo-cache")
            .arg("--help")
            .output();
        assert!(cc_help.is_ok(), "cargo-cache --help failed");
        let help_real = String::from_utf8_lossy(&cc_help.unwrap().stdout).into_owned();
        let help_desired = "cargo-cache 0.1.0
matthiaskrgr
Manage cargo cache\n
USAGE:
    cargo [FLAGS] [OPTIONS] [SUBCOMMAND]\n
FLAGS:
    -a, --autoclean              Removes registry src checkouts and git repo checkouts
    -e, --autoclean-expensive    Removes registry src checkouts, git repo checkouts and gcs repos
    -d, --dry-run                Don't remove anything, just pretend
    -g, --gc                     Recompress git repositories (may take some time).
    -h, --help                   Prints help information
    -i, --info                   Give information on directories
    -l, --list-dirs              List found directory paths.
    -V, --version                Prints version information\n
OPTIONS:
    -k, --keep-duplicate-crates <N>      Remove all but N versions of duplicate crates in the source cache
    -r, --remove-dir <dir1,dir2,dir3>    Remove directories, accepted values: git-db,git-repos,registry-
                                         sources,registry-crate-cache,registry,all\n
SUBCOMMANDS:
    cache    Manage cargo cache
    help     Prints this message or the help of the given subcommand(s)
";

        assert_eq!(help_desired, help_real);
    }
}