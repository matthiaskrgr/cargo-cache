// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/// This file provides the `TableLine` struct which is used by
/// `format_2_row_table()` to create neat-looking 2-column tables.

/// struct used to format 2-column tables
#[derive(Clone, Debug)]
pub(crate) struct TableLine {
    /// the padding before `left_column`, mostly used for semantic indentation
    indent_front: usize,
    /// left column
    left_column: String,
    /// right column
    right_column: String,
}

impl TableLine {
    /// creates a new `TableLine` struct
    /// if the right column ends with " B", we pad it to "  B" to align with " MB", " GB" etc
    pub(crate) fn new<LC: ToString, RC: ToString>(
        indent_front: usize,
        left_column: &LC,
        right_column: &RC,
    ) -> Self {
        let mut right_column = right_column.to_string();
        if right_column.ends_with(" B") {
            right_column = right_column.replace(" B", "  B"); // align with "x xB"
        }

        Self {
            indent_front,
            left_column: left_column.to_string(),
            right_column,
        }
    }
}

/// creates a formatted 2 row table (String) from a `Vec` of `TableLines`
pub(crate) fn two_row_table(
    // minimal padding between left and right column
    min_padding_middle: usize,
    // List of TableLine lines to format
    lines: Vec<TableLine>,
    // whether the first line is to be aligned or not
    align_first_line: bool,
) -> String {
    let mut first_line: Option<String> = None;
    #[allow(clippy::shadow_same)]
    let mut lines = lines;
    if !align_first_line && !lines.is_empty() {
        // save the first line and remove it from the vec
        // the first line is special
        // Cargo cache '/home/matthias/.cargo':
        // and must not mess up the alignment
        first_line = Some(lines.remove(0).left_column);
    }

    let total_entries = lines.len();

    // get the length of the longest elements
    let max_len_left_col: usize = if align_first_line {
        lines
            .iter()
            .map(|line| line.left_column.len())
            .max()
            .unwrap_or(0)
    } else {
        lines
            .iter()
            .skip(1)
            .map(|line| line.left_column.len())
            .max()
            .unwrap_or(0)
    };
    let max_len_right_col: usize = lines
        .iter()
        .map(|line| line.right_column.len())
        .max()
        .unwrap_or(0);
    let max_indent_front: usize = lines
        .iter()
        .map(|line| line.indent_front)
        .max()
        .unwrap_or(0);

    let max_indent_front_chars: usize = max_indent_front * 2;
    // ↓padding
    //  103 installed binaries:             1.06 GB
    //   ↑left_col              ↑min_pad_mid    ↑right_col
    let line_length: usize =
        max_len_left_col + max_len_right_col + min_padding_middle + max_indent_front_chars;

    let mut table = String::with_capacity({
        // try to guess how big the final table will be
        line_length * total_entries
    });

    match first_line {
        None => {}
        Some(line) => table.push_str(&line),
    }

    for line in lines {
        // left padding at the beginning of the line
        let indent_front_len = line.indent_front * 2;
        table.push_str(&" ".repeat(indent_front_len));
        // the right column
        table.push_str(&line.left_column);
        //  max len -(padding + left_column + right_column )   == the amount of spaces needed here
        let spaces = line_length
            - (indent_front_len
                + line.left_column.len()
                + min_padding_middle
                + line.right_column.len());
        table.push_str(&" ".repeat(min_padding_middle + spaces));
        table.push_str(&line.right_column);
        table.push('\n');
    }

    table
}

/*
structures the table as follows:
 vec![
    vec![line1_word1, line1_word2, line1_word3],
    vec![line2_word1, line2_word2, line2_word3],
    vec![line3_word1, line3_word2, line3_word3]
]

*/
pub(crate) fn format_table(table: &[Vec<String>], padding: usize) -> String {
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
                cell_new.push_str(&" ".repeat(padding));
                new_row.push_str(&cell_new);
            } else {
                // just add the new cell
                let mut cell = cell.clone();
                cell.push_str(&" ".repeat(padding));
                new_row.push_str(&cell);
            }
            // add space between each cell
            new_row.push_str(SEPARATOR);
        }
        let mut row2 = new_row.trim().to_string();
        row2.push('\n');
        out.push_str(&row2);
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
        let t = format_table(&v, 0);
        let output = String::from("\n");
        assert_eq!(t, output);
    }

    #[test]
    fn one_cell() {
        let v = vec![vec!["hello".into()]];
        let t = format_table(&v, 0);
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
        let t = format_table(&v, 0);
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
        let t = format_table(&v, 0);
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
                String::new(),
                String::from("I don't get it"),
            ],
        ];
        let t = format_table(&v, 0);
        let output = String::from(
            "wasdwasdwasd word word\noh           why  this\nAAAAAA            I don\'t get it\n",
        );
        // println!("{}", output);

        /*
        wasdwasdwasd word word
        oh           why  this
        AAAAAA            I don't get it
         */
        assert_eq!(t, output);
    }
}
