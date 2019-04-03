## cargo cache

[![Build Status](https://travis-ci.org/matthiaskrgr/cargo-cache.svg?branch=master)](https://travis-ci.org/matthiaskrgr/cargo-cache)
[![dependency status](https://deps.rs/repo/github/matthiaskrgr/cargo-cache/status.svg)](https://deps.rs/repo/github/matthiaskrgr/cargo-cache)
[![Latest Version](https://img.shields.io/crates/v/cargo-cache.svg)](https://crates.io/crates/cargo-cache)
[![Crates.rs](https://img.shields.io/badge/crates.rs-gray.svg)](https://crates.rs/crates/cargo-cache)

Display information on the cargo cache `~/.cargo/`. Optional cache pruning.

`stable`, `beta` and `nightly` channels are supported.

#### Installation:
```cargo install cargo-cache```

or for the bleeding edge development version:

```cargo install --git https://github.com/matthiaskrgr/cargo-cache```


#### Sample output:
````
Cargo cache '/home/matthias/.cargo':

Total size:                             3.77 GB
Size of 87 installed binaries:            558.30 MB
Size of registry:                         1.97 GB
Size of registry index:                     44.32 MB
Size of 6639 crate archives:                1.24 GB
Size of 1312 crate source checkouts:        688.91 MB
Size of git db:                           1.24 GB
Size of 172 bare git repos:                 1.03 GB
Size of 13 git repo checkouts:              210.87 MB
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

Total size:                             3.77 GB
Size of 87 installed binaries:            558.30 MB
Size of registry:                         1.97 GB
Size of registry index:                     44.32 MB
Size of 6639 crate archives:                1.24 GB
Size of 1312 crate source checkouts:        688.91 MB
Size of git db:                           1.24 GB
Size of 172 bare git repos:                 1.03 GB
Size of 13 git repo checkouts:              210.87 MB

Size changed from 3.77 GB to 2.87 GB (-899.78 MB, -23.87%)
````
Checking the sizes after cleanup:
````
Total size:                             2.87 GB
Size of 87 installed binaries:            558.30 MB
Size of registry:                         1.28 GB
Size of registry index:                     44.32 MB
Size of 6639 crate archives:                1.24 GB
Size of 0 crate source checkouts:           0 B
Size of git db:                           1.03 GB
Size of 172 bare git repos:                 1.03 GB
Size of 0 git repo checkouts:               0 B
````

#### License:

Copyright 2017-2019 Matthias Krüger

Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
<LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
option. All files in the project carrying such notice may not be
copied, modified, or distributed except according to those terms.
