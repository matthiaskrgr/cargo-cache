// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/// This file provides the `DirSize` struct which holds information on the sizes and the number of files of the cargo cache.
/// When constructing the struct, the caches from the cache modules are used.
/// The new() method does parallel processing to a bit of time
use std::fmt;

use crate::cache::caches::Cache;
use crate::cache::caches::RegistrySubCache;
use crate::cache::caches::RegistrySuperCache;

use crate::cache::*;
use crate::library::*;
use crate::tables::*;

use humansize::{file_size_opts, FileSize};

/// Holds the sizes and the number of files of the components of the cargo cache
// useful for saving a "snapshot" of the current state of the cache
#[derive(Debug)]
pub(crate) struct DirSizes<'a> {
    /// total size of the cache / .cargo rood directory
    total_size: u64,
    /// number of binaries found
    numb_bins: usize,
    /// total size of binaries
    total_bin_size: u64,
    /// total size of the registries (src + cache)
    total_reg_size: u64,
    /// total size of the git db (bare repos and checkouts)
    total_git_db_size: u64,
    /// total size of bare git repos
    total_git_repos_bare_size: u64,
    /// number of bare git repos
    numb_git_repos_bare_repos: usize,
    /// number of git checkouts (source checkouts)
    numb_git_checkouts: usize,
    /// total size of git checkouts
    total_git_chk_size: u64,
    /// total size of registry caches (.crates)
    total_reg_cache_size: u64,
    /// total size of registry sources (extracted .crates, .rs sourcefiles)
    total_reg_src_size: u64,
    /// total size of registry indices
    total_reg_index_size: u64,
    /// total number of registry indices
    total_reg_index_num: u64,
    /// number of source archives (.crates) // @TODO clarify
    numb_reg_cache_entries: usize,
    /// number of registry source checkouts// @TODO clarify
    numb_reg_src_checkouts: usize,
    /// root path of the cache
    root_path: &'a std::path::PathBuf,
}

impl<'a> DirSizes<'a> {
    /// create a new `DirSize` object by querying the caches for their data, done in parallel

    pub(crate) fn new(
        bin_cache: &mut bin::BinaryCache,
        checkouts_cache: &mut git_checkouts::GitCheckoutCache,
        bare_repos_cache: &mut git_bare_repos::GitRepoCache,
        registry_pkg_cache: &mut registry_pkg_cache::RegistryPkgCaches,
        registry_index_caches: &mut registry_index::RegistryIndicesCache,
        registry_sources_caches: &mut registry_sources::RegistrySourceCaches,
        ccd: &'a CargoCachePaths,
    ) -> Self {
        let mut reg_index_size: Option<u64> = None;
        let mut bin_dir_size: Option<u64> = None;
        let mut numb_bins: Option<usize> = None;
        let mut total_git_repos_bare_size: Option<u64> = None;
        let mut numb_git_repos_bare_repos: Option<usize> = None;
        let mut total_git_chk_size: Option<u64> = None;
        let mut numb_git_checkouts: Option<usize> = None;
        let mut total_reg_cache_size: Option<u64> = None;
        let mut total_reg_cache_entries: Option<usize> = None;
        let mut total_reg_src_size: Option<u64> = None;
        let mut numb_reg_src_checkouts: Option<usize> = None;

        rayon::scope(|s| {
            // spawn one thread per cache
            s.spawn(|_| reg_index_size = Some(registry_index_caches.total_size()));

            s.spawn(|_| {
                bin_dir_size = Some(bin_cache.total_size());
                numb_bins = Some(bin_cache.number_of_files());
            });

            s.spawn(|_| {
                total_git_repos_bare_size = Some(bare_repos_cache.total_size());
                numb_git_repos_bare_repos = Some(bare_repos_cache.number_of_items());
            });

            s.spawn(|_| {
                total_git_chk_size = Some(checkouts_cache.total_size());
                numb_git_checkouts = Some(checkouts_cache.number_of_items());
            });

            s.spawn(|_| {
                total_reg_cache_size = Some(registry_pkg_cache.total_size());
                total_reg_cache_entries = Some(registry_pkg_cache.total_number_of_files());
            });

            s.spawn(|_| {
                total_reg_src_size = Some(registry_sources_caches.total_size());
                numb_reg_src_checkouts = Some(registry_sources_caches.number_of_items());
            });
        });

        let root_path = &ccd.cargo_home;
        let total_reg_size =
            total_reg_cache_size.unwrap() + total_reg_src_size.unwrap() + reg_index_size.unwrap();
        let total_git_db_size = total_git_repos_bare_size.unwrap() + total_git_chk_size.unwrap();

        let total_bin_size = bin_dir_size.unwrap();

        let total_size = total_reg_size + total_git_db_size + total_bin_size;
        Self {
            total_size,                    // total size of cargo root dir
            numb_bins: numb_bins.unwrap(), // number of binaries found
            total_bin_size,                // total size of binaries found
            total_reg_size,                // registry size
            total_git_db_size,             // size of bare repos and checkouts combined
            total_git_repos_bare_size: total_git_repos_bare_size.unwrap(), // git db size
            numb_git_repos_bare_repos: numb_git_repos_bare_repos.unwrap(), // number of cloned repos
            numb_git_checkouts: numb_git_checkouts.unwrap(), // number of checked out repos
            total_git_chk_size: total_git_chk_size.unwrap(), // git checkout size
            total_reg_cache_size: total_reg_cache_size.unwrap(), // registry cache size
            total_reg_src_size: total_reg_src_size.unwrap(), // registry sources size
            total_reg_index_size: reg_index_size.unwrap(), // registry index size
            total_reg_index_num: registry_index_caches.number_of_subcaches() as u64, // number  of indices //@TODO parallelize like the rest
            numb_reg_cache_entries: total_reg_cache_entries.unwrap(), // number of source archives
            numb_reg_src_checkouts: numb_reg_src_checkouts.unwrap(),  // number of source checkouts
            root_path,
        }
    }

