## Git
````
Parallelize internal cache accesses resulting in reduced wall clock execution time
Fix some paths being linuxish on windows
Print size of the registry index (${CARGO_HOME}registry/index) in default view
Add "query" subcommand (run "cargo cache query" or short "cargo cache q"
	Pass a parameter which will be interpreted as regex to only display sizes of
	items that match.
	For example "cargo cache query serde_derive" will print the sizes of all findings and
	their sizes of this crate inside the subdirectories of the cache.
	You can get human readable sizes via --human-readable and sort by size/name (alphabetically)
	via --sort-by $option
	Check out cargo cache q --help for more details
Revert "rename -l (--list-dirs) to -L."
	short form of --list-dirs is -l again.
````
## Version 0.1.2
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

## Version 0.1.1
````
add readme key to Cargo.toml
add crates.io shield to readme
````
## Version 0.1.0
````
Initial release on crates.io
````
