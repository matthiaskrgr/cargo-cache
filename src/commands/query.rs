// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cache::caches::{Cache, RegistrySuperCache};
use crate::cache::*;
use crate::library::Error;

use clap::ArgMatches;
use humansize::{file_size_opts, FileSize};
use rayon::prelude::*;
use regex::Regex;
use walkdir::WalkDir;

#[derive(Debug)]
struct File<'a> {
    #[allow(unused)]
    path: &'a Path,
    name: String,
    size: u64,
}

#[inline]
fn path_to_name_stemmed(path: &Path) -> String {
    path.file_stem()
        .unwrap()
        .to_str()
        .unwrap_or_default()
        .to_string()
}

#[inline]
fn path_to_name_unstemmed(path: &Path) -> String {
    path.file_name()
        .unwrap()
        .to_str()
        .unwrap_or_default()
        .to_string()
}

fn binary_to_file(path: &Path) -> File<'_> {
    File {
        path,
        name: path_to_name_unstemmed(path),
        size: fs::metadata(path)
            .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &path.display()))
            .len(),
    }
}

fn git_checkout_to_file(path: &Path) -> File<'_> {
    File {
        path,
        name: path_to_name_unstemmed(path),
        size: WalkDir::new(path.display().to_string())
            .into_iter()
            .map(|d| d.unwrap().into_path())
            .filter(|f| f.exists())
            .collect::<Vec<PathBuf>>()
            .par_iter()
            .map(|f| {
                fs::metadata(f)
                    .unwrap_or_else(|_| panic!("Failed to read size of file: '{:?}'", f))
                    .len()
            })
            .sum(),
    }
}

fn bare_repo_to_file(path: &Path) -> File<'_> {
    File {
        path,
        name: path_to_name_unstemmed(path),
        size: WalkDir::new(path.display().to_string())
            .into_iter()
            .map(|d| d.unwrap().into_path())
            .filter(|f| f.exists())
            .collect::<Vec<PathBuf>>()
            .par_iter()
            .map(|f| {
                fs::metadata(f)
                    .unwrap_or_else(|_| panic!("Failed to read size of file: '{:?}'", f))
                    .len()
            })
            .sum(),
    }
}

fn registry_pkg_cache_to_file(path: &Path) -> File<'_> {
    File {
        // todo: sum up the versions
        path,
        name: path_to_name_stemmed(path),
        size: WalkDir::new(path.display().to_string())
            .into_iter()
            .map(|d| d.unwrap().into_path())
            .filter(|f| f.exists())
            .collect::<Vec<PathBuf>>()
            .par_iter()
            .map(|f| {
                fs::metadata(f)
                    .unwrap_or_else(|_| panic!("Failed to read size of file: '{:?}'", f))
                    .len()
            })
            .sum(),
    }
}

fn registry_source_cache_to_file(path: &Path) -> File<'_> {
    File {
        // todo: sum up the versions
        path,
        name: path_to_name_unstemmed(path),
        size: WalkDir::new(path.display().to_string())
            .into_iter()
            .map(|d| d.unwrap().into_path())
            .filter(|f| f.exists())
            .collect::<Vec<PathBuf>>()
            .par_iter()
            .map(|f| {
                fs::metadata(f)
                    .unwrap_or_else(|_| panic!("Failed to read size of file: '{:?}'", f))
                    .len()
            })
            .sum(),
    }
}

