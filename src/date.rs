// Copyright 2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::cache::caches::{Cache, RegistrySuperCache};
use crate::cache::*;
use crate::library::*;
use crate::remove::*;

use chrono::{prelude::*, NaiveDateTime};
use regex::Regex;

// remove cache items that are older than X or younger than Y (or between X and Y)

//  testing:
// ./target/debug/cargo-cache --dry-run  --remove-dir=git-db  --remove-if-younger-than 08:08:08

// check how to query files
#[derive(Debug, Clone)]
enum AgeRelation<'a> {
    None,
    FileOlderThanDate(&'a str),
    FileYoungerThanDate(&'a str),
    // OlderOrYounger(&'a str, &'a str),
}

fn parse_date(date: &str) -> Result<NaiveDateTime, Error> {
    // @TODO handle yyyyy.mm.dd hh:mm:ss
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
                if let Some(date2) = NaiveDate::from_ymd_opt(split[0] as i32, split[1], split[2]) {
                    date2
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

            let nd = if let Some(date2) =
                NaiveDate::from_ymd_opt(today.year(), today.month(), today.day())
            {
                date2
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

#[derive(Debug, Clone)]
struct FileWithDate {
    file: std::path::PathBuf,
    access_date: NaiveDateTime,
}

fn filter_files_by_date<'a>(
    date: &AgeRelation<'_>,
    files: &'a [FileWithDate],
) -> Result<Vec<&'a FileWithDate>, Error> {
    match date {
        AgeRelation::None => {
            unreachable!("ERROR: no dates were supplied although -o or -y were passed!");
        }
        AgeRelation::FileYoungerThanDate(younger_date) => {
            // file is younger than date if file.date > date_param
            let date_parameter = parse_date(younger_date)?;
            Ok(files
                .iter()
                .filter(|file| file.access_date > date_parameter)
                .collect())
        }
        AgeRelation::FileOlderThanDate(older_date) => {
            // file is older than date if file.date < date_param
            let date_parameter = parse_date(older_date)?;
            Ok(files
                .iter()
                .filter(|file| file.access_date < date_parameter)
                .collect())
        } /*   DateComparison::OlderOrYounger(older_date, younger_date) => {
              let younger_than = parse_date(younger_date)?;
              let older_than = parse_date(older_date)?;

              Ok(files
                  .iter()
                  // this may be bugged
                  .filter(|file| file.access_date < younger_than || file.access_date > older_than)
                  .collect())
          } */
    }
}

/// removes files that are older than $date from the cache, dirs can be specified
#[allow(clippy::too_many_arguments)]
pub(crate) fn remove_files_by_dates(
    // we need to know which part of the cargo-cache we need to clear out!
    checkouts_cache: &mut git_checkouts::GitCheckoutCache,
    bare_repos_cache: &mut git_bare_repos::GitRepoCache,
    registry_pkg_caches: &mut registry_pkg_cache::RegistryPkgCaches,
    registry_sources_caches: &mut registry_sources::RegistrySourceCaches,
    arg_younger: Option<&str>,
    arg_older: Option<&str>,
    dry_run: bool,
    dirs: Option<&str>,
    size_changed: &mut bool,
) -> Result<(), Error> {
    if dirs.is_none() {
        return Err(Error::RemoveDirNoArg);
    }

    // get the list of components that we want to check
    let components_to_remove_from = components_from_groups(dirs)?;
    // println!("components: {:?}", components_to_remove_from);

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
                files_of_components.extend(checkouts_cache.items().iter().cloned());
            }
            Component::GitDB => {
                files_of_components.extend(bare_repos_cache.items().iter().cloned());
            }
        }
    });

    // try to find out how to compare dates
    let date_comp: AgeRelation<'_> = match (arg_older, arg_younger) {
        (None, None) => AgeRelation::None,
        (None, Some(younger)) => AgeRelation::FileYoungerThanDate(younger),
        (Some(older), None) => AgeRelation::FileOlderThanDate(older),
        (Some(_older), Some(_younger)) => {
            unreachable!(
                "{}",
                "passing both, --remove-if-{older,younger}-than was temporarily disabled!"
            )
        } // (Some(older), Some(younger)) => DateComparison::OlderOrYounger(older, younger),
    };

    // for each file, get the access time
    let mut dates: Vec<FileWithDate> = files_of_components
        .into_iter()
        .map(|path| {
            let access_time = path.metadata().unwrap().accessed().unwrap();
            let naive_datetime = chrono::DateTime::<Local>::from(access_time).naive_local();
            FileWithDate {
                file: path,
                access_date: naive_datetime,
            }
        })
        .collect();

    dates.sort_by_key(|f| f.file.clone());

    // filter the files by comparing the given date and the files access time
    let filtered_files: Vec<&FileWithDate> = filter_files_by_date(&date_comp, &dates)?;

    if dry_run {
        // if we dry run, we won't have to invalidate caches
        println!(
            "dry-run: would delete {} items that are {}...",
            filtered_files.len(),
            match date_comp {
                AgeRelation::FileYoungerThanDate(date) => format!("younger than {}", date),
                AgeRelation::FileOlderThanDate(date) => format!("older than {}", date),
                AgeRelation::None => unreachable!(
                    "DateComparisonOlder and Younger or None not supported right now (dry run)"
                ),
            },
        );
    } else {
        // no dry run / actual run
        println!(
            "Deleting {} items that are {}...",
            filtered_files.len(),
            match date_comp {
                AgeRelation::FileYoungerThanDate(date) => format!("younger than {}", date),
                AgeRelation::FileOlderThanDate(date) => format!("older than {}", date),
                AgeRelation::None => unreachable!(
                    "DateComparisonOlder and Younger or None not supported right now (no dry run)"
                ),
            },
        );
        filtered_files
            .into_iter()
            .map(|fwd| &fwd.file)
            //.inspect(|p| println!("{}", p.display()))
            .for_each(|path| {
                remove_file(
                    path,
                    false,
                    size_changed,
                    None,
                    &DryRunMessage::Default,
                    None,
                );
            });

        // invalidate caches that we removed from
        components_to_remove_from.iter().for_each(|component| {
            match component {
                Component::RegistryCrateCache => {
                    registry_pkg_caches.invalidate();
                }
                Component::RegistrySources => {
                    registry_sources_caches.invalidate();
                }
                Component::RegistryIndex => { /* ignore this case */ }
                Component::GitRepos => {
                    checkouts_cache.invalidate();
                }
                Component::GitDB => {
                    bare_repos_cache.invalidate();
                }
            }
        });
    }
    // summary is printed from inside main()
    Ok(())
}