    pub(crate) fn total_size(&self) -> u64 {
        self.total_size
    }
    pub(crate) fn numb_bins(&self) -> usize {
        self.numb_bins
    }
    pub(crate) fn total_bin_size(&self) -> u64 {
        self.total_bin_size
    }
    pub(crate) fn total_reg_size(&self) -> u64 {
        self.total_reg_size
    }
    pub(crate) fn total_git_db_size(&self) -> u64 {
        self.total_git_db_size
    }
    pub(crate) fn total_git_repos_bare_size(&self) -> u64 {
        self.total_git_repos_bare_size
    }
    pub(crate) fn numb_git_repos_bare_repos(&self) -> usize {
        self.numb_git_repos_bare_repos
    }
    pub(crate) fn numb_git_checkouts(&self) -> usize {
        self.numb_git_checkouts
    }
    pub(crate) fn total_git_chk_size(&self) -> u64 {
        self.total_git_chk_size
    }
    pub(crate) fn total_reg_cache_size(&self) -> u64 {
        self.total_reg_cache_size
    }
    pub(crate) fn total_reg_src_size(&self) -> u64 {
        self.total_reg_src_size
    }
    pub(crate) fn total_reg_index_size(&self) -> u64 {
        self.total_reg_index_size
    }
    pub(crate) fn total_reg_index_num(&self) -> u64 {
        self.total_reg_index_num
    }
    pub(crate) fn numb_reg_cache_entries(&self) -> usize {
        self.numb_reg_cache_entries
    }
    pub(crate) fn numb_reg_src_checkouts(&self) -> usize {
        self.numb_reg_src_checkouts
    }
    pub(crate) fn root_path(&self) -> &'a std::path::PathBuf {
        self.root_path
    }
}

impl<'a> DirSizes<'a> {
    /// returns the header of the summary which contains the path to the cache and its total size
    fn header(&self) -> Vec<TableLine> {
        vec![
            TableLine::new(
                0,
                &format!("Cargo cache '{}':\n\n", &self.root_path().display()),
                &String::new(),
            ),
            TableLine::new(
                0,
                &"Total: ".to_string(),
                &self
                    .total_size()
                    .file_size(file_size_opts::DECIMAL)
                    .unwrap(),
            ),
        ]
    }

