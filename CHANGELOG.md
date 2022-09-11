## Version 0.8.3
```
MSRV: bump from 1.56 to 1.57

Fix panic when "cargo cache toolchain" runs on a system that does not have rust installed via rustup
but instead via system distributor for example.
Instead of crashing, display a nice info message: "Could not find any toolchains installed via rustup!" (#121)
```

## Version 0.8.2 (b7bf4f4)
```
Fix build with nightly rustc 1.60.0-nightly (734368a20 2022-02-07)
unreachable!("{}") is no longer accepted: https://github.com/rust-lang/rust/issues/92137

Build: add "vendored-libgit" feature that enables libgit2 vendoring (statically link libgit2 into cargo-cache instead of linking agaist the system libgit2)
The feature is on by default but it can be skipped (using --no-default-features) by distributors who would like cargo-cache to link against system libgit2
```

## Version 0.8.1 (1fcffbb)
```
Fix: 'verify' incorrectly determines paths as missing due to different unicode representations. (#113 / #114)

New deps:
	unicode-normalization 0.1.19
```

## Version 0.8.0 (e36f9df)
```
Add new subcommand "verify"
cargo cache verify  tries to check corrupted (modified) crate sources
The command looks into the .crate archive and compares the contained files with the extracted sources.
If we find files that are not the same size in both places, the source is considered corrupted.
cargo cache verify --clean-corrupted will automatically remove the corrupted sources. (supports --dry-run as well)

Upgrade from clap 2.33.3 to 3.0.0
If you notice any change in behaviour that is not listed here, please let me know!

Cli: removed "cargo cache query -h" for --human-readable because it was ambiguous with -h of --help.

Cli: fix bugs where I was implementing a subcommand cargo-cache foo but then actually checking for it as an argument cargo ceche --foo
or vice versa.
Found during migration to clap v3

New deps:
	flate2 1.0.22
	tar 0.4.38
```

