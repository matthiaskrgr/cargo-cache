// these [allow()] by default, make them warn:
#![warn(
    ellipsis_inclusive_range_patterns,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unsafe_code,
    unused,
    rust_2018_compatibility,
    rust_2018_idioms
)]
// enable additional clippy warnings
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        clippy,
        clippy_correctness,
        clippy_perf,
        clippy_complexity,
        clippy_style,
        clippy_pedantic,
        clippy_nursery,
        shadow_reuse,
        shadow_same,
        shadow_unrelated,
        pub_enum_variant_names,
        string_add,
        string_add_assign,
        needless_borrow
    )
)]
#![feature(test)]

mod library;

use std::{fs, process};

use clap::value_t;
use humansize::{file_size_opts, FileSize};

use crate::library::*;
fn main() {}
