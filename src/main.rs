extern crate humansize;
extern crate walkdir;

use std::fs;
use std::path::Path;
use humansize::{FileSize, file_size_opts as options};
use walkdir::WalkDir;

fn main() {
    let cargo_dir = "/home/matthias/.cargo/";

    // make sure we actually have a cargo dir
    if !Path::new(cargo_dir).is_dir() {
        println!("Error, no '~/.cargo/' dir found");
        std::process::exit(1);
    }

    let mut cumulative_size = 0;
    // traverse recursively and sum filesizes
    for entry in WalkDir::new(cargo_dir) {
        let entry = entry.unwrap();
        let path = entry.path();
        //println!("{}", path.display());

        if path.is_file() {
            cumulative_size += fs::metadata(path).unwrap().len();
        }

    }


    println!("Cargo cache:\n\n");
    println!("Total size: {} b", cumulative_size);
    println!(
        "Total size: {} ",
        cumulative_size.file_size(options::DECIMAL).unwrap()
    );



}
