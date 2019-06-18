// Copyright 2017-2019 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


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