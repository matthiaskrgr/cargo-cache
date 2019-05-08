// Copyright 2017-2019 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;

use crate::cache::dircache::Cache;
use crate::cache::*;
use crate::library;
use crate::library::*;

use humansize::{file_size_opts, FileSize};

#[derive(Debug)]
pub(crate) struct DirSizes<'a> {
    pub(crate) total_size: u64,                // total size of cargo root dir
    pub(crate) numb_bins: usize,               // number of binaries found
    pub(crate) total_bin_size: u64,            // total size of binaries found
    pub(crate) total_reg_size: u64,            // registry size
    pub(crate) total_git_db_size: u64,         // size of bare repos and checkouts combined
    pub(crate) total_git_repos_bare_size: u64, // git db size
    pub(crate) numb_git_repos_bare_repos: usize, // number of cloned repos
    pub(crate) numb_git_checkouts: usize,      // number of checked out repos
    pub(crate) total_git_chk_size: u64,        // git checkout size
    pub(crate) total_reg_cache_size: u64,      // registry cache size
    pub(crate) total_reg_src_size: u64,        // registry sources size
    pub(crate) total_reg_index_size: u64,      // registry index size
    pub(crate) numb_reg_cache_entries: usize,  // number of source archives
    pub(crate) numb_reg_src_checkouts: usize,  // number of source checkouts
    pub(crate) root_path: &'a std::path::PathBuf,
}

impl<'a> DirSizes<'a> {
    pub(crate) fn new(
        bin_cache: &mut bin::BinaryCache,
        checkouts_cache: &mut git_checkouts::GitCheckoutCache,
        bare_repos_cache: &mut git_repos_bare::GitRepoCache,
        registry_pkg_cache: &mut registry_pkg_cache::RegistryCache,
        registry_index_cache: &mut registry_index::RegistryIndexCache,
        registry_sources_cache: &mut registry_sources::RegistrySourceCache,
        ccd: &'a CargoCachePaths,
    ) -> Self {
        #[allow(clippy::type_complexity)]
        let (
            (
                reg_index_size,
                ((bin_dir_size, numb_bins), (total_git_repos_bare_size, numb_git_repos_bare_repos)),
            ),
            (
                (total_git_chk_size, numb_git_checkouts),
                (
                    (total_reg_cache_size, total_reg_cache_entries),
                    (total_reg_src_size, numb_reg_src_checkouts),
                ),
            ),
        ): (
            (u64, ((u64, usize), (u64, usize))),
            ((u64, usize), ((u64, usize), (u64, usize))),
        ) = rayon::join(
            || {
                rayon::join(
                    || registry_index_cache.total_size(),
                    || {
                        rayon::join(
                            || (bin_cache.total_size(), bin_cache.number_of_files()),
                            || {
                                (
                                    bare_repos_cache.total_size(),
                                    bare_repos_cache.number_of_checkout_repos().unwrap(),
                                )
                            },
                        )
                    },
                )
            },
            || {
                rayon::join(
                    || {
                        (
                            checkouts_cache.total_size(),
                            checkouts_cache.number_of_files_at_depth_2(),
                        )
                    },
                    || {
                        rayon::join(
                            || {
                                (
                                    registry_pkg_cache.total_size(),
                                    registry_pkg_cache.number_of_files(),
                                )
                            },
                            || {
                                (
                                    registry_sources_cache.total_size(),
                                    registry_sources_cache.number_of_files_at_depth_2(),
                                )
                            },
                        )
                    },
                )
            },
        );

        let root_path = &ccd.cargo_home;
        let total_reg_size = total_reg_cache_size + total_reg_src_size + reg_index_size;
        let total_git_db_size = total_git_repos_bare_size + total_git_chk_size;

        let total_bin_size = bin_dir_size;

        let total_size = total_reg_size + total_git_db_size + total_bin_size;
        Self {
            total_size,                                      // total size of cargo root dir
            numb_bins,                                       // number of binaries found
            total_bin_size,                                  // total size of binaries found
            total_reg_size,                                  // registry size
            total_git_db_size,         // size of bare repos and checkouts combined
            total_git_repos_bare_size, // git db size
            numb_git_repos_bare_repos, // number of cloned repos
            numb_git_checkouts,        // number of checked out repos
            total_git_chk_size,        // git checkout size
            total_reg_cache_size,      // registry cache size
            total_reg_src_size,        // registry sources size
            total_reg_index_size: reg_index_size, // registry index size
            numb_reg_cache_entries: total_reg_cache_entries, // number of source archives
            numb_reg_src_checkouts,    // number of source checkouts
            root_path,
        }
    }
}

