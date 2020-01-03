## cargo cache

[![Build Status](https://github.com/matthiaskrgr/cargo-cache/workflows/ci/badge.svg)](https://github.com/matthiaskrgr/cargo-cache/actions)
[![dependency status](https://deps.rs/repo/github/matthiaskrgr/cargo-cache/status.svg)](https://deps.rs/repo/github/matthiaskrgr/cargo-cache)
[![Latest Version](https://img.shields.io/crates/v/cargo-cache.svg)](https://crates.io/crates/cargo-cache)
[![libs.rs](https://img.shields.io/badge/libs.rs-gray.svg)](https://lib.rs/crates/cargo-cache)

Display information on the cargo cache (`~/.cargo/` or `$CARGO_HOME`). Optional cache pruning.


![Screenshot of cargo cache default output (it's listed below also in textual form)](data/screenshot_readme_f724ec8.png?raw=true "Cargo Cache")

#### Key Features:
* check the size of the cargo cache and its components (cmd: `cargo cache`)
* do a simple cleanup removing checkouts but keeping original files needed for reconstruction on disk (`--autoclean`)
* clean up everything (cargo will re-download as needed)
* dry-run to see what would be removed (`--dry-run`)
* recompress git repos (`--gc`)
* search cache via regex queries (`cargo cache query`)
* print crates that take the most space (`--top-cache-items`)
* alternative registries supported
* builds and runs on `stable`, `beta` and `nightly` channel

#### Installation:
```cargo install cargo-cache```

or for the bleeding edge development version:

```cargo install --git https://github.com/matthiaskrgr/cargo-cache cargo-cache```

#### Default output (`cargo cache`):
This only calculates the sizes and does not touch anything:
````
Cargo cache '/home/matthias/.cargo':

Total:                                4.22 GB
  102 installed binaries:           920.95 MB
  Registry:                           2.25 GB
    Registry index:                 227.07 MB
    4412 crate archives:            684.29 MB
    2411 crate source checkouts:      1.34 GB
  Git db:                             1.05 GB
    113 bare git repos:             993.72 MB
    9 git repo checkouts:            55.48 MB
````
To learn more about the subdirectories inside the cargo home and what can be safely deleted, check `--info`.


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
    -i, --info                   Print information cache directories, what they are for and what can be safely deleted
    -l, --list-dirs              List all found directory paths
    -V, --version                Prints version information
OPTIONS:
    -k, --keep-duplicate-crates <N>      Remove all but N versions of crate in the source archives directory
    -r, --remove-dir <dir1,dir2,dir3>    Remove directories, accepted values: git-db,git-repos,registry-
                                         sources,registry-crate-cache,registry,all
    -t, --top-cache-items <N>            List the top N items taking most space in the cache
````

#### Show the largest items in the cargo home:
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

Total:                                4.22 GB
  102 installed binaries:           920.95 MB
  Registry:                           2.25 GB
    Registry index:                 227.07 MB
    4412 crate archives:            684.29 MB
    2411 crate source checkouts:      1.34 GB
  Git db:                             1.05 GB
    113 bare git repos:             993.72 MB
    9 git repo checkouts:            55.48 MB

Size changed from 4.22 GB to 2.83 GB (-1.39 GB, -33.02%)
````
Checking the sizes after cleanup:
````
Total:                                2.83 GB
  102 installed binaries:           920.95 MB
  Registry:                         911.36 MB
    Registry index:                 227.07 MB
    4412 crate archives:            684.29 MB
    0 crate source checkouts:            0  B
  Git db:                           993.72 MB
    113 bare git repos:             993.72 MB
    0 git repo checkouts:                0  B
````

The crate also works if you override the default location of the cargo home via
the $CARGO_HOME env var!


Side note: cargo-cache started as my *learning-by-doing* rust project, if you see something that you find very odd or is in dire need of improvement please let me know and open a ticket!

#### Cleaning the cache on CI
Sometimes it is desired to [cache the $CARGO_HOME in CI](https://doc.rust-lang.org/nightly/cargo/guide/cargo-home.html#caching-the-cargo-home-in-ci).
As noted in the document, this might cache sources twice which adds unneccessary overhead.  
To reduce the size of the cache before storing it, you might want to run `cargo cache --autoclean`.
The `ci-autoclean` feature provides a very stripped-down version of the crate that is only capable of running `cargo-cache --autoclean` automatically on launch and should compile within a couple of seconds.  
To make use of this, you can add these commands to your ci:
````bash
cargo install (--git github.com/matthiaskrgr/cargo-cache OR cargo-cache) --no-default-features --features ci-autoclean
cargo-cache # no further arguments required
````

#### FAQ
Q: Is this project related to [sccache](https://github.com/mozilla/sccache)?  
A: Nope, this project does not interact with sccaches cache.


#### License:

Copyright 2017-2019 Matthias Krüger

````
Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
<LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
option. All files in the project carrying such notice may not be
copied, modified, or distributed except according to those terms.
````
