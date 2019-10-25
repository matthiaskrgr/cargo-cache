use crate::cache::*;
use crate::library::Error;
use chrono::{prelude::*, NaiveDateTime};
use regex::Regex;

fn parse_date(date: &str) -> Result<NaiveDateTime, Error> {
    //  dbg!(&date);
    let date_to_compare: NaiveDateTime = {
        // we only have a date but no time
        if Regex::new(r"^\d{4}.\d{2}.\d{2}$").unwrap(/*@FIXME*/).is_match(date) {
            // most likely a date
            println!("date is ymd");
            dbg!(date);
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
            println!("date is hms");
            dbg!(date);

            let today = Local::today();
            let split = date
                .split(':')
                .map(|d| d.parse::<u32>().unwrap()) // else parse error
                .collect::<Vec<u32>>();

            NaiveDate::from_ymd_opt(today.year(), today.month(), today.day())
                .unwrap() // else parse error
                .and_hms(split[0], split[1], split[2])
        } else {
            println!("could not parse date");
            return Err(Error::DateParseError("a".into(), "b".into())); // parse error
        }
    };
    // dbg!(date_to_compare);
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

    // @TODO  if both are supplied, combine them with  OR

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

    // get the current date
    // let now = Local::now();

    //let current_date = now.format("%Y.%M.%D"); // get the current date
    //let current_time = now.format("%H:%M:%S"); // current time

    // dbg!((arg_younger, arg_older));

    let filtered_files: Vec<&FileWithDate> = match (arg_younger, arg_older) {
        (None, None) => {
            // @TODO warn no date
            vec![]
        }
        (Some(younger_date), None) => {
            let younger_than = parse_date(&younger_date).unwrap(/*@TODO*/);
            dates
                .iter()
                .filter(|file| file.access_date < younger_than)
                .collect()
        }
        (None, Some(older_date)) => {
            let older_than = parse_date(&older_date).unwrap(/*@TODO*/);
            //   dbg!(older_than);
            dates
                .iter()
                .filter(|file| file.access_date > older_than)
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

    //  dbg!(&filtered_files);

    // parse user time

    // if the user does not specify a date, (which we need), take the default date of $today
    // and use it

    //let compare_date = NaiveDateTime::parse_from_str("12:09:13", "%H:%M:%S %Y.%M.%D");

    // NaiveDate::from_ymd(2015, 9, 25).and_hms(12, 34, 56);

    // then, filter out all files with date older/younger than x
    //
    // // https://docs.rs/chrono/0.4.9/chrono/naive/struct.NaiveDateTime.html#method.date

    //  println!("{:?}", dates);
    //
    let names = filtered_files.iter().map(|f| &f.file).collect::<Vec<_>>();
    names.iter().for_each(|n| println!("{}", n.display()));
    /* println!(
        "{:?}",
        filtered_files.iter().map(|f| &f.file).collect::<Vec<_>>()
    );*/
}
