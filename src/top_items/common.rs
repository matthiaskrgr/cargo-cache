// Copyright 2017-2019 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::path::PathBuf;

pub(crate) fn dir_exists(path: &PathBuf) -> bool {
    // check if a directory exists and print an warning message if not
    if path.exists() {
        true
    } else {
        eprintln!("Skipping '{}' because it doesn't exist.", path.display());
        false
    }
}

pub(crate) fn format_table(table: &[Vec<String>]) -> String {
    const SEPARATOR: &str = " ";
    let mut out = String::new();

    if table.is_empty() {
        return out;
    }

    // find out the largest elements of a column so we know how padding to apply
    // assume all rows have the same length
    let mut max_lengths: Vec<usize> = vec![0; table[0].len()];

    for row in table {
        for (idx, cell) in row.iter().enumerate() {
            // if the cell is bigger than the max, update the max
            if cell.len() > max_lengths[idx] {
                max_lengths[idx] = cell.len();
            }
        }
    }

    // pad the strings
    for row in table {
        let mut new_row = String::new();
        for (idx, cell) in row.iter().enumerate() {
            if cell.len() < max_lengths[idx] {
                // we need to add padding
                let diff = max_lengths[idx] - cell.len();
                let mut cell_new = cell.clone();
                cell_new.push_str(&" ".repeat(diff)); // pad the string
                new_row.push_str(&cell_new);
            } else {
                // just add the new cell
                new_row.push_str(&cell);
            }
            // add space between each cell
            new_row.push_str(SEPARATOR);
        }
        let row = new_row.trim();
        out.push_str(&row);
        out.push_str("\n");
        out.trim();
        // move on to the next cell
    }

    out
}

#[cfg(test)]
mod format_table_tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn empty() {
        let v = vec![Vec::new()];
        let t = format_table(&v);
        let output = String::from("\n");
        assert_eq!(t, output);
    }

    #[test]
    fn one_cell() {
        let v = vec![vec!["hello".into()]];
        let t = format_table(&v);
        let output = String::from("hello\n");
        assert_eq!(t, output);
    }

    #[test]
    fn one_row() {
        let v = vec![vec![
            "hello".into(),
            "a".into(),
            "shrt".into(),
            "very long perhaps a few words".into(),
        ]];
        let t = format_table(&v);
        let output = String::from("hello a shrt very long perhaps a few words\n");
        assert_eq!(t, output);
    }

    #[test]
    fn one_column() {
        let v = vec![
            vec!["hello".into()],
            vec!["a".into()],
            vec!["shrt".into()],
            vec!["very long perhaps a few words".into()],
        ];
        let t = format_table(&v);
        let output = String::from("hello\na\nshrt\nvery long perhaps a few words\n");
        assert_eq!(t, output);
    }

    #[test]
    fn matrix() {
        // cargo test matrix -- --nocapture
        let v = vec![
            vec![
                String::from("wasdwasdwasd"),
                String::from("word"),
                String::from("word"),
            ],
            vec![
                String::from("oh"),
                String::from("why"),
                String::from("this"),
            ],
            vec![
                String::from("AAAAAA"),
                String::from(""),
                String::from("I don't get it"),
            ],
        ];
        let t = format_table(&v);
        let output = String::from(
            "wasdwasdwasd word word\noh           why  this\nAAAAAA            I don\'t get it\n",
        );
        // println!("{}", output);
        assert_eq!(t, output);
    }

}
