extern crate humansize;
extern crate walkdir;
extern crate clap;
extern crate cargo;

use std::fs;
use std::path::Path;

use clap::App;
use humansize::{FileSize, file_size_opts as options};
use walkdir::WalkDir;
//use cargo::util::config;

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

    let cargo_cfg = cargo::util::config::Config::default().unwrap();
    let cargo_home_str = format!("{}", cargo_cfg.home().display());
    let cargo_home_path = Path::new(&cargo_home_str);


    // make sure we actually have a cargo dir
    if !cargo_home_path.is_dir() {
        println!("Error, no '{} dir found", &cargo_home_str);
        std::process::exit(1);
    }
    println!("cargo home: {}", cargo_home_str);
    let cumulative_size_cargo = cumulative_dir_size(&cargo_home_str);


    let bin_dir = (cargo_home_path.clone()).join("bin/");
    let bin_dir_str = bin_dir.clone().into_os_string().into_string().unwrap();
    println!("bin dir: {}", bin_dir_str);
    let mut cumulative_bin_size = 0;
    let mut number_of_bins = 0;
    if bin_dir.is_dir() {
        cumulative_bin_size = cumulative_dir_size(&bin_dir_str);
        number_of_bins = get_file_number(&bin_dir_str);
    }


    let registry_dir = (cargo_home_path.clone()).join("registry/");
    let registry_dir_str = (registry_dir.clone())
        .into_os_string()
        .into_string()
        .unwrap();
    println!("registry dir: {}", registry_dir_str);
    let mut cumulative_registry_size = 0;
    if registry_dir.is_dir() {
        cumulative_registry_size = cumulative_dir_size(&registry_dir_str);
    }


    let git_db = (cargo_home_path.clone()).join("git/db/");
    let git_db_str = git_db.clone().into_os_string().into_string().unwrap();
    println!("git db dir: {}", git_db_str);
    let mut git_db_size = 0;
    if git_db.is_dir() {
        git_db_size = cumulative_dir_size(&git_db_str);
    }

    let git_checkouts = (cargo_home_path.clone()).join("git/checkouts/");
    let git_checkouts_size_str = (git_checkouts.clone())
        .into_os_string()
        .into_string()
        .unwrap();
    println!("checkouts dir: {}", git_checkouts_size_str);
    let mut git_checkouts_size = 0;
    if git_checkouts.is_dir() {
        git_checkouts_size = cumulative_dir_size(&git_checkouts_size_str);
    }



    println!("\nCargo cache:\n");
    println!(
        "Total size: {} ",
        cumulative_size_cargo.file_size(options::DECIMAL).unwrap()
    );
    println!(
        "Size of {} installed binaries {} ",
        number_of_bins,
        cumulative_bin_size.file_size(options::DECIMAL).unwrap()
    );
    println!(
        "Size of registry {} ",
        cumulative_registry_size
            .file_size(options::DECIMAL)
            .unwrap()
    );
    println!(
        "Size of git db  {} ",
        git_db_size.file_size(options::DECIMAL).unwrap()
    );
    println!(
        "Size of git repo checkouts {} ",
        git_checkouts_size.file_size(options::DECIMAL).unwrap()
    );

}
