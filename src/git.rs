// Copyright 2017-2018 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fs;
use std::io::{stdout, Write};
use std::path::PathBuf;
use std::process::Command;

use humansize::{file_size_opts, FileSize};

use crate::library::*;

fn gc_repo(path: &PathBuf, dry_run: bool) -> Result<(u64, u64), (ErrorKind, String)> {
    // get name of the repo (last item of path)
    let repo_name = match path.iter().last() {
        Some(name) => name.to_os_string().into_string().unwrap(),
        None => "<unknown>".to_string(),
    };
    debug_assert_ne!(repo_name, "<unknown>", "unknown repo name: '{:?}'", &path);

    print!("Recompressing '{}': ", &repo_name);
    // if something went wrong and this is not actually a directory, return an error
    if !path.is_dir() {
        return Err((ErrorKind::GitRepoDirNotFound, path.display().to_string()));
    }

    // get size before
    let repo_size_before = cumulative_dir_size(path).dir_size;
    let sb_human_readable = repo_size_before.file_size(file_size_opts::DECIMAL).unwrap();
    print!("{} => ", sb_human_readable);

    // we need to flush stdout manually for incremental print();
    let _ = stdout().flush(); // ignore errors

    if dry_run {
        // don't do anything on dry run
        println!("{} (+0)", sb_human_readable);
        Ok((0, 0))
    } else {
        // validate that the directory is a git repo
        let repo = match git2::Repository::open(&path) {
            Ok(repo) => repo,
            Err(e) => return Err(((ErrorKind::GitRepoNotOpened), format!("{:?}", e))),
        };
        let repo_path = repo.path();
        // delete all history of all checkouts and so on.
        // this will enable us to remove *all* dangling commits
        if let Err(e) = Command::new("git")
            .arg("reflog")
            .arg("expire")
            .arg("--expire=1.minute")
            .arg("--all")
            .current_dir(repo_path)
            .output()
        {
            return Err((ErrorKind::GitReflogFailed, format!("{:?}", e)));
        }

        // pack refs of branches/tags etc into one file
        if let Err(e) = Command::new("git")
            .arg("pack-refs")
            .arg("--all")
            .arg("--prune")
            .current_dir(repo_path)
            .output()
        {
            return Err((ErrorKind::GitPackRefsFailed, format!("{:?}", e)));
        }

        // recompress the repo from scratch and ignore all dangling objects
        if let Err(e) = Command::new("git")
            .arg("gc")
            .arg("--aggressive")
            .arg("--prune=now")
            .current_dir(repo_path)
            .output()
        {
            return Err((ErrorKind::GitGCFailed, format!("{:?}", e)));
        }

        let repo_size_after = cumulative_dir_size(path).dir_size;
        println!(
            "{}",
            size_diff_format(repo_size_before, repo_size_after, false)
        );

        Ok((repo_size_before, repo_size_after))
    }
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::stutter))]
pub(crate) fn git_gc_everything(
    git_repos_bare_dir: &PathBuf,
    registry_cache_dir: &PathBuf,
    dry_run: bool,
) {
    // gc repos and registries inside cargo cache

    fn gc_subdirs(path: &PathBuf, dry_run: bool) -> (u64, u64) {
        if path.is_file() {
            panic!("gc_subdirs() tried to compress file instead of directory")
        } else if !path.is_dir() {
            // if the directory does not exist, skip it
            return (0, 0);
        }
        // takes directory, finds all subdirectories and tries to gc those
        let mut size_sum_before: u64 = 0;
        let mut size_sum_after: u64 = 0;

        for entry in fs::read_dir(&path).unwrap() {
            let repo = entry.unwrap().path();
            let repostr = repo.display();
            // compress
            let (size_before, size_after) = match gc_repo(&repo, dry_run) {
                // run gc
                Ok((before, after)) => (before, after),
                Err((errorkind, msg)) => match errorkind {
                    ErrorKind::GitGCFailed => {
                        println!("Warning, git gc failed, skipping '{}'", repostr);
                        println!("git error: '{}'", msg);
                        continue;
                    }
                    ErrorKind::GitRepoDirNotFound => {
                        println!("Git repo not found: '{}'", msg);
                        continue;
                    }
                    ErrorKind::GitRepoNotOpened => {
                        println!("Failed to parse git repo: '{}'", msg);
                        continue;
                    }
                    _ => unreachable!(),
                },
            };
            size_sum_before += size_before;
            size_sum_after += size_after;
        }
        (size_sum_before, size_sum_after)
    } // fn gc_subdirs

    // gc cloned git repos of crates and registries
    let mut total_size_before: u64 = 0;
    let mut total_size_after: u64 = 0;

    println!("\nRecompressing repositories. Please be patient...");
    // gc git repos of crates
    let (repos_before, repos_after) = gc_subdirs(git_repos_bare_dir, dry_run);
    total_size_before += repos_before;
    total_size_after += repos_after;

    println!("Recompressing registries....");
    let mut repo_index = registry_cache_dir.clone();
    // cd "../index"
    repo_index.pop();
    repo_index.push("index");
    // gc registries
    let (regs_before, regs_after) = gc_subdirs(&repo_index, dry_run);
    total_size_before += regs_before;
    total_size_after += regs_after;

    println!(
        "Compressed {} to {}",
        total_size_before
            .file_size(file_size_opts::DECIMAL)
            .unwrap(),
        size_diff_format(total_size_before, total_size_after, false)
    );
}

