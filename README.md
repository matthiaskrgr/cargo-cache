Requires rust 1.29 (due to rust 2018 edition usage).

DISCLAIMER: I only tested this on linux.

Sample output:
````
Cargo cache '/home/matthias/.cargo':
Total size:                   2.52 GB
Size of 62 installed binaries:     726.57 MB
Size of registry:                  1.25 GB
Size of registry crate cache:           418.04 MB
Size of registry source checkouts:      770.58 MB
Size of git db:                    276.01 MB
Size of git repo checkouts:        266.36 MB
````

Usage:

````
cargo cache [...]


    cargo [FLAGS] [OPTIONS] [SUBCOMMAND]
FLAGS:
    -a, --autoclean              Removes registry src checkouts and git repo checkouts
    -e, --autoclean-expensive    Removes registry src checkouts, git repo checkouts and gcs repos
    -d, --dry-run                Don't remove anything, just pretend
    -g, --gc                     Recompress git repositories (may take some time).
    -h, --help                   Prints help information
    -i, --info                   Give information on directories
    -l, --list-dirs              List found directory paths.
    -V, --version                Prints version information
OPTIONS:
    -k, --keep-duplicate-crates <N>      Remove all but N versions of duplicate crates in the source cache
    -r, --remove-dir <dir1,dir2,dir3>    Remove directories, accepted values: git-db,git-repos,registry-
                                         sources,registry-crate-cache,registry,all

````
