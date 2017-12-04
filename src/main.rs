extern crate walkdir;

use std::fs;
use walkdir::WalkDir;


fn main() {

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
