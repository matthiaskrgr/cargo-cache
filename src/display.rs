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

pub(crate) fn format_table(
    indent_lvl_front: &[usize],
    min_padding_middle: usize,
    first_column: &[&str],
    second_column: &[&str],
) -> String {
    assert_eq!(
        first_column.len(),
        second_column.len(),
        "pad_strings: tried to format columns of different lengths!\nfirst: '{:?}'\nsecond: '{:?}'",
        first_column,
        second_column
    );
    // get the length of the longest elements
    let max_len_first_col: usize = first_column.iter().map(|s| s.len()).max().unwrap_or(0);
    let max_len_second_col: usize = second_column.iter().map(|s| s.len()).max().unwrap_or(0);

    let mut table = String::with_capacity(
        first_column.len() * (max_len_first_col + max_len_second_col + min_padding_middle),
    );

    // zip all three together
    let l_r = indent_lvl_front
        .iter()
        .zip(first_column.iter().zip(second_column.iter()));

    for (indent_lvl, tmp) in l_r {
        // destruct the zipping
        let (left_word, right_word) = tmp;
        let mut left: String = left_word.to_string();
        left.push_str(&" ".repeat(max_len_first_col - left_word.len()));
        let mut right: String = (&" ".repeat(max_len_second_col - right_word.len())).to_string();
        right.push_str(&right_word);

        // apply universal padding
        table.push_str(&" ".repeat(2 * *indent_lvl));

        table.push_str(&left);
        table.push_str(&" ".repeat(min_padding_middle));
        table.push_str(&right);
        table.push_str("\n")
    }

    table
}