## Version 0.7.0 (ab0166b)
````
Fully remove -d shorthand for --dry-run, use -n instead (#97)
Update git2 dependency and enable static bundling; don't link against system libgit2 (#105)
Switch crate to edition 2021 and specify rust 1.56 as minimum supported rust version.

Updated dependencies:
	cargo_metadata 0.13.1 -> 0.14.1
	git2 0.13.12 -> 0.13.22 (+ vendored-libgit2 feat)
	pretty_assertions 0.7.2 -> 1.0.0
````

## Version 0.6.3 (7be9257)
````
Add better explanation what the "limit" arg of --trim accepts (#99)
Fix cargo cache dry-run --trim (#100)
Make cargo cache --gc use different git commands to compress a bit better
Make cargo cache --help wrap properly and add colors (#101)

Updated dependencies:
	remove_dir_all 0.6.1 -> 0.7.0
	pretty_assertions 0.6.1 -> 0.7.2
	cargo_metadata 0.12.3 -> 0.13.1
````

## Version 0.6.2 (5c15652)
````
Start deprecation of -d for --dry-run, use -n instead. (#97)
Cli: make sure that --autoclean-expensive --gc, --autoclean-expensive --autoclean as well as
    --gc --autoclean  cause "only" --autoclean-expensive to run.
````

## Version 0.6.1 (967cd4a)
````
clean-unref: fix bug where some sources were accidentally removed (#96)
````

## Version 0.6.0 (a55d4db)
````
Make the summary when sizes change more detailed, it now looks like this:

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


Print a more informative error message when "git" is not installed and we run "cargo cache --gc" (#94)
cargo cache sc: fix detection of `sccache` cache dir on windows (#90)
cargo cache clean-unref: print same summary stats as in "cargo cache -a" (#95) 
Changed dependency `dirs` to `dirs-next`, as `dirs` is no longer maintained.
Add "trim" subcommand which gets a --limit param and trims the cache down to that size limit.
	While calculating the cache size, registry indices and installed binaries are skipped.
Add a "toolchain" subcommand which displays stats on rustup-installed toolchains:

Toolchain Name                               Files  Size     Percentage
beta-x86_64-unknown-linux-gnu                23507  1.77 GB  21.75 %
nightly-x86_64-unknown-linux-gnu             23570  1.75 GB  21.51 %
nightly-2021-01-15-x86_64-unknown-linux-gnu  23536  1.69 GB  20.68 %
stable-x86_64-unknown-linux-gnu              19097  1.63 GB  20.02 %
master                                       3843   1.31 GB  16.04 %

Total                                        93553  8.16 GB  100 %


Updated dependencies:
	cargo-metadata: 0.11.0 -> 0.12.1
	clap: 2.33.1 -> 2.33.3
	git2: 0.13.5-> 0.13.12
	rayon: 1.3.0 -> 1.5.0
	regex: 1.3.7 -> 1.4.2
	cfg-if: 0.1.10 -> 1.0.0
	chrono: 0.4.11 -> 0.4.19
	dirs-next: 1.0.1 -> 2.0.0
	path-slash: 0.1.1 -> 0.1.3
	fx-extra 1.1.0: -> 1.2.0
	remove_dir_all: 0.5.3 -> 0.6.0
````

## Version 0.5.1 (bb1b58e)
````
Fix "sccache" subcommand not finding sccache cache dir on macos (#87)

````

## Version 0.5.0 (b0af676)
````
Added "scache"/"sc" subcommand which prints a summary for a local(!) sccache cache and shows the
	number of files last accessed on a given date: cargo cache sc
Files  Day         Size       Percentage
2203   2020-07-16  1.21 GB    22.59 %
3813   2020-07-17  1.63 GB    30.41 %
1061   2020-07-18  727.39 MB  13.56 %
1592   2020-07-19  831.35 MB  15.50 %
1873   2020-07-20  962.11 MB  17.94 %
Added new clean-unref subcommand which removes all crates that are not referenced in the 
    dependency-tree of a crate from the cache.
    By default it will check if it can find a Cargo.toml nearby, alternatively,
    you can pass the --manifest-path <path/to/Cargo.toml> to specify a Cargo.toml.
    This command helps minimizing the ci-cache of travis/azure-ci/github-actions etc. (#76)
--keep-duplicate-crates: fix parsing of packages with alpha/beta/pre versions (#81)
Improve state of build and tests on NixOS (#84)

Updated dependencies:
	cargo-metadata: 0.10.0 -> 0.11.0
	dirs: new
	remove_dir_all: new
````

## Version 0.4.3 (79a2f8a)
````
Fix bug where functionality of --remove-if-older-than and --remove-if-younger-than was swapped by accident (#80)

Updated dependencies:
	cargo_metadata: 0.9.0 -> 0.10.0
	clap 2.33.0 -> 2.33.1
	git2: 0.13.0 -> 0.13.5
	home: 0.5.1 -> 0.5.3
	rayon: 1.2.0 -> 1.3.0
	regex: 1.3.1 -> 1.3.7
	walkdir: 2.2.9 -> 2.3.1
	chrono: 0.4.9 -> 0.4.11
````

## Version 0.4.2 (79a2f8a)
````
Fix readme and --help mentioning unimplemented features. Sorry about that!
````

## Version 0.4.1 (2a446ea)
````
Fix wrong date format string in readme, --help and tests
````

## Version 0.4.0 (0d92713)
````
local: ignore crate deps when using the local subcommand
	this fixes "cargo-cache local" "freezing" when being run while a different process was already writing to the $CARGO_HOME
Print message that we are clearing the cache when using autoclean after printing the before-stats since clearing can
	take some time depending on hardware
Add "offline_tests" feature which disables tests that would require internet connection
	(to download crates/registry indices and initialize ${CARGO_HOMES})
Add --remove-if-younger-than and --remove-if-older-than to remove files in the cache by date.
	This requires the --remove-dir argument to know where files should be removed from.
	You can pass a time or a date as formatted parameter to the flags for example YYYY.MM.DD or HH::MM::SS.
	Example: cargo-cache --remove-dir=git-db --remove-if-younger-than 10:15:00

Updated dependencies:
	chrono: new
	git2: 0.11.0 -> 0.13.0
````

## Version 0.3.4 (650849a)
````
Fix bug where `cargo cache local` would panic on crates with workspaces
		This was caused by passing wrong parameters to cargo_metadata when parsing the manifest
Add ci-autoclean feature
		The feature creates a `cargo-cache` binary that only executes `cargo cache --autoclean` by default (requires no arguments, just run `cargo-cache`).
		and has all other features and dependencies stripped out.
		This is intended for usage on continuous integration when the $CARGO_HOME is cached in order to reduce size	before uploading the cargo home to the ci cache.
		Run `cargo install (--git github.com/matthiaskrgr/cargo-cache / cargo-cache) --no-default-features --features ci-autoclean` to install inside ci.

Updated dependencies:
	git2 0.10.2 -> 0.11.0
	cargo_metadata 0.8.2 -> 0.9.0
	home 0.5.0 -> 0.5.1
	cfg-if: new
````

## Version 0.3.3 (b5e5752)
````
Fix bugs where `--remove-dir` could panic if we removed a directory and an internal cache
	was not updated properly leading to inconsistent cache state, unmet expectations and unwrap of None resulting in a panic.
	This was discovered while investigating #72
````

## Version 0.3.2 (f7184cc)
````
Warn if --dry-run is passed but nothing to be dry-run is specified. (#59)
Fix crash when calling "local" subcommand on a project without target dir (#67)

Updated dependencies:
	cargo_metadata 0.8.1 -> 0.8.2
	git2 0.10.0 -> 0.10.1
	home 0.3.4 -> 0.5.0
	rayon 1.1.0 -> 1.2.0
	regex 1.2.0 -> 1.3.1
````

## Version 0.3.1 (4b9baf6)
````
Don't crash when encountering spurious files in git cache directories where folders are expected (#65)
````

## Version 0.3.0 (aaed2c9)
````
Change the default summary/output from:

Total size:                             4.21 GB
Size of 102 installed binaries:           920.95 MB
Size of registry:                         2.25 GB
Size of registry index:                     226.98 MB
Size of 4411 crate archives:                684.28 MB
Size of 2399 crate source checkouts:        1.34 GB
Size of git db:                           1.04 GB
Size of 113 bare git repos:                 993.72 MB
Size of 7 git repo checkouts:               51.05 MB

to

Total:                                4.21 GB
  102 installed binaries:           920.95 MB
  Registry:                           2.25 GB
    2 registry indices              226.98 MB
    4411 crate archives:            684.28 MB
    2399 crate source checkouts:      1.34 GB
  Git db:                             1.04 GB
    113 bare git repos:             993.72 MB
    7 git repo checkouts:            51.05 MB

(#44, #36)

Add support for alternative registries.
"cargo cache registry" or "cargo cache r" will print a per-registry summary:

Cargo cache '/home/matthias/.cargo':

Total:                               4.06 GB
  103 installed binaries:            1.10 GB
  Registry:                        163.43 MB
    Registry index:                156.47 MB
    5 crate archives:                6.96 MB
  Registry: dl.cloudsmith.io         4.18 KB
    Registry index:                  3.21 KB
    1 crate archives:                 971  B
  Registry: github.com               1.40 GB
    Registry index:                 94.28 MB
    5404 crate archives:           799.06 MB
    1031 crate source checkouts:   504.58 MB
  Git db:                            1.40 GB
    138 bare git repos:              1.37 GB
    4 git repo checkouts:           32.98 MB

(#50, #51, #53)

print the number of registry indices (if > 0)
--info: make more detailed, add registry index
query: don't print empty line if no matches were found
--autoclean: when combined with --dry-run, print size of
	the to-be-removed directory (#54)
build git2 dependency without default features
Updated dependencies:
	cargo-metadata 0.8.0 -> 0.8.1
	git2: 0.8.0 -> 0.9.1
	rayon: 1.0.3 -> 1.1.0
	regex 1.1.6 -> 1.2.0
	walkdir: 2.2.7 -> 2.2.9
````
## Version 0.2.1 (319bee6)
````
Fix version numbers registry source cache findings of query subcmd being cut off (#41)
Fix crash when calling "cargo cache local" on a target dir that was actively used by a cargo process.
	I was collecting files, filtering out nonexisting paths and then (in parallel) requesting metadata on 
	the files but sometimes it happened that temporary files were deleted by cargo between the collection of
	available paths and a rayon job asking for the metadata leading fs::metadata() to fail unwrapping as the
	file was already gone.
	Mitigate by only asking for metadata if the file still exists right before doing so. (#43).
Fix wrong order of lines in "query" subcmd when --sort-by size is passed (#42)
Add --fsck flag to run git fsck on the cache repos (cargo cache --fsck / -f)
Updated dependencies:
	cargo-metadata: 0.7.4 -> 0.8.0
	rustc_tools_util: 0.1.1-> 0.2.0
````
## Version 0.2.0 (eada74c)
````
Parallelize internal cache accesses resulting in reduced wall clock execution time
Fix some paths being linuxish on windows
Print size of the registry index (${CARGO_HOME}/registry/index) in default view
Add "query" subcommand (run "cargo cache query" or short "cargo cache q"
	Pass a parameter which will be interpreted as regex to only display sizes of
	items that match.
	For example "cargo cache query serde_derive" will print the sizes of all findings and
	their sizes of this crate inside the subdirectories of the cache.
	You can get human readable sizes via --human-readable and sort by size/name (alphabetically)
	via --sort-by $option
	Check out cargo cache q --help for more details
Add libs.rs badge to readme
Revert "rename -l (--list-dirs) to -L."
	short form of --list-dirs is -l again.
Add "local" subcommand ("cargo cache local" or "cargo cache l") which displays
	sizes of a target/ dir of a built crate
Make --remove-dir accept "registry-index" for clearing out the registry index
Make --dry-run print a summary of how much space would be freed
Use "home" crate instead of "cargo" to get the ${CARGO_HOME} value.
	This gets rid of a lot of dependencies and speeds up the build.
Update dependencies:
	cargo: removed
	clap: 2.32.0 -> 2.33.0
	regex: 1.1.0 -> 1.1.6
	pretty_assertions: 0.5.1 -> 0.6.1
	cargo_metadata: new
	home: new
````
## Version 0.1.2 (01f6952)
````
run cargo update
don't assume that we know if a folder will only contain directories or files, add some checks.
	This fixes crashes when trying to get the size of files there were actually dead symlinks
	or the contents of directories that turned out to be files
	fixes #31
add function that is responsible for all file/directory deletion and honours --dry-run.
	No other function in the crate should call fs::remove.* to make sure --dry-run is always
	taken into account.
add error messages to unwrap() failures of fs::metadata() calls in the cache module
add more usage examples to the readme
dependencies: update rustc_tools_util from 0.1.0 to 0.1.1
	fixes #28 (strange version info output when installed from crates.io)
````

## Version 0.1.1 (3d50deb)
````
add readme key to Cargo.toml
add crates.io shield to readme
````
## Version 0.1.0 (2ec7647)
````
Initial release on crates.io
````
