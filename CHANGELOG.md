## Git
````
local: ignore crate deps when using the local subcommand
	this cargo-cache local "freezing" when being run while a different process was already writing to the $CARGO_HOME
Print message that we are clearing the cache when using autoclean after printing the before-stats since clearing can
	take some time depending on hardware
Add "offline_tests" feature which disables tests that would require internet connection
	(to download crates/registry indices and initialize ${CARGO_HOMES})
Add --remove-if-younger-than and --remove-if-older-than to remove files in the cache by date.
	This requires the --remove-dir argument to know where files should be removed from.
	You can pass a time or a date as formatted parameter to the flags for example YYYY.MM.DD or HH::MM::SS.
	Example: cargo-cache --remove-dir=git-db --remove-if-younger-than 10:15:00

Dependencies:
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
