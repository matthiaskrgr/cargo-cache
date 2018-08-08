Requires rust 1.29 (due to rust 2018 edition usage).

DISCLAIMER: I only tested this on linux.

Sample output:
````
Cargo cache '/home/matthias/.cargo':

Total size:                             2.06 GB
Size of 62 installed binaries:            722.49 MB
Size of registry:                         950.39 MB
Size of 3022 crate archives:                440.57 MB
Size of 621 crate source checkouts:         445.13 MB
Size of git db:                           387.48 MB
Size of 95 bare git repos:                  377.87 MB
Size of 4 git repo checkouts:               9.61 MB
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
