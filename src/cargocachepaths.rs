use std::path::PathBuf;

use crate::library::*;

#[derive(Debug, Clone)]
pub(crate) struct CargoCachePaths {
    pub(crate) cargo_home: PathBuf,
    pub(crate) bin_dir: PathBuf,
    pub(crate) registry: PathBuf,
    pub(crate) registry_cache: PathBuf,
    pub(crate) registry_sources: PathBuf,
    pub(crate) registry_index: PathBuf,
    pub(crate) git_repos_bare: PathBuf,
    pub(crate) git_checkouts: PathBuf,
}

impl CargoCachePaths {
    // holds the PathBufs to the different componens of the cargo cache
    pub(crate) fn new() -> Result<Self, (ErrorKind, String)> {
        let cargo_cfg = match cargo::util::config::Config::default() {
            Ok(cargo_cfg) => cargo_cfg,
            Err(_) => {
                return Err((
                    ErrorKind::CargoFailedGetConfig,
                    "Failed to get cargo config!".to_string(),
                ))
            }
        };

        let cargo_home_path = cargo_cfg.home().clone().into_path_unlocked();
        let cargo_home_str = cargo_home_path.display();
        let cargo_home_path_clone = cargo_home_path.clone();

        if !cargo_home_path.is_dir() {
            let msg = format!(
                "Error, no cargo home path directory '{}' found.",
                &cargo_home_str
            );
            return Err((ErrorKind::CargoHomeNotDirectory, msg));
        }
        // get the paths to the relevant directories
        let cargo_home = cargo_home_path;
        let bin = cargo_home.join("bin/");
        let registry = cargo_home.join("registry/");
        let registry_index = registry.join("index/");
        let reg_cache = registry.join("cache/");
        let reg_src = registry.join("src/");
        let git_repos_bare = cargo_home.join("git/db/");
        let git_checkouts = cargo_home_path_clone.join("git/checkouts/");

        Ok(Self {
            cargo_home,
            bin_dir: bin,
            registry,
            registry_index,
            registry_cache: reg_cache,
            registry_sources: reg_src,
            git_repos_bare,
            git_checkouts,
        })
    }

    pub(crate) fn get_dir_paths(&self) -> String {
        let mut s = String::with_capacity(500);
        s.push_str("\n");
        s.push_str(&format!(
            "cargo home:                 {}\n",
            &self.cargo_home.display()
        ));

        s.push_str(&format!(
            "binaries directory:         {}\n",
            &self.bin_dir.display()
        ));
        s.push_str(&format!(
            "registry directory:         {}\n",
            &self.registry.display()
        ));
        s.push_str(&format!(
            "registry index:             {}\n",
            &self.registry_index.display()
        ));
        s.push_str(&format!(
            "crate source archives:      {}\n",
            &self.registry_cache.display()
        ));
        s.push_str(&format!(
            "unpacked crate sources:     {}\n",
            &self.registry_sources.display()
        ));
        s.push_str(&format!(
            "bare git repos:             {}\n",
            &self.git_repos_bare.display()
        ));
        s.push_str(&format!(
            "git repo checkouts:         {}\n",
            &self.git_checkouts.display()
        ));
        s
    }
}

#[cfg(test)]
mod libtests {
    use super::*;
    // use pretty_assertions::assert_eq;
    use std::env;
    use std::fs;

    #[allow(non_snake_case)]
    #[test]
    fn test_CargoCachePaths_gen() {
        // set cargo cache root dir to /tmp
        env::set_var("/tmp/", "CARGO_HOME");
        let dir_paths = CargoCachePaths::new();
        assert!(dir_paths.is_ok());
    }

    #[allow(non_snake_case)]
    #[test]
    fn test_CargoCachePaths_paths() {
        // get cargo target dir
        let mut target_dir = std::env::current_dir().unwrap();
        target_dir.push("target");
        let mut cargo_home = target_dir;
        cargo_home.push("cargo_home");
        //make sure this worked
        let CH_string = format!("{}", cargo_home.display());
        assert!(CH_string.ends_with("cargo-cache/target/cargo_home"));

        // create the directory
        if !std::path::PathBuf::from(&CH_string).is_dir() {
            std::fs::DirBuilder::new().create(&CH_string).unwrap();
        }
        assert!(fs::metadata(&CH_string).unwrap().is_dir());
        assert!(std::path::PathBuf::from(&CH_string).is_dir());

        // set cargo home to this directory
        std::env::set_var("CARGO_HOME", CH_string);
        let ccp = CargoCachePaths::new().unwrap();

        // test all the paths
        assert!(ccp.cargo_home.display().to_string().ends_with("cargo_home"));
        assert!(
            ccp.bin_dir
                .display()
                .to_string()
                .ends_with("cargo_home/bin/")
        );
        assert!(
            ccp.registry
                .display()
                .to_string()
                .ends_with("cargo_home/registry/")
        );
        assert!(
            ccp.registry_index
                .display()
                .to_string()
                .ends_with("cargo_home/registry/index/")
        );
        assert!(
            ccp.registry_cache
                .display()
                .to_string()
                .ends_with("cargo_home/registry/cache/")
        );
        assert!(
            ccp.registry_sources
                .display()
                .to_string()
                .ends_with("cargo_home/registry/src/")
        );
        assert!(
            ccp.git_repos_bare
                .display()
                .to_string()
                .ends_with("cargo_home/git/db/")
        );
        assert!(
            ccp.git_checkouts
                .display()
                .to_string()
                .ends_with("cargo_home/git/checkouts/")
        );
    }

    #[allow(non_snake_case)]
    #[test]
    fn test_CargoCachePaths_print() {
        // get cargo target dir
        let mut target_dir = std::env::current_dir().unwrap();
        target_dir.push("target");
        let mut cargo_home = target_dir;
        cargo_home.push("cargo_home");
        //make sure this worked
        let CH_string = format!("{}", cargo_home.display());
        assert!(CH_string.ends_with("cargo-cache/target/cargo_home"));

        // create the directory
        if !std::path::PathBuf::from(&CH_string).is_dir() {
            std::fs::DirBuilder::new().create(&CH_string).unwrap();
        }
        assert!(fs::metadata(&CH_string).unwrap().is_dir());
        assert!(std::path::PathBuf::from(&CH_string).is_dir());

        // set cargo home to this directory
        std::env::set_var("CARGO_HOME", CH_string);
        let ccp = CargoCachePaths::new().unwrap();

        let output = ccp.get_dir_paths();
        let iter = output.lines();
    }

}
