extern crate walkdir;

//use std::env;
//use std::io;
use walkdir::WalkDir;
//use std::path::Path;
//use std::fs::{self, DirEntry};
use std::fs;



// let metadata = fs::metadata("foo.txt")?;
// let size =  metadata.len()
fn main() {


//    let mut paths_to_check = Vec::new();
    //let mut files_to_check = Vec::new();
    let cargo_dir = "/home/matthias/.cargo/";

    let mut cumulative_size = 0;

    for entry in WalkDir::new(cargo_dir) {
        let entry = entry.unwrap();
        let path = entry.path();
        //println!("{}", path.display());

        if path.is_file() {
            cumulative_size += fs::metadata(path).unwrap().len();
        }

        //files_to_check.push(path.clone());

    }
    println!("Total size: {} b", cumulative_size );


}
