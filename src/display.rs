// Copyright 2017-2019 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

pub(crate) struct TableLine {
    indent_front: usize,
    left_column: String,
    right_column: String,
}

impl TableLine {
    pub(crate) fn new(indent_front: usize, left_column: String, right_column: String) -> Self {
        let mut right_column = right_column;
        if right_column.ends_with(" B") {
            right_column = right_column.replace(" B", "  B"); // align with "x xB"
        }

        Self {
            indent_front,
            left_column,
            right_column,
        }
    }
}

pub(crate) fn format_2_row_table(min_padding_middle: usize, lines: &[TableLine]) -> String {
    let total_entries = lines.len();

    // get the length of the longest elements
    let max_len_left_col: usize = lines
        .iter()
        .map(|line| line.left_column.len())
        .max()
        .unwrap_or(0);
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
        table.push_str("\n");
    }

    table
}
