## Git
````
Add --fsck flag to run git fsck on the cache repos (cargo cache --fsck / -f)
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
