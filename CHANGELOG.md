## Git
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

(#44 / #36)

print the number of registry indices (if > 0)
--info: make more detailed, add registry index
query: don't print empty line if no matches were found
build git2 dependency without default features
Updated dependencies:
	git2: 0.8.0 -> 0.9.1
	rayon: 1.0.3 -> 1.1.0
	regex 1.1.6 -> 1.1.7
	walkdir: 2.2.7 -> 2.2.8
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
