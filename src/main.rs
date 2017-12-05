extern crate humansize;
extern crate walkdir;
extern crate clap;

use std::fs;
use std::path::Path;

use clap::App;
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

fn get_file_number(dir: &str) -> u64 {
    let mut number_of_files = 0;
    for entry in WalkDir::new(dir) {
        let entry = entry.unwrap();
        let path = entry.path();
        //println!("{}", path.display());

        if path.is_file() {
            number_of_files += 1;
        }
    } // walkdir
    number_of_files
}

fn main() {

    App::new("cargo-show")
        .version("0.1")
        .about("Manage cargo cache")
        .author("matthiaskrgr")
        .get_matches();

    let cargo_dir = "/home/matthias/.cargo/";

    // make sure we actually have a cargo dir
    if !Path::new(cargo_dir).is_dir() {
        println!("Error, no '{} dir found", cargo_dir);
        std::process::exit(1);
    }
    let cumulative_size_cargo = cumulative_dir_size(&cargo_dir);



    let bin_dir = cargo_dir.to_owned() + "bin/";
    let mut cumulative_bin_size = 0;
    let mut number_of_bins = 0;
    if Path::new(&bin_dir).is_dir() {
        cumulative_bin_size = cumulative_dir_size(&bin_dir);
        number_of_bins = get_file_number(&bin_dir);
    }


    let registry_dir = cargo_dir.to_owned() + "registry/";
    let mut cumulative_registry_size = 0;
    if Path::new(&registry_dir).is_dir() {
        cumulative_registry_size = cumulative_dir_size(&registry_dir);
    }


    let git_db = cargo_dir.to_owned() + "git/db/";
    let mut git_db_size = 0;
    if Path::new(&git_db).is_dir() {
        git_db_size = cumulative_dir_size(&git_db);
    }

    let git_checkouts =  cargo_dir.to_owned() + "git/checkouts/";
    let mut git_checkouts_size = 0;
    if Path::new(&git_checkouts).is_dir() {
        git_checkouts_size = cumulative_dir_size(&git_checkouts);
    }


    println!("Cargo cache:\n\n");
    //println!("Total size: {} b", cumulative_size_cargo);
    println!(
        "Total size: {} ",
        cumulative_size_cargo.file_size(options::DECIMAL).unwrap()
    );
    println!(
        "Total size of {} binaries {} ",
        number_of_bins,
        cumulative_bin_size.file_size(options::DECIMAL).unwrap()
    );
    println!(
        "Total size registry {} ",
        cumulative_registry_size
            .file_size(options::DECIMAL)
            .unwrap()
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
