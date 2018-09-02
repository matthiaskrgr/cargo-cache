use std::path::PathBuf;

use crate::library::*;

use humansize::{file_size_opts, FileSize};

#[cfg_attr(feature = "cargo-clippy", allow(similar_names))] // FP due to derives
#[derive(Debug, Clone)]
pub(crate) struct DirSizes {
    pub(crate) total_size: u64,                // total size of cargo root dir
    pub(crate) numb_bins: u64,                 // number of binaries found
    pub(crate) total_bin_size: u64,            // total size of binaries found
    pub(crate) total_reg_size: u64,            // registry size
    pub(crate) total_git_db_size: u64,         // size of bare repos and checkouts combined
    pub(crate) total_git_repos_bare_size: u64, // git db size
    pub(crate) numb_git_repos_bare_repos: u64, // number of cloned repos
    pub(crate) numb_git_checkouts: u64,        // number of checked out repos
    pub(crate) total_git_chk_size: u64,        // git checkout size
    pub(crate) total_reg_cache_size: u64,      // registry cache size
    pub(crate) total_reg_src_size: u64,        // registry sources size
    pub(crate) numb_reg_cache_entries: u64,    // number of source archives
    pub(crate) numb_reg_src_checkouts: u64,    // number of source checkouts
}

impl DirSizes {
    pub(crate) fn new(ccd: &CargoCachePaths) -> Self {
        let bindir = cumulative_dir_size(&ccd.bin_dir);
        let git_repos_bare = cumulative_dir_size(&ccd.git_repos_bare);
        let git_checkouts = cumulative_dir_size(&ccd.git_checkouts);
        let reg_cache = cumulative_dir_size(&ccd.registry_cache);
        let reg_src = cumulative_dir_size(&ccd.registry_sources);
        let reg_index = cumulative_dir_size(&ccd.registry_index);

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
    pub(crate) fn print_pretty(&self, cache_root_dir: &PathBuf) -> String {
        // create a string and concatenate all the things we want to print with it
        // and only print it in the end, this should save a few syscalls and be faster than
        // printing every line one by one

        fn pad_strings(indent_lvl: i64, beginning: &str, end: &str) -> String {
            // max line width
            const MAX_WIDTH: i64 = 40;

            let left = MAX_WIDTH + (indent_lvl * 2);
            let right = beginning.len() as i64;
            let len_padding = left - right;
            assert!(
                len_padding > 0,
                format!(
                    "len_padding is negative: '{} - {} = {}' ",
                    left, right, len_padding
                )
            );

            let mut formatted_line = beginning.to_string();
            #[cfg_attr(
                feature = "cargo-clippy",
                allow(cast_sign_loss, cast_possible_truncation)
            )]
            // I tried mittigating via previous assert()
            formatted_line.push_str(&" ".repeat(len_padding as usize));
            formatted_line.push_str(end);
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
            "Size of git db: ",
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

#[cfg(test)]
mod libtests {
    use super::*;
    use pretty_assertions::assert_eq;
    use test::Bencher;

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

    #[allow(non_snake_case)]
    #[test]
    fn test_DirInfo() {
        let x = DirInfo {
            dir_size: 10,
            file_number: 20,
        };
        assert_eq!(x.dir_size, 10);
        assert_eq!(x.file_number, 20);
    }

    #[allow(non_snake_case)]
    #[test]
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

Total size:                             1.94 GB
Size of 31 installed binaries:            121.21 KB
Size of registry:                         1.94 GB
Size of 23445 crate archives:               89 B
Size of 123909849 crate source checkouts:   1.94 GB
Size of git db:                           156.20 KB
Size of 37 bare git repos:                  121.21 KB
Size of 8 git repo checkouts:               34.98 KB\n";

        assert_eq!(output_is, output_should);
    }

    #[bench]
    fn bench_pretty_print(b: &mut Bencher) {
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
        let dir_sizes = DirSizes::new_manually(
            &bindir,
            &git_repos_bare,
            &git_checkouts,
            &reg_cache,
            &reg_src,
            &reg_index,
        );

        let cache_root = PathBuf::from("/home/user/.cargo");

        b.iter(|| {
            dir_sizes.print_pretty(&cache_root);
        })
    }

    #[allow(non_snake_case)]
    #[test]
    fn test_DirSizes_gigs() {
        // DirInfors to construct DirSizes from
        let bindir = DirInfo {
            dir_size: 6_4015_8118,
            file_number: 69,
        };
        let git_repos_bare = DirInfo {
            dir_size: 3_0961_3689,
            file_number: 123,
        };
        let git_checkouts = DirInfo {
            dir_size: 39_2270_2821,
            file_number: 36,
        };
        let reg_cache = DirInfo {
            dir_size: 5_5085_5781,
            file_number: 3654,
        };
        let reg_src = DirInfo {
            dir_size: 9_0559_6846,
            file_number: 1615,
        };
        let reg_index = DirInfo {
            dir_size: 23,
            file_number: 0,
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

Total size:                             6.33 GB
Size of 69 installed binaries:            640.16 MB
Size of registry:                         1.46 GB
Size of 3654 crate archives:                550.86 MB
Size of 1615 crate source checkouts:        905.60 MB
Size of git db:                           4.23 GB
Size of 123 bare git repos:                 309.61 MB
Size of 36 git repo checkouts:              3.92 GB\n";

        assert_eq!(output_is, output_should);
    }

    #[allow(non_snake_case)]
    #[test]
    fn test_DirSizes_almost_empty() {
        // DirInfors to construct DirSizes from
        let bindir = DirInfo {
            dir_size: 0,
            file_number: 0,
        };
        let git_repos_bare = DirInfo {
            dir_size: 0,
            file_number: 0,
        };
        let git_checkouts = DirInfo {
            dir_size: 0,
            file_number: 0,
        };
        let reg_cache = DirInfo {
            dir_size: 130_4234_1234,
            file_number: 4,
        };
        let reg_src = DirInfo {
            dir_size: 2_6846_1234,
            file_number: 4,
        };
        let reg_index = DirInfo {
            dir_size: 12_5500_0000,
            file_number: 1,
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

Total size:                             14.57 GB
Size of 0 installed binaries:             0 B
Size of registry:                         14.57 GB
Size of 4 crate archives:                   13.04 GB
Size of 4 crate source checkouts:           268.46 MB
Size of git db:                           0 B
Size of 0 bare git repos:                   0 B
Size of 0 git repo checkouts:               0 B\n";

        assert_eq!(output_is, output_should);
    }

    #[allow(non_snake_case)]
    #[test]
    fn test_DirSizes_actually_empty() {
        // DirInfors to construct DirSizes from
        let bindir = DirInfo {
            dir_size: 0,
            file_number: 0,
        };
        let git_repos_bare = DirInfo {
            dir_size: 0,
            file_number: 0,
        };
        let git_checkouts = DirInfo {
            dir_size: 0,
            file_number: 0,
        };
        let reg_cache = DirInfo {
            dir_size: 0,
            file_number: 0,
        };
        let reg_src = DirInfo {
            dir_size: 0,
            file_number: 0,
        };
        let reg_index = DirInfo {
            dir_size: 0,
            file_number: 0,
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

Total size:                             0 B
Size of 0 installed binaries:             0 B
Size of registry:                         0 B
Size of 0 crate archives:                   0 B
Size of 0 crate source checkouts:           0 B
Size of git db:                           0 B
Size of 0 bare git repos:                   0 B
Size of 0 git repo checkouts:               0 B\n";

        assert_eq!(output_is, output_should);
    }
}
