// Copyright 2017-2019 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

pub(crate) struct TableLine<'a> {
    indent_front: usize,
    left_column: &'a str,
    right_column: &'a str,
}

pub(crate) fn format_table_2(min_padding_middle: usize, lines: &[&TableLine]) -> String {
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
        table.push_str(line.left_column);
        //  max len -(padding + left_column + right_column )   == the amount of spaces needed here
        let spaces = line_length
            - (indent_front_len
                + line.left_column.len()
                + min_padding_middle
                + line.right_column.len())
            - 1; // -1: the final "\n" we will insert at the end
        table.push_str(&" ".repeat(min_padding_middle + spaces));
        table.push_str(line.right_column);
        table.push_str("\n");
    }

    table
}
