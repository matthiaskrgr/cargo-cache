use std::path::PathBuf;

use humansize::{file_size_opts, FileSize};

#[derive(Debug, Clone)]
pub(crate) struct DirInfo {
    // make sure we do not accidentally confuse dir_size and file_number
    // since both are of the same type
    pub(crate) dir_size: u64,
    pub(crate) file_number: u64,
}

#[cfg_attr(feature = "cargo-clippy", allow(similar_names))] // FP due to derives
#[derive(Debug, Clone)]
pub(crate) struct DirSizes {
    pub(crate) total_size: u64,     // total size of cargo root dir
    numb_bins: u64,                 // number of binaries found
    total_bin_size: u64,            // total size of binaries found
    total_reg_size: u64,            // registry size
    total_git_db_size: u64,         // size of bare repos and checkouts combined
    total_git_repos_bare_size: u64, // git db size
    numb_git_repos_bare_repos: u64, // number of cloned repos
    numb_git_checkouts: u64,        // number of checked out repos
    total_git_chk_size: u64,        // git checkout size
    total_reg_cache_size: u64,      // registry cache size
    total_reg_src_size: u64,        // registry sources size
    numb_reg_cache_entries: u64,    // number of source archives
    numb_reg_src_checkouts: u64,    // number of source checkouts
}

impl DirSizes {
    pub(crate) fn print_pretty(&self, cache_root_dir: &PathBuf) -> String {
        // create a string and concatenate all the things we want to print with it
        // and only print it in the end, this should save a few syscalls and be faster than
        // printing every line one by one

        fn pad_strings(indent_lvl: i8, beginning: &str, end: &str) -> String {
            // max line width
            const MAX_WIDTH: i8 = 37;

            let len_padding: i8 = (MAX_WIDTH + indent_lvl * 2) - (beginning.len() as i8);
            let mut formatted_line = beginning.to_string();
            formatted_line.push_str(&String::from(" ").repeat(len_padding as usize));
            formatted_line.push_str(&end);
            formatted_line.push_str("\n");
            formatted_line
        }

        // @TODO use format_args!() ?
        let mut s = String::with_capacity(470);

        s.push_str(&format!(
            "Cargo cache '{}/':\n\n",
            &cache_root_dir.display()
        ));

        s.push_str(&pad_strings(
            0,
            "Total size: ",
            &self.total_size.file_size(file_size_opts::DECIMAL).unwrap(),
        ));

        s.push_str(&pad_strings(
            1,
            &format!("Size of {} installed binaries: ", self.numb_bins),
            &self
                .total_bin_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap(),
        ));

        s.push_str(&pad_strings(
            1,
            "Size of registry: ",
            &self
                .total_reg_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap(),
        ));

        s.push_str(&pad_strings(
            2,
            &format!("Size of {} crate archives: ", self.numb_reg_cache_entries),
            &self
                .total_reg_cache_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap(),
        ));

        s.push_str(&pad_strings(
            2,
            &format!(
                "Size of {} crate source checkouts: ",
                self.numb_reg_src_checkouts
            ),
            &self
                .total_reg_src_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap(),
        ));

        s.push_str(&pad_strings(
            1,
            &format!("Size of git db: "),
            &self
                .total_git_db_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap(),
        ));

        s.push_str(&pad_strings(
            2,
            &format!(
                "Size of {} bare git repos: ",
                self.numb_git_repos_bare_repos
            ),
            &self
                .total_git_repos_bare_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap(),
        ));

        s.push_str(&pad_strings(
            2,
            &format!("Size of {} git repo checkouts: ", self.numb_git_checkouts),
            &self
                .total_git_chk_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap(),
        ));

        s
    }
}

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



#[cfg(test)]
mod libtests {
    use super::*;

    impl DirSizes {
        #[allow(non_snake_case)]
        pub(super) fn new_manually(
            DI_bindir: &DirInfo,
            DI_git_repos_bare: &DirInfo,
            DI_git_checkout: &DirInfo,
            DI_reg_cache: &DirInfo,
            DI_reg_src: &DirInfo,
            DI_reg_index: &DirInfo,
        ) -> Self {
            let bindir = DI_bindir;
            let git_repos_bare = DI_git_repos_bare;
            let git_checkouts = DI_git_checkout;
            let reg_cache = DI_reg_cache;
            let reg_src = DI_reg_src;
            let reg_index = DI_reg_index;

            let total_reg_size = reg_index.dir_size + reg_cache.dir_size + reg_src.dir_size;
            let total_git_db_size = git_repos_bare.dir_size + git_checkouts.dir_size;

            Self {
                //no need to recompute all of this from scratch
                total_size: total_reg_size + total_git_db_size + bindir.dir_size,
                numb_bins: bindir.file_number,
                total_bin_size: bindir.dir_size,
                total_reg_size,

                total_git_db_size,
                total_git_repos_bare_size: git_repos_bare.dir_size,
                numb_git_repos_bare_repos: git_repos_bare.file_number,

                total_git_chk_size: git_checkouts.dir_size,
                numb_git_checkouts: git_checkouts.file_number,

                total_reg_cache_size: reg_cache.dir_size,
                numb_reg_cache_entries: reg_cache.file_number,

                total_reg_src_size: reg_src.dir_size,
                numb_reg_src_checkouts: reg_src.file_number,
            }
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_DirSizes() {
        // DirInfors to construct DirSizes from
        let bindir = DirInfo {
            dir_size: 121_212,
            file_number: 31,
        };
        let git_repos_bare = DirInfo {
            dir_size: 121_212,
            file_number: 37,
        };
        let git_checkouts = DirInfo {
            dir_size: 34984,
            file_number: 8,
        };
        let reg_cache = DirInfo {
            dir_size: 89,
            file_number: 23445,
        };
        let reg_src = DirInfo {
            dir_size: 1_938_493_989,
            file_number: 123_909_849,
        };
        let reg_index = DirInfo {
            dir_size: 23,
            file_number: 12345,
        };

        // create a DirSizes object
        let dirSizes = DirSizes::new_manually(
            &bindir,
            &git_repos_bare,
            &git_checkouts,
            &reg_cache,
            &reg_src,
            &reg_index,
        );

        let cache_root = PathBuf::from("/home/user/.cargo");
        let output_is = dirSizes.print_pretty(&cache_root);

        let output_should = "Cargo cache '/home/user/.cargo/':

Total size:                          1.94 GB
Size of 31 installed binaries:         121.21 KB
Size of registry:                      1.94 GB
Size of 23445 crate archives:            89 B
Size of 123909849 crate source checkouts:1.94 GB
Size of git db:                        156.20 KB
Size of 37 bare git repos:               121.21 KB
Size of 8 git repo checkouts:            34.98 KB\n";

        assert_eq!(output_is, output_should);
    }

}