#[cfg(test)]
mod gittest {
    use super::*;
    use std::fs::File;
    use std::path::PathBuf;
    use std::process::Command;

    #[test]
    fn test_gc_repo() {
        // create a fake git repo in the target dir
        let git_init = Command::new("git")
            .arg("init")
            .arg("gitrepo")
            .current_dir("target")
            .output();
        assert!(
            git_init.is_ok(),
            "git_init did not succeed: '{:?}'",
            git_init
        );
        // create a file and add some text
        let mut file = File::create("target/gitrepo/testfile.txt").unwrap();
        file.write_all(b"Hello hello hello this is a test \n hello \n hello")
            .unwrap();
        let git_add = Command::new("git")
            .arg("add")
            .arg("testfile.txt")
            .current_dir("target/gitrepo/")
            .output();
        assert!(git_add.is_ok(), "git add did not succeed: '{:?}'", git_add);
        let git_commit = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("commit msg")
            .current_dir("target/gitrepo/")
            .output();
        assert!(
            git_commit.is_ok(),
            "git commit did not succeed: '{:?}'",
            git_commit
        );
        // create another commit
        let mut file = File::create("target/gitrepo/testfile.txt").unwrap();
        file.write_all(
            b"Hello hello hello this is a test \n bla bla bla bla bla  \n hello
        \n this is some more text\n
        lorem ipsum",
        )
        .unwrap();
        let git_add = Command::new("git")
            .arg("add")
            .arg("testfile.txt")
            .current_dir("target/gitrepo/")
            .output();
        assert!(git_add.is_ok(), "git add did not succeed: '{:?}'", git_add);
        let git_commit = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("another commit msg")
            .current_dir("target/gitrepo/")
            .output();
        assert!(
            git_commit.is_ok(),
            "git commit did not succeed: '{:?}'",
            git_commit
        );

        let (dryrun_before, dryrun_after) =
            match gc_repo(&PathBuf::from("target/gitrepo/"), true /* dry run */) {
                Ok((x, y)) => (x, y),
                _ => (0, 0),
            };
        // dryrun should not change sizes!
        assert_eq!(dryrun_before, 0);
        assert_eq!(dryrun_after, 0);

        let (before, after) =
            match gc_repo(&PathBuf::from("target/gitrepo/"), false /* dry run */) {
                Ok((x, y)) => (x, y),
                _ => (0, 0),
            };
        assert!(
            !before > after,
            format!("git gc is funky: before: {}  after: {}", before, after)
        );
    }

}
