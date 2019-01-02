## cargo cache

[![Build Status](https://travis-ci.org/matthiaskrgr/cargo-cache.svg?branch=master)](https://travis-ci.org/matthiaskrgr/cargo-cache)
[![dependency status](https://deps.rs/repo/github/matthiaskrgr/cargo-cache/status.svg)](https://deps.rs/repo/github/matthiaskrgr/cargo-cache)
[![Latest Version](https://img.shields.io/crates/v/cargo-cache.svg)](https://crates.io/crates/cargo-cache)

Display information on the cargo cache `~/.cargo/`. Optional cache pruning.

`stable`, `beta` and `nightly` channels are supported.

#### Installation:
```cargo install cargo-cache```

or for the bleeding edge development version:

```cargo install --git https://github.com/matthiaskrgr/cargo-cache```


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
    -L, --list-dirs              List all found directory paths
    -V, --version                Prints version information
OPTIONS:
    -k, --keep-duplicate-crates <N>      Remove all but N versions of crate in the source archives directory
    -r, --remove-dir <dir1,dir2,dir3>    Remove directories, accepted values: git-db,git-repos,registry-
                                         sources,registry-crate-cache,registry,all
    -t, --top-cache-items <N>            List the top N items taking most space in the cache
````

#### Show the largest items in the cache:
````
cargo cache --top-cache-items 5

Summary of: /home/matthias/.cargo/bin/ (588.35 MB total)
Name         Size
alacritty    38.40 MB
xsv          29.78 MB
rg           28.51 MB
cargo-geiger 15.11 MB
mdbook       12.39 MB

Summary of: /home/matthias/.cargo/registry/src/ (3.11 GB total)
Name                         Count Average   Total
mozjs_sys                    4     131.83 MB 527.31 MB
wabt-sys                     2     83.73 MB  167.46 MB
openblas-src                 2     78.42 MB  156.84 MB
curl-sys                     6     18.47 MB  110.83 MB
winapi-x86_64-pc-windows-gnu 2     54.90 MB  109.80 MB

Summary of: /home/matthias/.cargo/registry/cache/ (1.18 GB total)
Name        Count Average  Total
mozjs_sys   10    29.45 MB 294.50 MB
curl-sys    16    3.03 MB  48.54 MB
libgit2-sys 18    2.54 MB  45.64 MB
servo-skia  6     5.23 MB  31.39 MB
openssl-src 5     5.55 MB  27.73 MB

Summary of: /home/matthias/.cargo/git/db/ (918.97 MB total)
Name         Count Average   Total
polonius     1     136.63 MB 136.63 MB
mdbook       1     111.45 MB 111.45 MB
rust-rocksdb 2     33.31 MB  66.62 MB
osmesa-src   2     28.45 MB  56.90 MB
ring         2     23.02 MB  46.04 MB

Summary of: /home/matthias/.cargo/git/checkouts/ (3.80 GB total)
Name            Count Average   Total
parity-ethereum 2     666.36 MB 1.33 GB
xori            1     372.69 MB 372.69 MB
polonius        2     186.34 MB 372.67 MB
alacritty       9     39.08 MB  351.74 MB
osmesa-src      2     166.12 MB 332.24 MB
````
#### Do a light cleanup
This removes extracted tarball sources and repository checkouts.
The original source archives and git repos are kept and will be extracted as needed by cargo.
````
cargo cache --autoclean

Cargo cache '/home/matthias/.cargo/':

Total size:                             9.61 GB
Size of 84 installed binaries:            588.35 MB
Size of registry:                         4.31 GB
Size of 6136 crate archives:                1.18 GB
Size of 3738 crate source checkouts:        3.11 GB
Size of git db:                           4.72 GB
Size of 172 bare git repos:                 918.98 MB
Size of 138 git repo checkouts:             3.80 GB

Size changed from 9.61 GB to 2.71 GB (-6.90 GB, -71.8%)
````
Checking the sizes after cleanup:
````
Total size:                             2.78 GB
Size of 84 installed binaries:            588.35 MB
Size of registry:                         1.27 GB
Size of 6136 crate archives:                1.18 GB
Size of 0 crate source checkouts:           0 B
Size of git db:                           918.98 MB
Size of 172 bare git repos:                 918.98 MB
Size of 0 git repo checkouts:               0 B
````

#### License:

Copyright 2017-2019 Matthias Krüger

Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
<LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
option. All files in the project carrying such notice may not be
copied, modified, or distributed except according to those terms.
