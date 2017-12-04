extern crate humansize;
extern crate walkdir;

use std::fs;
use std::path::Path;
use humansize::{FileSize, file_size_opts as options};
use walkdir::WalkDir;


fn cumulative_dir_size(dir: &str) -> u64 {
    //@TODO add some clever caching
    let mut cumulative_size = 0;

    // traverse recursively and sum filesizes
    for entry in WalkDir::new(dir) {
        let entry = entry.unwrap();
        let path = entry.path();
        //println!("{}", path.display());

        if path.is_file() {
            cumulative_size += fs::metadata(path).unwrap().len();
        }
    } // walkdir

    cumulative_size
}

fn main() {
    let cargo_dir = "/home/matthias/.cargo/";

    // make sure we actually have a cargo dir
    if !Path::new(cargo_dir).is_dir() {
        println!("Error, no '~/.cargo/' dir found");
        std::process::exit(1);
    }
    let cumulative_size_cargo = cumulative_dir_size(&cargo_dir);



    let bin_dir = "/home/matthias/.cargo/bin/";
    let mut cumulative_bin_size = 0;
    if Path::new(bin_dir).is_dir() {
        cumulative_bin_size = cumulative_dir_size(&bin_dir);
    }


    let registry_dir = "/home/matthias/.cargo/registry/";
    let mut cumulative_registry_size = 0;
    if Path::new(registry_dir).is_dir() {
        cumulative_registry_size = cumulative_dir_size(&registry_dir);
    }


    let git_db = "/home/matthias/.cargo/git/db";
    let mut git_db_size = 0;
    if Path::new(git_db).is_dir() {
        git_db_size= cumulative_dir_size(&git_db);
    }

    let git_checkouts = "/home/matthias/.cargo/git/checkouts";
    let mut git_checkouts_size = 0;
    if Path::new(git_checkouts).is_dir() {
        git_checkouts_size= cumulative_dir_size(&git_checkouts);
    }


    println!("Cargo cache:\n\n");
    println!("Total size: {} b", cumulative_size_cargo);
    println!(
        "Total size: {} ",
        cumulative_size_cargo.file_size(options::DECIMAL).unwrap()
    );
    println!(
        "Total size binaries {} ",
        cumulative_bin_size.file_size(options::DECIMAL).unwrap()
    );
    println!(
        "Total size registry {} ",
        cumulative_registry_size.file_size(options::DECIMAL).unwrap()
    );

    println!(
        "Total git_db  {} ",
        git_db_size.file_size(options::DECIMAL).unwrap()
    );

    println!(
        "Total git repo checkouts {} ",
        git_checkouts_size.file_size(options::DECIMAL).unwrap()
    );

}