#[cfg(test)]
mod libtests {
    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn parse_dates() {
        assert!(parse_date("").is_err());
        assert!(parse_date(&String::from("a")).is_err());

        assert!(parse_date(&String::from("01.01:2002")).is_err());
        assert!(parse_date(&String::from("01.01.2002")).is_err()); // need yyyy.mm.dd
        assert!(parse_date(&String::from("2002.30.30")).is_err());

        assert_eq!(
            parse_date(&String::from("2002.01.01"))
                .unwrap()
                .format("%Y.%m.%d")
                .to_string(),
            String::from("2002.01.01")
        );

        assert_eq!(
            parse_date(&String::from("1234.12.08"))
                .unwrap()
                .format("%Y.%m.%d")
                .to_string(),
            String::from("1234.12.08")
        );

        assert_eq!(
            parse_date(&String::from("1990.12.08"))
                .unwrap()
                .format("%Y.%m.%d")
                .to_string(),
            String::from("1990.12.08")
        );

        assert_eq!(
            parse_date(&String::from("12:00:00"))
                .unwrap()
                .format("%H:%M:%S")
                .to_string(),
            String::from("12:00:00")
        );

        assert_eq!(
            parse_date(&String::from("00:00:00"))
                .unwrap()
                .format("%H:%M:%S")
                .to_string(),
            String::from("00:00:00")
        );
    }

    #[test]
    #[should_panic(expected = "invalid time")]
    fn parse_dates_panic1() {
        assert!(parse_date(&String::from("24:00:00")).is_err());
    }

    #[test]
    #[should_panic(expected = "invalid time")]
    fn parse_dates_panic2() {
        assert!(parse_date(&String::from("24:30:24")).is_err());
    }

    #[test]
    #[should_panic(expected = "invalid time")]
    fn parse_dates_panic3() {
        assert!(parse_date(&String::from("30:30:24")).is_err());
    }
}
