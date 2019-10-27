use crate::cache::*;
use crate::library::Error;
use chrono::{prelude::*, NaiveDateTime};
use regex::Regex;

fn parse_date(date: &str) -> Result<NaiveDateTime, Error> {
    let date_to_compare: NaiveDateTime = {
        // we only have a date but no time
        if Regex::new(r"^\d{4}.\d{2}.\d{2}$").unwrap().is_match(date) {
            // most likely a date
            println!("date is ymd");
            dbg!(date);
            let now = Local::now();
            let split = date
                .split('.')
                .map(|d| {
                    d.parse::<u32>()
                        .expect(&format!("'{}' seems to not be an u32", d))
                }) // else parse error
                .collect::<Vec<u32>>();
            NaiveDate::from_ymd_opt(split[0] as i32, split[1], split[2])
                .expect(&format!(
                    "Failed to parse  {}.{}.{} as date",
                    split[0], split[1], split[2]
                ))
                .and_hms(now.hour(), now.minute(), now.second())
        } else if Regex::new(r"^\d{2}:\d{2}:\d{2}$").unwrap().is_match(date) {
            // probably a time
            println!("date is hms");
            dbg!(date);

            let today = Local::today();
            let split = date
                .split(':')
                .map(|d| {
                    d.parse::<u32>()
                        .expect(&format!("'{}' seems to not be an u32", d))
                })
                .collect::<Vec<u32>>();

            NaiveDate::from_ymd_opt(today.year(), today.month(), today.day())
                .expect(&format!(
                    "Failed to parse  {}:{}:{} as time",
                    split[0], split[1], split[2]
                ))
                .and_hms(split[0], split[1], split[2])
        } else {
            println!("could not parse date");
            return Err(Error::DateParseFailure("a".into(), "b".into())); // @TODO
        }
    };
    Ok(date_to_compare)
}

// need to get (part of the?) clap config
pub(crate) fn dates(
    reg_cache: &mut registry_sources::RegistrySourceCaches,
    arg_younger: &Option<&str>,
    arg_older: &Option<&str>,
) {
    //  dbg!(arg_younger);
    //   dbg!(arg_older);
    #[derive(Debug, Clone)]
    struct FileWithDate {
        file: std::path::PathBuf,
        access_date: NaiveDateTime,
    }

    let files = reg_cache.total_checkout_folders();

    let mut dates: Vec<FileWithDate> = files
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

    let filtered_files: Vec<&FileWithDate> = match (arg_younger, arg_older) {
        (None, None) => {
            eprintln!("ERROR: no dates were supplied altough -o -y were passed!");
            vec![]
        }
        (Some(younger_date), None) => {
            let younger_than = parse_date(younger_date).unwrap(/*@TODO*/);
            dates
                .iter()
                .filter(|file| file.access_date < younger_than)
                .collect()
        }
        (None, Some(older_date)) => {
            let older_than = parse_date(older_date).unwrap(/*@TODO*/);
            //   dbg!(older_than);
            dates
                .iter()
                .filter(|file| file.access_date > older_than)
                .collect()
        }
        (Some(younger_date), Some(older_date)) => {
            let younger_than = parse_date(younger_date).unwrap(/*@TODO*/);
            let older_than = parse_date(older_date).unwrap(/*@TODO*/);

            dates
                .iter()
                .filter(|file| file.access_date < older_than || file.access_date > younger_than)
                .collect()
        }
    };

    let names = filtered_files.iter().map(|f| &f.file).collect::<Vec<_>>();
    names.iter().for_each(|n| println!("{}", n.display()));
    /* println!(
        "{:?}",
        filtered_files.iter().map(|f| &f.file).collect::<Vec<_>>()
    );*/
}
