//extern crate walkdir;

//use std::env;
//use std::io;
//use walkdir::WalkDir;
//use std::path::Path;
//use std::fs::{self, DirEntry};
use std::fs;



// let metadata = fs::metadata("foo.txt")?;
// let size =  metadata.len()
fn main() {
    let mut paths_to_check = Vec::new();
    let mut files_to_check = Vec::new();

    let cargo_dir = "/home/matthias/.cargo/";
    //let cargo_dir = ".";

    for entry in fs::read_dir(cargo_dir).unwrap() {
        //println!("{:?}", entry);
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            paths_to_check.push(path);
        } else if path.is_file() {
            files_to_check.push(path);
        }

    } // for

    println!{"paths to check {:?}", paths_to_check};
    println!{"files to check {:?}", files_to_check};


}




// one possible implementation of walking a directory only visiting files
