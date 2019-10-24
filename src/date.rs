use crate::cache::*;
use crate::library::Error;
use chrono::{prelude::*, NaiveDateTime};
use regex::Regex;

fn parse_date(date: &str) -> Result<NaiveDateTime, Error> {
    let date_to_compare: NaiveDateTime = {
        // we only have a date but no time
        if Regex::new(r"^\d{4}.\d{2}.\d{2}$").unwrap(/*@FIXME*/).is_match(date) {
            // most likely a date
            dbg!("date is ymd");
            let now = Local::now();
            let split = date
                .split('.')
                .map(|d| d.parse::<u32>().unwrap()) // else parse error
                .collect::<Vec<u32>>();
            NaiveDate::from_ymd_opt(split[0] as i32, split[1], split[2])
                .unwrap() // else parse error
                .and_hms(now.hour(), now.minute(), now.second())
        } else if Regex::new(r"^\d{2}:\d{2}:\d{2}$").unwrap(/*@FIXME*/).is_match(date) {
            // probably a time
            dbg!("date is hms");

            let today = Local::today();
            let split = date
                .split(':')
                .map(|d| d.parse::<u32>().unwrap()) // else parse error
                .collect::<Vec<u32>>();

            NaiveDate::from_ymd_opt(today.year(), today.month(), today.day())
                .unwrap() // else parse error
                .and_hms(split[0], split[1], split[2])
        } else {
            return Err(Error::DateParseError("a".into(), "b".into())); // parse error
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
    struct FileWithDate {
        file: std::path::PathBuf,
        access_date: NaiveDateTime,
    }

    // @TODO  if both are supplied, combine them with  OR

    let files = reg_cache.total_checkout_folders();

    let mut dates: Vec<FileWithDate> = files
        .iter()
        .map(|f| {
            let path = f;
            let access_time = f.metadata().unwrap().accessed().unwrap();
            let naive_datetime = chrono::DateTime::<Local>::from(access_time).naive_local();
            FileWithDate {
                file: *path,
                access_date: naive_datetime,
            }
        })
        .collect();

    dates.sort_by_key(|f| f.file);

    // get the current date
    let now = Local::now();

    let current_date = now.format("%Y.%M.%D"); // get the current date
    let current_time = now.format("%H:%M:%S"); // current time

    let filter_closure: Vec<&FileWithDate> = match (arg_younger, arg_older) {
        (None, None) => {
            // @TODO warn no date
            vec![]
        }
        (Some(younger_date), None) => {
            let younger_than = parse_date(&younger_date).unwrap(/*@TODO*/);
            dates
                .iter()
                .filter(|file| file.access_date > younger_than)
                .collect()
        }
        (None, Some(older_date)) => {
            let older_than = parse_date(&older_date).unwrap(/*@TODO*/);
            dates
                .iter()
                .filter(|file| file.access_date < older_than)
                .collect()
        }
        (Some(younger_date), Some(older_date)) => {
            let younger_than = parse_date(&younger_date).unwrap(/*@TODO*/);
            let older_than = parse_date(&older_date).unwrap(/*@TODO*/);

            dates
                .iter()
                .filter(|file| file.access_date < older_than || file.access_date > younger_than)
                .collect()
        }
    };

    //let date_to_compare = parse_date();

    let date_to_compare: NaiveDateTime = {
        // we only havea date but no time
        if Regex::new(r"^\d{4}.\d{2}.\d{2}$").unwrap(/*@FIXME*/).is_match(user_input) {
            // most likely a date
            dbg!("there");
            let now = Local::now();
            let split = user_input
                .split('.')
                .map(|d| d.parse::<u32>().unwrap())
                .collect::<Vec<u32>>();
            NaiveDate::from_ymd_opt(split[0] as i32, split[1], split[2])
                .unwrap()
                .and_hms(now.hour(), now.minute(), now.second())
        } else if Regex::new(r"^\d{2}:\d{2}:\d{2}$").unwrap(/*@FIXME*/).is_match(user_input) {
            // probably a time
            dbg!("here");

            let today = Local::today();
            let split = user_input
                .split(':')
                .map(|d| d.parse::<u32>().unwrap())
                .collect::<Vec<u32>>();

            NaiveDate::from_ymd_opt(today.year(), today.month(), today.day())
                .unwrap()
                .and_hms(split[0], split[1], split[2])
        } else {
            panic!("Failed to parse: '{}'", user_input);
        }
    };

    let filtered = dates
        .iter()
        .filter(|file_date| **file_date > date_to_compare)
        .collect::<Vec<_>>();

    // parse user time

    // if the user does not specify a date, (which we need), take the default date of $today
    // and use it

    //let compare_date = NaiveDateTime::parse_from_str("12:09:13", "%H:%M:%S %Y.%M.%D");

    // NaiveDate::from_ymd(2015, 9, 25).and_hms(12, 34, 56);

    // then, filter out all files with date older/younger than x
    //
    // // https://docs.rs/chrono/0.4.9/chrono/naive/struct.NaiveDateTime.html#method.date

    //  println!("{:?}", dates);
    println!("{:?}", date_to_compare);

    println!("filtered len: {}", filtered.len());
}
