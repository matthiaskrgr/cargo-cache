## cargo cache

[![Build Status](https://github.com/matthiaskrgr/cargo-cache/workflows/ci/badge.svg)](https://github.com/matthiaskrgr/cargo-cache/actions) <!-- [![dependency status](https://deps.rs/repo/github/matthiaskrgr/cargo-cache/status.svg)](https://deps.rs/repo/github/matthiaskrgr/cargo-cache)' -->
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
* search cache via regex queries (`cargo cache query "reg.*x"`)
* print crates that take the most space (`--top-cache-items`)
* alternative registries supported
* remove files older or younger than X (`--remove-if-{older,younger}-than`)
* builds and runs on `stable`, `beta` and `nightly` channel
* purge cache entries not unused to build a specified crate (`cargo cache clean-unref`)
* print size stats on a local sccache build cache  (`cargo cache sc`)
* verify extracted crate sources (`cargo cache verify`)

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
    cargo cache [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -a, --autoclean                        Removes crate source checkouts and git repo checkouts
    -e, --autoclean-expensive              As --autoclean, but also recompresses git repositories
    -f, --fsck                             Fsck git repositories
    -g, --gc                               Recompress git repositories (may take some time)
    -h, --help                             Print help information
    -i, --info                             Print information cache directories, what they are for and what can be safely deleted
    -k, --keep-duplicate-crates <N>        Remove all but N versions of crate in the source archives directory
    -l, --list-dirs                        List all found directory paths
    -n, --dry-run                          Don't remove anything, just pretend
    -o, --remove-if-older-than <date>      Removes items older than specified date: YYYY.MM.DD or HH:MM:SS
    -r, --remove-dir <dir1,dir2,dir3>      Remove directories, accepted values: all,git-db,git-repos,
                                           registry-sources,registry-crate-cache,registry-index,registry
    -t, --top-cache-items <N>              List the top N items taking most space in the cache
    -V, --version                          Print version information
    -y, --remove-if-younger-than <date>    Removes items younger than the specified date: YYYY.MM.DD or HH:MM:SS

SUBCOMMANDS:
    clean-unref    remove crates that are not referenced in a Cargo.toml from the cache
    help           Print this message or the help of the given subcommand(s)
    l              check local build cache (target) of a rust project
    local          check local build cache (target) of a rust project
    q              run a query
    query          run a query
    r              query each package registry separately
    registry       query each package registry separately
    sc             gather stats on a local sccache cache
    sccache        gather stats on a local sccache cache
    toolchain      print stats on installed toolchains
    trim           trim old items from the cache until maximum cache size limit is reached
    verify         verify crate sources
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
Run `cargo cache --autoclean`:
````
Clearing cache...

Cargo cache '/home/matthias/.cargo':

Total:                                     3.38 GB => 3.28 GB
  62 installed binaries:                            665.64 MB
  Registry:                                2.03 GB => 2.00 GB
    2 registry indices:                             444.25 MB
    10570 crate archives:                             1.55 GB
    96 => 0 crate source checkouts:          34.81 MB => 0  B
  Git db:                              685.13 MB => 619.64 MB
    114 bare git repos:                             619.64 MB
    7 => 0 git repo checkouts:               65.48 MB => 0  B

Size changed 3.38 GB => 3.28 GB (-100.29 MB, -2.96%)
````

The crate also works if you override the default location of the cargo home via
the $CARGO_HOME env var!


Side note: cargo-cache started as my *learning-by-doing* rust project, if you see something that you find very odd or is in dire need of improvement please let me know and open a ticket!

#### Cleaning the cache on CI
Sometimes it is desired to [cache the $CARGO_HOME in CI](https://doc.rust-lang.org/nightly/cargo/guide/cargo-home.html#caching-the-cargo-home-in-ci).
As noted in the document, this might cache sources twice which adds unnecessary overhead.
To reduce the size of the cache before storing it, you might want to run `cargo cache --autoclean`.
The `ci-autoclean` feature provides a very stripped-down version of the crate that is only capable of running `cargo-cache --autoclean` automatically on launch and should compile within a couple of seconds.
To make use of this, you can add these commands to your ci:
````bash
cargo install (--git git://github.com/matthiaskrgr/cargo-cache OR cargo-cache) --no-default-features --features ci-autoclean cargo-cache
cargo-cache # no further arguments required
````
You can add the `vendored-libgit` feature if you would like to link libgit statically into cargo-cache.

#### FAQ
Q: Is this project related to [sccache](https://github.com/mozilla/sccache)?
A: Not really.
   `cargo cache sccache` prints a little summary of the local(!) sccache-cache and shows how many files were last accessed on a given date but
   it does not modify sccaches cache. It also does not act as a compiler cache such as (s)ccache.


#### License:

Copyright 2017-2022 Matthias Krüger

````
Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
<LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
option. All files in the project carrying such notice may not be
copied, modified, or distributed except according to those terms.
````
