## Git

add function that is responsible for all file/directory deletion and honours --dry-run.
	no other function in the crate should call fs::remove.* to make sure --dry-run is always
    taken into account.
add error messages to unwrap() failures of fs::metadata() calls in the cache module
add more usage examples to the readme
dependencies: update rustc_tools_util from 0.1.0 to 0.1.1
	fixes #28 (strange version info output when installed from crates.io)


## Version 0.1.1

add readme key to Cargo.toml
add crates.io shield to readme

## Version 0.1.0

Initial release on crates.io