impl<'a> fmt::Display for DirSizes<'a> {
    fn fmt(&self, f: &'_ mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cargo cache '{}':\n\n", &self.root_path.display())?;

        write!(
            f,
            "{}",
            library::pad_strings(
                0,
                40,
                "Total size: ",
                &self.total_size.file_size(file_size_opts::DECIMAL).unwrap(),
            )
        )?;

        write!(
            f,
            "{}",
            library::pad_strings(
                1,
                40,
                &format!("Size of {} installed binaries: ", self.numb_bins),
                &self
                    .total_bin_size
                    .file_size(file_size_opts::DECIMAL)
                    .unwrap(),
            )
        )?;

        write!(
            f,
            "{}",
            library::pad_strings(
                1,
                40,
                "Size of registry: ",
                &self
                    .total_reg_size
                    .file_size(file_size_opts::DECIMAL)
                    .unwrap(),
            )
        )?;

        write!(
            f,
            "{}",
            library::pad_strings(
                2,
                40,
                "Size of registry index: ",
                &self
                    .total_reg_index_size
                    .file_size(file_size_opts::DECIMAL)
                    .unwrap(),
            )
        )?;

        write!(
            f,
            "{}",
            library::pad_strings(
                2,
                40,
                &format!("Size of {} crate archives: ", self.numb_reg_cache_entries),
                &self
                    .total_reg_cache_size
                    .file_size(file_size_opts::DECIMAL)
                    .unwrap(),
            )
        )?;

        write!(
            f,
            "{}",
            library::pad_strings(
                2,
                40,
                &format!(
                    "Size of {} crate source checkouts: ",
                    self.numb_reg_src_checkouts
                ),
                &self
                    .total_reg_src_size
                    .file_size(file_size_opts::DECIMAL)
                    .unwrap(),
            )
        )?;

        write!(
            f,
            "{}",
            library::pad_strings(
                1,
                40,
                "Size of git db: ",
                &self
                    .total_git_db_size
                    .file_size(file_size_opts::DECIMAL)
                    .unwrap(),
            )
        )?;

        write!(
            f,
            "{}",
            library::pad_strings(
                2,
                40,
                &format!(
                    "Size of {} bare git repos: ",
                    self.numb_git_repos_bare_repos
                ),
                &self
                    .total_git_repos_bare_size
                    .file_size(file_size_opts::DECIMAL)
                    .unwrap(),
            )
        )?;

        write!(
            f,
            "{}",
            library::pad_strings(
                2,
                40,
                &format!("Size of {} git repo checkouts: ", self.numb_git_checkouts),
                &self
                    .total_git_chk_size
                    .file_size(file_size_opts::DECIMAL)
                    .unwrap(),
            )
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod libtests {
    use super::*;

    use pretty_assertions::assert_eq;
    use std::path::PathBuf;

    impl<'a> DirSizes<'a> {
        #[allow(clippy::cast_possible_truncation)]
        #[allow(non_snake_case)]
        pub(super) fn new_manually(
            DI_bindir: &DirInfo,
            DI_git_repos_bare: &DirInfo,
            DI_git_checkout: &DirInfo,
            DI_reg_cache: &DirInfo,
            DI_reg_src: &DirInfo,
            DI_reg_index: &DirInfo,
            path: &'a PathBuf,
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
                // no need to recompute all of this from scratch
                total_size: total_reg_size + total_git_db_size + bindir.dir_size,
                numb_bins: bindir.file_number as usize,
                total_bin_size: bindir.dir_size,
                total_reg_size,

                total_git_db_size,
                total_git_repos_bare_size: git_repos_bare.dir_size,
                numb_git_repos_bare_repos: git_repos_bare.file_number as usize,

                total_git_chk_size: git_checkouts.dir_size,
                numb_git_checkouts: git_checkouts.file_number as usize,

                total_reg_cache_size: reg_cache.dir_size,
                numb_reg_cache_entries: reg_cache.file_number as usize,

                total_reg_src_size: reg_src.dir_size,
                numb_reg_src_checkouts: reg_src.file_number as usize,

                total_reg_index_size: reg_index.dir_size,
                root_path: path,
            }
        }
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

        let pb = PathBuf::from("/home/user/.cargo");

        // create a DirSizes object
        let dirSizes = DirSizes::new_manually(
            &bindir,
            &git_repos_bare,
            &git_checkouts,
            &reg_cache,
            &reg_src,
            &reg_index,
            &pb,
        );

        let output_is = format!("{}", dirSizes);

        let output_should = "Cargo cache '/home/user/.cargo':

Total size:                             1.94 GB
Size of 31 installed binaries:            121.21 KB
Size of registry:                         1.94 GB
Size of registry index:                     23 B
Size of 23445 crate archives:               89 B
Size of 123909849 crate source checkouts:   1.94 GB
Size of git db:                           156.20 KB
Size of 37 bare git repos:                  121.21 KB
Size of 8 git repo checkouts:               34.98 KB\n";

        assert_eq!(output_is, output_should);
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

        let pb = PathBuf::from("/home/user/.cargo");
        // create a DirSizes object
        let dirSizes = DirSizes::new_manually(
            &bindir,
            &git_repos_bare,
            &git_checkouts,
            &reg_cache,
            &reg_src,
            &reg_index,
            &pb,
        );

        let output_is = format!("{}", dirSizes);

        let output_should = "Cargo cache '/home/user/.cargo':

Total size:                             6.33 GB
Size of 69 installed binaries:            640.16 MB
Size of registry:                         1.46 GB
Size of registry index:                     23 B
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

        let pb = PathBuf::from("/home/user/.cargo");

        // create a DirSizes object
        let dirSizes = DirSizes::new_manually(
            &bindir,
            &git_repos_bare,
            &git_checkouts,
            &reg_cache,
            &reg_src,
            &reg_index,
            &pb,
        );

        let output_is = format!("{}", dirSizes);

        let output_should = "Cargo cache '/home/user/.cargo':

Total size:                             14.57 GB
Size of 0 installed binaries:             0 B
Size of registry:                         14.57 GB
Size of registry index:                     1.25 GB
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

        let pb = PathBuf::from("/home/user/.cargo");

        // create a DirSizes object
        let dirSizes = DirSizes::new_manually(
            &bindir,
            &git_repos_bare,
            &git_checkouts,
            &reg_cache,
            &reg_src,
            &reg_index,
            &pb,
        );

        let output_is = format!("{}", &dirSizes);

        let output_should = "Cargo cache '/home/user/.cargo':

Total size:                             0 B
Size of 0 installed binaries:             0 B
Size of registry:                         0 B
Size of registry index:                     0 B
Size of 0 crate archives:                   0 B
Size of 0 crate source checkouts:           0 B
Size of git db:                           0 B
Size of 0 bare git repos:                   0 B
Size of 0 git repo checkouts:               0 B\n";

        assert_eq!(output_is, output_should);
    }
}

#[cfg(all(test, feature = "bench"))]
mod benchmarks {
    use super::*;
    use crate::test::black_box;
    use crate::test::Bencher;
    use std::path::PathBuf;

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

        let pb = PathBuf::from("/home/user/.cargo");
        // create a DirSizes object
        let dir_sizes = DirSizes::new_manually(
            &bindir,
            &git_repos_bare,
            &git_checkouts,
            &reg_cache,
            &reg_src,
            &reg_index,
            &pb,
        );

        b.iter(|| {
            let x = format!("{}", dir_sizes);
            black_box(x);
        });
    }

}