    /// returns amount and size of installed crate binaries
    fn bin(&self) -> Vec<TableLine> {
        vec![TableLine::new(
            1,
            &format!("{} installed binaries: ", self.numb_bins()),
            &self
                .total_bin_size()
                .file_size(file_size_opts::DECIMAL)
                .unwrap(),
        )]
    }

    /// returns amount and size of bare git repos and git repo checkouts
    fn git(&self) -> Vec<TableLine> {
        vec![
            TableLine::new(
                1,
                &"Git db: ".to_string(),
                &self
                    .total_git_db_size()
                    .file_size(file_size_opts::DECIMAL)
                    .unwrap(),
            ),
            TableLine::new(
                2,
                &format!("{} bare git repos: ", self.numb_git_repos_bare_repos()),
                &self
                    .total_git_repos_bare_size()
                    .file_size(file_size_opts::DECIMAL)
                    .unwrap(),
            ),
            TableLine::new(
                2,
                &format!("{} git repo checkouts: ", self.numb_git_checkouts()),
                &self
                    .total_git_chk_size()
                    .file_size(file_size_opts::DECIMAL)
                    .unwrap(),
            ),
        ]
    }

    /// returns summary of sizes of registry indices and registries (both, .crate archives and the extracted sources)
    fn registries_summary(&self) -> Vec<TableLine> {
        let tl1 = TableLine::new(
            1,
            &"Registry: ".to_string(),
            &self
                .total_reg_size()
                .file_size(file_size_opts::DECIMAL)
                .unwrap(),
        );

        let left = if let 1 = self.total_reg_index_num {
            String::from("Registry index: ")
        } else {
            format!("{} registry indices: ", &self.total_reg_index_num())
        };
        let tl2 = TableLine::new(
            2,
            &left,
            &self
                .total_reg_index_size()
                .file_size(file_size_opts::DECIMAL)
                .unwrap(),
        );

        let tl3 = TableLine::new(
            2,
            &format!("{} crate archives: ", self.numb_reg_cache_entries()),
            &self
                .total_reg_cache_size()
                .file_size(file_size_opts::DECIMAL)
                .unwrap(),
        );

        let tl4 = TableLine::new(
            2,
            &format!("{} crate source checkouts: ", self.numb_reg_src_checkouts()),
            &self
                .total_reg_src_size()
                .file_size(file_size_opts::DECIMAL)
                .unwrap(),
        );

        vec![tl1, tl2, tl3, tl4]
    }

