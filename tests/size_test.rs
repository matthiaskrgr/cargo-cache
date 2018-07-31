use std::path::*;
use std::process::Command;
use walkdir::WalkDir;

// note: to make debug prints work:
// cargo test -- --nocapture
#[cfg(test)]
mod sizetests {
    use super::*;

    #[test]
    fn build_and_check_size_test() {
        // move into the directory of our dummy crate
        // set a fake CARGO_HOME and build the dummy crate there
        let crate_path = PathBuf::from("tests/size_test/");
        let fchp = "tests/size_test/fake_cargo_home"; // cake cargo_home path
        let status = Command::new("cargo")
            .arg("check")
            .current_dir(&crate_path)
            .env("CARGO_HOME", "fake_cargo_home")
            .output();
        // make sure the build succeeded
        assert!(status.is_ok(), "build of dummy crate did not succeed");
        assert!(
            PathBuf::from(&fchp).is_dir(),
            "fake cargo home was not created!"
        );
        // make sure the size of the registry matches and we have 4 entries
        let mut registry_cache_path = PathBuf::from(&fchp);
        registry_cache_path.push("registry");
        registry_cache_path.push("cache");
        assert!(registry_cache_path.is_dir(), "no registry cache found");
        let mut dirs = WalkDir::new(registry_cache_path).min_depth(2).into_iter();
        // make sure we have all the items, unroll the iterator
        let pkg_cfg = dirs.next();
        let cc = dirs.next();
        let unicode = dirs.next();
        let libc = dirs.next();
        let empty = dirs.next(); // should be None
        assert!(empty.is_none(), "did not find exactly 4 downloaded crates!");
        // make sure the filenames all match
        assert_eq!(
            "pkg-config-0.3.12.crate",
            pkg_cfg.unwrap().unwrap().path().file_name().unwrap()
        );
        assert_eq!(
            "cc-1.0.18.crate",
            cc.unwrap().unwrap().path().file_name().unwrap()
        );
        assert_eq!(
            "unicode-xid-0.0.4.crate",
            unicode.unwrap().unwrap().path().file_name().unwrap()
        );
        assert_eq!(
            "libc-0.2.42.crate",
            libc.unwrap().unwrap().path().file_name().unwrap()
        );
        // make sure cargo cache is built
        let cargo_build = Command::new("cargo").arg("build").output();
        assert!(cargo_build.is_ok(), "could not build cargo cache");
        // run it on the fake cargo cache dir
        let cargo_cache = Command::new("target/debug/cargo-cache")
            .env("CARGO_HOME", &fchp)
            .output();
        assert!(cargo_cache.is_ok(), "cargo cache failed to run");
        let cc_output = String::from_utf8_lossy(&cargo_cache.unwrap().stdout).into_owned();
        // we need to get the actual path to fake cargo home dir and make it an absolute path
        let absolute_fchp = PathBuf::from(&fchp).canonicalize().unwrap();
        let mut desired_output = format!("Cargo cache '{}':\n\n", absolute_fchp.display());

        desired_output.push_str(
            "Total size:                   66.57 MB
Size of 0 installed binaries:     0 B
Size of registry:                  66.57 MB
Size of registry crate cache:           407.94 KB
Size of registry source checkouts:      2.04 MB
Size of git db:                    0 B
Size of git repo checkouts:        0 B\n",
        );
        // make sure the sizes match
        assert_eq!(desired_output, cc_output);
    }
}
