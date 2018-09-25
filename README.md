## cargo cache

[![Build Status](https://travis-ci.org/matthiaskrgr/cargo-cache.svg?branch=master)](https://travis-ci.org/matthiaskrgr/cargo-cache)
![Minimum rust version: nightly](https://img.shields.io/badge/Minimum%20Rust%20Version-nightly-red.svg)

Requires ````rust nightly 1.30```` (due to `tool_lints` feature).

DISCLAIMER: I only tested this on linux.

#### Installation:
````
cargo +nightly install --git https://github.com/matthiaskrgr/cargo-cache
````

#### Sample output:
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

#### Usage:
````
USAGE:
    cargo cache [FLAGS] [OPTIONS]
FLAGS:
    -a, --autoclean              Removes crate source checkouts and git repo checkouts
    -e, --autoclean-expensive    As --autoclean, but also recompresses git repositories
    -d, --dry-run                Don't remove anything, just pretend
    -g, --gc                     Recompress git repositories (may take some time)
    -h, --help                   Prints help information
    -i, --info                   Print information on found cache directories
    -l, --list-dirs              List all found directory paths
    -V, --version                Prints version information
OPTIONS:
    -k, --keep-duplicate-crates <N>      Remove all but N versions of crate in the source archives directory
    -r, --remove-dir <dir1,dir2,dir3>    Remove directories, accepted values: git-db,git-repos,registry-
                                         sources,registry-crate-cache,registry,all
    -t, --top-cache-items <N>            List the top N items taking most space in the cache
````