    /// returns more detailed summary about each registry
    fn registries_seperate(
        &self,
        index_caches: &mut registry_index::RegistryIndicesCache,
        registry_sources: &mut registry_sources::RegistrySourceCaches,
        pkg_caches: &mut registry_pkg_cache::RegistryPkgCaches,
    ) -> Vec<TableLine> {
        let mut v: Vec<TableLine> = vec![];

        // we need to match the separate registries together somehow
        // do this by folder names
        let mut registries: Vec<String> = vec![];
        index_caches.caches().iter().for_each(|registry| {
            registries.push(
                registry
                    .path()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
            );
        });

        pkg_caches.caches().iter().for_each(|registry| {
            registries.push(
                registry
                    .path()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
            );
        });

        registry_sources.caches().iter().for_each(|registry| {
            registries.push(
                registry
                    .path()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
            );
        });
        // we now collected all the folder names of the registries and can match a single registry across multiple
        // caches by this

        /*
          Registry:                         1.52 GB
            5 registry indices:           250.20 MB
            5399 crate archives:          805.46 MB
            901 crate source checkouts:   460.77 MB
        */

        registries.sort();
        registries.dedup();

        for registry in &registries {
            let mut total_size = 0;

            let mut temp_vec: Vec<TableLine> = Vec::new();
            let mut registry_name: Option<String> = None;

            for index in index_caches.caches().iter_mut().filter(|r| {
                &r.path().file_name().unwrap().to_str().unwrap().to_string() == registry
            }) {
                temp_vec.push(TableLine::new(
                    2,
                    &String::from("Registry index:"),
                    &index
                        .total_size()
                        .file_size(file_size_opts::DECIMAL)
                        .unwrap(),
                ));
                total_size += index.total_size();
                if registry_name.is_none() {
                    registry_name = Some(index.name().into());
                }
            }

            for pkg_cache in pkg_caches.caches().iter_mut().filter(|p| {
                &p.path().file_name().unwrap().to_str().unwrap().to_string() == registry
            }) {
                temp_vec.push(TableLine::new(
                    2,
                    &format!("{} crate archives: ", pkg_cache.number_of_files()),
                    &pkg_cache
                        .total_size()
                        .file_size(file_size_opts::DECIMAL)
                        .unwrap(),
                ));
                total_size += pkg_cache.total_size();
                if registry_name.is_none() {
                    registry_name = Some(pkg_cache.name().into());
                }
            }

            for registry_source in registry_sources.caches().iter_mut().filter(|s| {
                &s.path().file_name().unwrap().to_str().unwrap().to_string() == registry
            }) {
                temp_vec.push(TableLine::new(
                    2,
                    &format!(
                        "{} crate source checkouts: ",
                        registry_source.number_of_items()
                    ),
                    &registry_source
                        .total_size()
                        .file_size(file_size_opts::DECIMAL)
                        .unwrap(),
                ));
                total_size += registry_source.total_size();
                if registry_name.is_none() {
                    registry_name = Some(registry_source.name().into());
                }
            }

            let header_line = TableLine::new(
                1,
                &format!("Registry: {}", registry_name.unwrap_or_default()),
                &total_size.file_size(file_size_opts::DECIMAL).unwrap(),
            );

            v.push(header_line);
            v.extend(temp_vec);
        }

        v
    } // registries separate

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn print_size_difference(
        cache_sizes_old: &DirSizes<'_>,
        cargo_cache: &CargoCachePaths,
        bin_cache: &mut bin::BinaryCache,
        checkouts_cache: &mut git_checkouts::GitCheckoutCache,
        bare_repos_cache: &mut git_bare_repos::GitRepoCache,
        registry_pkgs_cache: &mut registry_pkg_cache::RegistryPkgCaches,
        registry_index_caches: &mut registry_index::RegistryIndicesCache,
        registry_sources_caches: &mut registry_sources::RegistrySourceCaches,
    ) {
        // Total:           x Mb => y MB
        fn cmp_total(old: &DirSizes<'_>, new: &DirSizes<'_>) -> Vec<TableLine> {
            vec![
                TableLine::new(
                    0,
                    &format!("Cargo cache '{}':\n\n", &old.root_path().display()),
                    &String::new(),
                ),
                TableLine::new(
                    0,
                    &"Total: ".to_string(),
                    &if old.total_size() == new.total_size() {
                        old.total_size().file_size(file_size_opts::DECIMAL).unwrap()
                    } else {
                        format!(
                            "{} => {}",
                            &old.total_size().file_size(file_size_opts::DECIMAL).unwrap(),
                            &new.total_size().file_size(file_size_opts::DECIMAL).unwrap()
                        )
                    },
                ),
            ]
        }
        // binars are not  supposed to change, we can use the ::bins  function here

        fn git(old: &DirSizes<'_>, new: &DirSizes<'_>) -> Vec<TableLine> {
            vec![
                TableLine::new(
                    1,
                    &"Git db: ".to_string(),
                    &if old.total_git_db_size() == new.total_git_db_size() {
                        new.total_git_db_size()
                            .file_size(file_size_opts::DECIMAL)
                            .unwrap()
                    } else {
                        format!(
                            "{} => {}",
                            &old.total_git_db_size()
                                .file_size(file_size_opts::DECIMAL)
                                .unwrap(),
                            &new.total_git_db_size()
                                .file_size(file_size_opts::DECIMAL)
                                .unwrap()
                        )
                    },
                ),
                TableLine::new(
                    2,
                    &if old.numb_git_repos_bare_repos() == new.numb_git_repos_bare_repos() {
                        format!("{} bare git repos:", new.numb_git_repos_bare_repos())
                    } else {
                        format!(
                            "{} => {} bare git repos: ",
                            &old.numb_git_repos_bare_repos(),
                            &new.numb_git_repos_bare_repos()
                        )
                    },
                    &if old.total_git_repos_bare_size() == new.total_git_repos_bare_size() {
                        new.total_git_repos_bare_size()
                            .file_size(file_size_opts::DECIMAL)
                            .unwrap()
                    } else {
                        format!(
                            "{} => {}",
                            &old.total_git_repos_bare_size()
                                .file_size(file_size_opts::DECIMAL)
                                .unwrap(),
                            &new.total_git_repos_bare_size()
                                .file_size(file_size_opts::DECIMAL)
                                .unwrap()
                        )
                    },
                ),
                TableLine::new(
                    2,
                    &if old.numb_git_checkouts() == new.numb_git_checkouts() {
                        format!("{} git repo checkouts: ", new.numb_git_checkouts())
                    } else {
                        format!(
                            "{} => {} git repo checkouts: ",
                            &old.numb_git_checkouts(),
                            &new.numb_git_checkouts()
                        )
                    },
                    &if old.total_git_chk_size() == new.total_git_chk_size() {
                        new.total_git_chk_size()
                            .file_size(file_size_opts::DECIMAL)
                            .unwrap()
                    } else {
                        format!(
                            "{} => {}",
                            &old.total_git_chk_size()
                                .file_size(file_size_opts::DECIMAL)
                                .unwrap(),
                            &new.total_git_chk_size()
                                .file_size(file_size_opts::DECIMAL)
                                .unwrap()
                        )
                    },
                ),
            ]
        }

        fn regs(old: &DirSizes<'_>, new: &DirSizes<'_>) -> Vec<TableLine> {
            let tl1 = TableLine::new(
                1,
                &"Registry: ".to_string(),
                &if old.total_reg_size() == new.total_reg_size() {
                    new.total_reg_size()
                        .file_size(file_size_opts::DECIMAL)
                        .unwrap()
                } else {
                    format!(
                        "{} => {}",
                        &old.total_reg_size()
                            .file_size(file_size_opts::DECIMAL)
                            .unwrap(),
                        &new.total_reg_size()
                            .file_size(file_size_opts::DECIMAL)
                            .unwrap()
                    )
                },
            );

            let tl2 = TableLine::new(
                2,
                &if let 1 = &old.total_reg_index_num {
                    String::from("Registry index: ")
                } else {
                    format!("{} registry indices: ", &old.total_reg_index_num())
                },
                &if old.total_reg_index_size() == new.total_reg_index_size() {
                    old.total_reg_index_size()
                        .file_size(file_size_opts::DECIMAL)
                        .unwrap()
                } else {
                    format!(
                        "{} => {}",
                        &old.total_reg_index_size()
                            .file_size(file_size_opts::DECIMAL)
                            .unwrap(),
                        &new.total_reg_index_size()
                            .file_size(file_size_opts::DECIMAL)
                            .unwrap()
                    )
                },
            );

            let tl3 = TableLine::new(
                2,
                &if old.numb_reg_cache_entries() == new.numb_reg_cache_entries() {
                    format!("{} crate archives: ", new.numb_reg_cache_entries())
                } else {
                    format!(
                        "{} => {} crate archives: ",
                        &old.numb_reg_cache_entries(),
                        &new.numb_reg_cache_entries()
                    )
                },
                &if old.total_reg_cache_size() == new.total_reg_cache_size() {
                    new.total_reg_cache_size()
                        .file_size(file_size_opts::DECIMAL)
                        .unwrap()
                } else {
                    format!(
                        "{} => {}",
                        &old.total_reg_cache_size()
                            .file_size(file_size_opts::DECIMAL)
                            .unwrap(),
                        &new.total_reg_cache_size()
                            .file_size(file_size_opts::DECIMAL)
                            .unwrap(),
                    )
                },
            );

            let tl4 = TableLine::new(
                2,
                &if old.numb_reg_src_checkouts() == new.numb_reg_src_checkouts() {
                    format!("{} crate source checkouts: ", new.numb_reg_src_checkouts())
                } else {
                    format!(
                        "{} => {} crate source checkouts: ",
                        &old.numb_reg_src_checkouts(),
                        &new.numb_reg_src_checkouts()
                    )
                },
                &if old.total_reg_src_size() == new.total_reg_src_size() {
                    old.total_reg_src_size()
                        .file_size(file_size_opts::DECIMAL)
                        .unwrap()
                } else {
                    format!(
                        "{} => {}",
                        &old.total_reg_src_size()
                            .file_size(file_size_opts::DECIMAL)
                            .unwrap(),
                        &new.total_reg_src_size()
                            .file_size(file_size_opts::DECIMAL)
                            .unwrap(),
                    )
                },
            );

            vec![tl1, tl2, tl3, tl4]
        } // fn regs()

        // and requery it to let it do its thing
        let cache_sizes_new = DirSizes::new(
            bin_cache,
            checkouts_cache,
            bare_repos_cache,
            registry_pkgs_cache,
            registry_index_caches,
            registry_sources_caches,
            cargo_cache,
        );

        let mut v = Vec::new();
        v.extend(cmp_total(cache_sizes_old, &cache_sizes_new));
        v.extend(cache_sizes_new.bin());
        v.extend(regs(cache_sizes_old, &cache_sizes_new));
        v.extend(git(cache_sizes_old, &cache_sizes_new));

        let mut summary = two_row_table(3, v, false);

        let total_size_old = cache_sizes_old.total_size();
        let total_size_new = cache_sizes_new.total_size();

        // only show final summary if something changed
        if total_size_old != total_size_new {
            summary.push('\n');

            // final summary line
            let final_line = format!(
                "Size changed {}",
                size_diff_format(total_size_old, total_size_new, true)
            );
            summary.push_str(&final_line);
        }

        println!("{}", summary);
    }
} // print_size_difference()

