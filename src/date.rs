use crate::cache::*;
use chrono::{prelude::*, FixedOffset, NaiveDateTime, TimeZone};
use regex::Regex;


pub(crate) fn dates(reg_cache: &mut registry_sources::RegistrySourceCaches) {
    let files = reg_cache.total_checkout_folders();

    for file in files {
        let _m = file.metadata();
    }

    let mut dates = files
        .iter()
        .map(|f| f.metadata().unwrap().accessed().unwrap())
        .map(|d: std::time::SystemTime| chrono::DateTime::<Local>::from(d))
        .map(|d| d.naive_local())
        .collect::<Vec<_>>();

    dates.sort();

    // get the current date
    let date = Local::now();

    let _current_date = date.format("%Y.%M.%D"); // get the current date
    let _current_time = date.format("%H:%M:%S"); // current time
    let user_input = "2019.05.05"; //"20:33:02"; // 2019.10.01

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
        .filter(|file_date| file_date > &&date_to_compare)
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
