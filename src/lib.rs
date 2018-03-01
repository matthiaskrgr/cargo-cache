
extern crate walkdir;

use std::fs;
use std::path::PathBuf;

use walkdir::WalkDir;



pub fn cumulative_dir_size(dir: &PathBuf) -> u64 {

    let mut files = Vec::new();
    for entry in WalkDir::new(format!("{}", dir.display())) {
        let entry = entry.unwrap();
        let path = entry.path();
        files.push(path.to_owned());
    }
	
	let file2 = WalkDir::new(format!("{}", dir.display()));
    
    
	return 0;
}




/*





warning: unused import: `std::fs`
 --> src/lib.rs:5:5
  |
5 | use std::fs;
  |     ^^^^^^^
  |
  = note: #[warn(unused_imports)] on by default

warning: unused variable: `file2`
  --> src/lib.rs:21:6
   |
21 |     let file2 = WalkDir::new(format!("{}", dir.display()));
   |         ^^^^^ help: consider using `_file2` instead
   |
   = note: #[warn(unused_variables)] on by default

warning: unused import: `std::fs`
 --> src/lib.rs:5:5
  |
5 | use std::fs;
  |     ^^^^^^^
  |
  = note: #[warn(unused_imports)] on by default

warning: unused import: `lib::*`
  --> src/main.rs:19:5
   |
19 | use lib::*;
   |     ^^^^^^

warning: unused variable: `file2`
  --> src/lib.rs:21:6
   |
21 |     let file2 = WalkDir::new(format!("{}", dir.display()));
   |         ^^^^^ help: consider using `_file2` instead
   |
   = note: #[warn(unused_variables)] on by default

warning: function is never used: `cumulative_dir_size`
  --> src/lib.rs:12:1
   |
12 | pub fn cumulative_dir_size(dir: &PathBuf) -> u64 {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: #[warn(dead_code)] on by default

    Finished dev [unoptimized + debuginfo] target(s) in 1.19 secs















*/
