use home::*;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub(crate) struct DirInfo {
    // make sure we do not accidentally confuse dir_size and file_number
    // since both are of the same type
    /// size of a directory
    pub(crate) dir_size: u64,
    /// number of files of a directory
    pub(crate) file_number: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct CargoCachePaths {
    /// the root path to the cargo home
    pub(crate) cargo_home: PathBuf,
    /// the directory where installed (cargo install..) binaries are located
    pub(crate) bin_dir: PathBuf,
    /// path where registries are stored
    pub(crate) registry: PathBuf,
    /// path where registry caches are stored (the .crate archives)
    pub(crate) registry_pkg_cache: PathBuf,
    /// path where registry sources (.rs files / extracted .crate archives) are stored
    pub(crate) registry_sources: PathBuf,
    /// path where the registry indices (git repo containing information on available crates, versions etc) are stored
    pub(crate) registry_index: PathBuf,
    /// bare git repositories are stored here
    pub(crate) git_repos_bare: PathBuf,
    /// git repository checkouts are stored here
    pub(crate) git_checkouts: PathBuf,
}

impl CargoCachePaths {
    /// returns `CargoCachePaths` object which makes all the subpaths accessible to the crate
    pub(crate) fn default() -> Result<Self, ()> {
        let cargo_home = if let Ok(cargo_home) = home::cargo_home() {
            cargo_home
        } else {
            std::process::exit(1);
            //  return Err(Error::GetCargoHomeFailed);
        };

        if !cargo_home.is_dir() {
            std::process::exit(1);

            //   return Err(Error::CargoHomeNotDirectory(cargo_home));
        }
        // get the paths to the relevant directories
        let bin = cargo_home.join("bin");
        let registry = cargo_home.join("registry");
        let registry_index = registry.join("index");
        let reg_cache = registry.join("cache");
        let reg_src = registry.join("src");
        let git_repos_bare = cargo_home.join("git").join("db");
        let git_checkouts = cargo_home.join("git").join("checkouts");

        Ok(Self {
            cargo_home,
            bin_dir: bin,
            registry,
            registry_index,
            registry_pkg_cache: reg_cache,
            registry_sources: reg_src,
            git_repos_bare,
            git_checkouts,
        })
    }
} // impl CargoCachePaths

// this is the output of `cargo cache --list-dirs`
impl std::fmt::Display for CargoCachePaths {
    fn fmt(&self, f: &'_ mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "\ncargo home:                 {}",
            &self.cargo_home.display()
        )?;
        writeln!(f, "binaries directory:         {}", &self.bin_dir.display())?;
        writeln!(
            f,
            "registry directory:         {}",
            &self.registry.display()
        )?;
        writeln!(
            f,
            "registry index:             {}",
            &self.registry_index.display()
        )?;
        writeln!(
            f,
            "crate source archives:      {}",
            &self.registry_pkg_cache.display()
        )?;
        writeln!(
            f,
            "unpacked crate sources:     {}",
            &self.registry_sources.display()
        )?;
        writeln!(
            f,
            "bare git repos:             {}",
            &self.git_repos_bare.display()
        )?;
        writeln!(
            f,
            "git repo checkouts:         {}",
            &self.git_checkouts.display()
        )?;

        Ok(())
    }
}

/// get the total size and number of files of a directory
pub(crate) fn cumulative_dir_size(dir: &PathBuf) -> DirInfo {
    // Note: using a hashmap to cache dirsizes does apparently not pay out performance-wise
    if !dir.is_dir() {
        return DirInfo {
            dir_size: 0,
            file_number: 0,
        };
    }

    // traverse recursively and sum filesizes, parallelized by rayon
    let walkdir_start = dir.display().to_string();

    let dir_size = WalkDir::new(&walkdir_start)
        .into_iter()
        .map(|e| e.unwrap().path().to_owned())
        .filter(|f| f.exists()) // avoid broken symlinks
        .collect::<Vec<_>>() // @TODO perhaps WalkDir will impl ParallelIterator one day
        .iter()
        .filter(|f| f.exists()) // check if the file still exists. Since collecting and processing a
        // path, some time may have passed and if we have a "cargo build" operation
        // running in the directory, a temporary file may be gone already and failing to unwrap() (#43)
        .map(|f| {
            fs::metadata(f)
                .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &f.display()))
                .len()
        })
        .sum();

    // for the file number, we don't want the actual number of files but only the number of
    // files in the current directory, limit search depth

    let file_number = if walkdir_start.contains("registry") {
        WalkDir::new(&walkdir_start)
            .max_depth(2)
            .min_depth(2)
            .into_iter()
            .count()
    } else {
        fs::read_dir(&dir).unwrap().count()
    } as u64;

    DirInfo {
        dir_size,
        file_number,
    }
}

pub(crate) fn remove_file(
    path: &PathBuf,

    deletion_msg: Option<String>,
    dry_run_msg: Option<String>,
    total_size_from_cache: Option<u64>,
) {
    // print deletion message if we have one
    if let Some(msg) = deletion_msg {
        println!("{}", msg);
    }

    if path.is_file() && fs::remove_file(&path).is_err() {
        eprintln!("Warning: failed to remove file \"{}\".", path.display());
    } else {
    }

    if path.is_dir() && fs::remove_dir_all(&path).is_err() {
        eprintln!(
            "Warning: failed to recursively remove directory \"{}\".",
            path.display()
        );
    } else {
    }
}

fn main() {
    let cargo_cache = match CargoCachePaths::default() {
        Ok(cargo_cache) => cargo_cache,
        Err(e) => {
            std::process::exit(1);
        }
    };

    let reg_srcs = &cargo_cache.registry_sources;
    let git_checkouts = &cargo_cache.git_checkouts;
    for dir in &[reg_srcs, git_checkouts] {
        let size = cumulative_dir_size(dir);
        if dir.is_dir() {
            remove_file(dir, None, None, Some(size.dir_size));
        }
    }
}
