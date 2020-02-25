use crate::cache::caches::RegistrySuperCache;
use crate::cache::*;
use crate::library::*;

use chrono::{prelude::*, NaiveDateTime};
use regex::Regex;

// remove cache items that are older than X or younger than Y (or between X and Y)

//  testing:
// ./target/debug/cargo-cache --dry-run  --remove-dir=git-db  --remove-if-younger-than 08:08:08

// check how to query files
#[derive(Debug, Clone)]
enum DateComparison<'a> {
    NoDate,
    Older(&'a str),
    Younger(&'a str),
    OlderOrYounger(&'a str, &'a str),
}

fn parse_date(date: &str) -> Result<NaiveDateTime, Error> {
    // @TODO  handle dd.mm.yy if yy is yy and not yyyy
    let date_to_compare: NaiveDateTime = {
        // xxxx.xx.xx => yyyy.mm.dd
        // we only have a date but no time
        if Regex::new(r"^\d{4}.\d{2}.\d{2}$").unwrap().is_match(date) {
            // most likely a date
            let now = Local::now();
            let split: Result<Vec<u32>, _> = date.split('.').map(str::parse).collect();
            let split = match split {
                Ok(result) => result,
                Err(a) => return Err(Error::DateParseFailure(a.to_string(), "u32".into())),
            };
            #[allow(clippy::cast_possible_wrap)]
            let nd =
                if let Some(date) = NaiveDate::from_ymd_opt(split[0] as i32, split[1], split[2]) {
                    date
                } else {
                    return Err(Error::DateParseFailure(
                        format!("{}.{}.{}", split[0], split[1], split[2]),
                        "date".into(),
                    ));
                };

            nd.and_hms(now.hour(), now.minute(), now.second())

        // xx:xx:xx => hh::mm::ss
        } else if Regex::new(r"^\d{2}:\d{2}:\d{2}$").unwrap().is_match(date) {
            // probably a time
            let today = Local::today();

            let split: Result<Vec<u32>, _> = date.split(':').map(str::parse).collect();
            let split = match split {
                Ok(result) => result,
                Err(a) => return Err(Error::DateParseFailure(a.to_string(), "u32".into())),
            };

            let nd = if let Some(date) =
                NaiveDate::from_ymd_opt(today.year(), today.month(), today.day())
            {
                date
            } else {
                return Err(Error::DateParseFailure(
                    format!("{}:{}:{}", today.year(), today.month(), today.day()),
                    "date".into(),
                ));
            };

            nd.and_hms(split[0], split[1], split[2])
        } else {
            return Err(Error::DateParseFailure(date.into(), "a valid date".into()));
        }
    };
    Ok(date_to_compare)
}

// use the same info as --remove-dir  to pass dirs to be processed

#[derive(Debug, Clone)]
struct FileWithDate {
    file: std::path::PathBuf,
    access_date: NaiveDateTime,
}

fn filter_files_by_date<'a>(
    date: &DateComparison<'_>,
    files: &'a [FileWithDate],
) -> Vec<&'a FileWithDate> {
    match date {
        DateComparison::NoDate => {
            unreachable!("ERROR: no dates were supplied altough -o -y were passed!");
        }
        DateComparison::Younger(younger_date) => {
            let younger_than = parse_date(younger_date).unwrap(/*@TODO*/);
            files
                .iter()
                .filter(|file| file.access_date < younger_than)
                .collect()
        }
        DateComparison::Older(older_date) => {
            let older_than = parse_date(older_date).unwrap(/*@TODO*/);
            files
                .iter()
                .filter(|file| file.access_date > older_than)
                .collect()
        }
        DateComparison::OlderOrYounger(older_date, younger_date) => {
            let younger_than = parse_date(younger_date).unwrap(/*@TODO*/);
            let older_than = parse_date(older_date).unwrap(/*@TODO*/);

            files
                .iter()
                .filter(|file| file.access_date < older_than || file.access_date > younger_than)
                .collect()
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn remove_files_by_dates(
    // we need to know which part of the cargo-cache we need to clear out!
    checkouts_cache: &mut git_checkouts::GitCheckoutCache,
    bare_repos_cache: &mut git_repos_bare::GitRepoCache,
    registry_pkg_caches: &mut registry_pkg_cache::RegistryPkgCaches,
    registry_sources_caches: &mut registry_sources::RegistrySourceCaches,
    arg_younger: &Option<&str>,
    arg_older: &Option<&str>,
    dry_run: bool,
    dirs: &Option<&str>,
) -> Result<(), Error> {
    if dirs.is_none() {
        eprintln!("date: no deletable component supplied!"); //@TODO improve
        std::process::exit(9);
    }

    // get the list of components that we want to check
    let components_to_remove_from = components_from_groups(dirs)?;
    println!("components: {:?}", components_to_remove_from);

    let mut files_of_components: Vec<std::path::PathBuf> = Vec::new();

    components_to_remove_from.iter().for_each(|component| {
        match component {
            Component::RegistryCrateCache => {
                files_of_components.extend(registry_pkg_caches.files());
            }
            Component::RegistrySources => {
                files_of_components.extend(registry_sources_caches.files());
            }
            Component::RegistryIndex => { /* ignore this case */ }
            Component::GitRepos => {
                files_of_components.extend(
                    checkouts_cache
                        .checkout_folders()
                        .iter()
                        .map(|p| p.to_path_buf()),
                );
            }
            Component::GitDB => {
                files_of_components.extend(
                    bare_repos_cache
                        .bare_repo_folders()
                        .iter()
                        .map(|p| p.to_path_buf()),
                );
            }
        }
    });

    // try to find out how to compare dates
    let date_comp: DateComparison<'_> = match (arg_older, arg_younger) {
        (None, None) => DateComparison::NoDate,
        (None, Some(younger)) => DateComparison::Younger(younger),
        (Some(older), None) => DateComparison::Older(older),
        (Some(older), Some(younger)) => DateComparison::OlderOrYounger(older, younger),
    };

    // for each file, get the access time
    let mut dates: Vec<FileWithDate> = files_of_components
        .iter()
        .map(|f| {
            let path = f.clone();
            let access_time = f.metadata().unwrap().accessed().unwrap();
            let naive_datetime = chrono::DateTime::<Local>::from(access_time).naive_local();
            FileWithDate {
                file: path,
                access_date: naive_datetime,
            }
        })
        .collect();

    dates.sort_by_key(|f| f.file.clone());

    // @TODO we can probably do this without collecting first
    // filter the files by comparing the given date and the files access time
    let filtered_files: Vec<&FileWithDate> = filter_files_by_date(&date_comp, &dates);

    // name of the files we are going to delete
    let paths = filtered_files.iter().map(|f| &f.file).collect::<Vec<_>>();

    paths.iter().for_each(|n| println!("{}", n.display()));

    if dry_run {
        println!("Dry-run: would remove {} items", paths.len());
    } else {
        println!("Deleting {} items", paths.len());
    }
    // todo  remove the files
    Ok(())
}

/*
#[cfg(test)]
mod libtests {
    use super::*;
    use chrono::{prelude::*, NaiveDateTime};

    use pretty_assertions::assert_eq;

    #[allow(non_snake_case)]
    #[test]
    fn parse_dates() {}
}
*/