fn sort_files_by_name(v: &mut [File<'_>]) {
    v.sort_by_key(|f| f.name.clone() /* @TODO: don't clone*/);
}

fn sort_files_by_size(v: &mut [File<'_>]) {
    v.sort_by_key(|f| f.size);
}

pub(crate) fn run_query(
    query_config: &ArgMatches,
    bin_cache: &mut bin::BinaryCache,
    checkouts_cache: &mut git_checkouts::GitCheckoutCache,
    bare_repos_cache: &mut git_bare_repos::GitRepoCache,
    registry_pkg_cache: &mut registry_pkg_cache::RegistryPkgCaches,
    registry_sources_caches: &mut registry_sources::RegistrySourceCaches,
) -> Result<(), Error> {
    let sorting = query_config.value_of("sort");
    let query = query_config.value_of("QUERY").unwrap_or("" /* default */);
    let hr_size = query_config.is_present("hr");

    let mut output = String::new();

    // make the regex
    let re = match Regex::new(query) {
        Ok(re) => re,
        Err(_e) => {
            return Err(Error::QueryRegexFailedParsing(query.to_string()));
        }
    };

    let mut binary_matches: Vec<File<'_>> = bin_cache
        .files()
        .iter()
        .map(|path| binary_to_file(path)) // convert the path into a file struct
        .filter(|f| re.is_match(f.name.as_str())) // filter by regex
        .collect::<Vec<_>>();

    let mut git_checkout_matches: Vec<_> = checkouts_cache
        .items()
        .iter()
        .map(|path| git_checkout_to_file(path))
        .filter(|f| re.is_match(f.name.as_str())) // filter by regex
        .collect::<Vec<_>>();

    let mut bare_repos_matches: Vec<_> = bare_repos_cache
        .items()
        .iter()
        .map(|path| bare_repo_to_file(path))
        .filter(|f| re.is_match(f.name.as_str())) // filter by regex
        .collect::<Vec<_>>();

    let files = registry_pkg_cache.files(); //@TODO fixme?
    let mut registry_pkg_cache_matches: Vec<_> = files
        .iter()
        .map(|path| registry_pkg_cache_to_file(path))
        .filter(|f| re.is_match(f.name.as_str())) // filter by regex
        .collect::<Vec<_>>();

    let mut registry_source_caches_matches: Vec<_> = registry_sources_caches
        .items()
        .iter()
        .map(|path| registry_source_cache_to_file(path))
        .filter(|f| re.is_match(f.name.as_str())) // filter by regex
        .collect::<Vec<_>>();

    let humansize_opts = file_size_opts::FileSizeOpts {
        allow_negative: true,
        ..file_size_opts::DECIMAL
    };

    match sorting {
        // make "name" the default
        Some("name") | None => {
            // executables
            if !binary_matches.is_empty() {
                sort_files_by_name(&mut binary_matches);
                output.push_str("Binaries sorted by name:\n");
                binary_matches.iter().for_each(|b| {
                    let size = if hr_size {
                        b.size.file_size(&humansize_opts).unwrap()
                    } else {
                        b.size.to_string()
                    };
                    writeln!(output, "\t{}: {}", b.name, size).unwrap();
                });
            }

            // git checkouts
            if !git_checkout_matches.is_empty() {
                sort_files_by_name(&mut git_checkout_matches);
                output.push_str("\nGit checkouts sorted by name:\n");
                git_checkout_matches.iter().for_each(|b| {
                    let size = if hr_size {
                        b.size.file_size(&humansize_opts).unwrap()
                    } else {
                        b.size.to_string()
                    };
                    writeln!(output, "\t{}: {}", b.name, size).unwrap();
                });
            }
            // bare git repos
            if !bare_repos_matches.is_empty() {
                sort_files_by_name(&mut bare_repos_matches);
                output.push_str("\nBare git repos sorted by name:\n");
                bare_repos_matches.iter().for_each(|b| {
                    let size = if hr_size {
                        b.size.file_size(&humansize_opts).unwrap()
                    } else {
                        b.size.to_string()
                    };
                    writeln!(output, "\t{}: {}", b.name, size).unwrap();
                });
            }

            // registry cache
            if !registry_pkg_cache_matches.is_empty() {
                sort_files_by_name(&mut registry_pkg_cache_matches);
                output.push_str("\nRegistry cache sorted by name:\n");
                registry_pkg_cache_matches.iter().for_each(|b| {
                    let size = if hr_size {
                        b.size.file_size(&humansize_opts).unwrap()
                    } else {
                        b.size.to_string()
                    };
                    writeln!(output, "\t{}: {}", b.name, size).unwrap();
                });
            }

            // registry source
            if !registry_source_caches_matches.is_empty() {
                sort_files_by_name(&mut registry_source_caches_matches);
                output.push_str("\nRegistry source cache sorted by name:\n");
                registry_source_caches_matches.iter().for_each(|b| {
                    let size = if hr_size {
                        b.size.file_size(&humansize_opts).unwrap()
                    } else {
                        b.size.to_string()
                    };
                    writeln!(output, "\t{}: {}", b.name, size).unwrap();
                });
            }
        }

        Some("size") => {
            // executables
            if !binary_matches.is_empty() {
                sort_files_by_size(&mut binary_matches);
                output.push_str("\nBinaries sorted by size:\n");
                binary_matches.iter().for_each(|b| {
                    let size = if hr_size {
                        b.size.file_size(&humansize_opts).unwrap()
                    } else {
                        b.size.to_string()
                    };
                    writeln!(output, "\t{}: {}", b.name, size).unwrap();
                });
            }

            // git checkouts
            if !git_checkout_matches.is_empty() {
                sort_files_by_size(&mut git_checkout_matches);
                output.push_str("\nGit checkouts sorted by size:\n");
                git_checkout_matches.iter().for_each(|b| {
                    let size = if hr_size {
                        b.size.file_size(&humansize_opts).unwrap()
                    } else {
                        b.size.to_string()
                    };
                    writeln!(output, "\t{}: {}", b.name, size).unwrap();
                });
            }

            //bare repos matches
            if !bare_repos_matches.is_empty() {
                sort_files_by_size(&mut bare_repos_matches);
                output.push_str("\nBare git repos sorted by size:\n");
                bare_repos_matches.iter().for_each(|b| {
                    let size = if hr_size {
                        b.size.file_size(&humansize_opts).unwrap()
                    } else {
                        b.size.to_string()
                    };
                    writeln!(output, "\t{}: {}", b.name, size).unwrap();
                });
            }

            // registry cache
            if !registry_pkg_cache_matches.is_empty() {
                sort_files_by_size(&mut registry_pkg_cache_matches);
                output.push_str("\nRegistry cache sorted by size:\n");
                registry_pkg_cache_matches.iter().for_each(|b| {
                    let size = if hr_size {
                        b.size.file_size(&humansize_opts).unwrap()
                    } else {
                        b.size.to_string()
                    };
                    writeln!(output, "\t{}: {}", b.name, size).unwrap();
                });
            }

            // registry source
            if !registry_source_caches_matches.is_empty() {
                sort_files_by_size(&mut registry_source_caches_matches);
                output.push_str("\nRegistry source cache sorted by size:\n");
                registry_source_caches_matches.iter().for_each(|b| {
                    let size = if hr_size {
                        b.size.file_size(&humansize_opts).unwrap()
                    } else {
                        b.size.to_string()
                    };
                    writeln!(output, "\t{}: {}", b.name, size).unwrap();
                });
            }
        }

        Some(&_) => {
            unreachable!();
        }
    }

    let trimmed = output.trim();
    if !trimmed.is_empty() {
        println!("{}", trimmed);
    }
    Ok(())
}

#[cfg(test)]
mod query_tests {
    use crate::test_helpers::bin_path;
    use std::process::Command;

    #[test]
    fn query_subcmd_long() {
        let query_cmd = Command::new(bin_path()).arg("query").output();
        assert!(
            query_cmd.is_ok(),
            "cargo-cache query failed: '{:?}'",
            query_cmd
        );
    }

    #[test]
    fn query_subcmd_short() {
        let query_cmd = Command::new(bin_path()).arg("q").output();
        assert!(
            query_cmd.is_ok(),
            "cargo-cache query failed: '{:?}'",
            query_cmd
        );
    }

    #[test]
    fn query_subcmd_hyphen_long() {
        let query_cmd = Command::new(bin_path()).arg("cache-query").output();
        assert!(
            query_cmd.is_ok(),
            "cargo-cache query failed: '{:?}'",
            query_cmd
        );
    }

    #[test]
    fn query_subcmd_hyphen_short() {
        let query_cmd = Command::new(bin_path()).arg("cache-q").output();
        assert!(
            query_cmd.is_ok(),
            "cargo-cache query failed: '{:?}'",
            query_cmd
        );
    }
}