impl<'a> fmt::Display for DirSizes<'a> {
    /// returns the default summary of cargo-cache (cmd: "cargo cache")
    fn fmt(&self, f: &'_ mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table: Vec<TableLine> = vec![];
        table.extend(self.header());
        table.extend(self.bin());
        table.extend(self.registries_summary());
        table.extend(self.git());

        let string: String = two_row_table(2, table, false);

        write!(f, "{}", string)?;
        Ok(())
    }
}

/// returns a summary with details on each registry (cmd: "cargo cache registry")
pub(crate) fn per_registry_summary(
    dir_size: &DirSizes<'_>,
    index_caches: &mut registry_index::RegistryIndicesCache,
    pkg_caches: &mut registry_sources::RegistrySourceCaches,
    registry_sources: &mut registry_pkg_cache::RegistryPkgCaches,
) -> String {
    let mut table: Vec<TableLine> = vec![];
    table.extend(dir_size.header());
    table.extend(dir_size.bin());
    table.extend(dir_size.registries_seperate(index_caches, pkg_caches, registry_sources));
    table.extend(dir_size.git());

    two_row_table(2, table, false)
}

#[cfg(test)]
mod libtests {
    use super::*;

    use pretty_assertions::assert_eq;
    use std::path::PathBuf;

    impl<'a> DirSizes<'a> {
        #[allow(clippy::cast_possible_truncation, clippy::ptr_arg)]
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
                total_reg_index_num: 1,
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

Total:                                    1.94 GB
  31 installed binaries:                121.21 KB
  Registry:                               1.94 GB
    Registry index:                         23  B
    23445 crate archives:                   89  B
    123909849 crate source checkouts:     1.94 GB
  Git db:                               156.20 KB
    37 bare git repos:                  121.21 KB
    8 git repo checkouts:                34.98 KB\n";

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

Total:                               6.33 GB
  69 installed binaries:           640.16 MB
  Registry:                          1.46 GB
    Registry index:                    23  B
    3654 crate archives:           550.86 MB
    1615 crate source checkouts:   905.60 MB
  Git db:                            4.23 GB
    123 bare git repos:            309.61 MB
    36 git repo checkouts:           3.92 GB\n";

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

Total:                           14.57 GB
  0 installed binaries:              0  B
  Registry:                      14.57 GB
    Registry index:               1.25 GB
    4 crate archives:            13.04 GB
    4 crate source checkouts:   268.46 MB
  Git db:                            0  B
    0 bare git repos:                0  B
    0 git repo checkouts:            0  B\n";

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

Total:                          0  B
  0 installed binaries:         0  B
  Registry:                     0  B
    Registry index:             0  B
    0 crate archives:           0  B
    0 crate source checkouts:   0  B
  Git db:                       0  B
    0 bare git repos:           0  B
    0 git repo checkouts:       0  B\n";

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
            let _ = black_box(x);
        });
    }
}
