// Copyright 2017-2020 Matthias Krüger. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fs;
use std::io::{stdout, Write};
use std::path::Path;
use std::process::Command;

use humansize::{file_size_opts, FileSize};

use crate::library::Error;
use crate::library::*;

fn gc_repo(path: &Path, dry_run: bool) -> Result<(u64, u64), Error> {
    // get name of the repo (last item of path)
    let repo_name = match path.iter().last() {
        Some(name) => name.to_str().unwrap().to_string(),
        None => "<unknown>".to_string(),
    };
    debug_assert_ne!(repo_name, "<unknown>", "unknown repo name: '{:?}'", &path);

    print!("Recompressing '{}': ", &repo_name);
    // if something went wrong and this is not actually a directory, return an error
    if !path.is_dir() {
        return Err(Error::GitRepoDirNotFound(path.into()));
    }

    // get size before
    let repo_size_before = cumulative_dir_size(path).dir_size;
    let sb_human_readable = repo_size_before.file_size(file_size_opts::DECIMAL).unwrap();
    print!("{} => ", sb_human_readable);

    // we need to flush stdout manually for incremental print();
    // ignore errors
    let _ignore = stdout().flush();

    if dry_run {
        // don't do anything on dry run
        println!("{} (+0)", sb_human_readable);
        Ok((0, 0))
    } else {
        // validate that the directory is a git repo
        let repo = match git2::Repository::open(path) {
            Ok(repo) => repo,
            Err(_e) => return Err(Error::GitRepoNotOpened(path.into())),
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
            return Err(Error::GitReflogFailed(path.into(), e));
        }

        // pack refs of branches/tags etc into one file
        if let Err(e) = Command::new("git")
            .arg("pack-refs")
            .arg("--all")
            .arg("--prune")
            .current_dir(repo_path)
            .output()
        {
            return Err(Error::GitPackRefsFailed(path.into(), e));
        }

        // git gc the repo get rid of unneeded objects
        if let Err(e) = Command::new("git")
            .arg("gc")
            .arg("--prune=now")
            .current_dir(repo_path)
            .output()
        {
            return Err(Error::GitGCFailed(path.into(), e));
        }

        // git repacḱ the repo get rid of unneeded objects
        if let Err(e) = Command::new("git")
            .arg("repack")
            .arg("-a")
            .arg("-d")
            .arg("-f")
            .arg("--depth=250")
            .arg("--window=250")
            // create packs with at most 1G of size.
            // this should be enough for most projects and can reduce memory problems when recompressing repos
            .arg("--max-pack-size=1G")
            .arg("--unpack-unreachable=now")
            .current_dir(repo_path)
            .output()
        {
            return Err(Error::GitRepackFailed(path.into(), e));
        }

        let repo_size_after = cumulative_dir_size(path).dir_size;
        println!(
            "{}",
            size_diff_format(repo_size_before, repo_size_after, false)
        );

        Ok((repo_size_before, repo_size_after))
    }
}

#[allow(clippy::module_name_repetitions)]
pub(crate) fn git_gc_everything(
    git_repos_bare_dir: &Path,
    registry_pkg_cache_dir: &Path,
    dry_run: bool,
) -> Result<(), Error> {
    // gc repos and registries inside cargo cache

    fn gc_subdirs(path: &Path, dry_run: bool) -> Result<(u64, u64), Error> {
        if path.is_file() {
            return Err(Error::GitGCFile(path.to_path_buf()));
        } else if !path.is_dir() {
            // if the directory does not exist, skip it
            return Ok((0, 0));
        }
        // takes directory, finds all subdirectories and tries to gc those
        let mut size_sum_before: u64 = 0;
        let mut size_sum_after: u64 = 0;

        let mut git_repos: Vec<_> = fs::read_dir(path)
            .unwrap()
            .map(|x| x.unwrap().path())
            .collect();
        // sort git repos in alphabetical order
        git_repos.sort();

        for repo in git_repos {
            // compress
            let (size_before, size_after) = match gc_repo(&repo, dry_run) {
                // run gc
                Ok((before, after)) => (before, after),
                Err(error) => match error {
                    // Error::GitNotInstalled  should be handled before this function is called
                    Error::GitGCFailed(_, _)
                    | Error::GitRepoDirNotFound(_)
                    | Error::GitRepoNotOpened(_) => {
                        eprintln!("{}", error);
                        continue;
                    }

                    _ => unreachable!(),
                },
            };
            size_sum_before += size_before;
            size_sum_after += size_after;
        }
        Ok((size_sum_before, size_sum_after))
    } // fn gc_subdirs

    // make sure git is actually installed (#94), throw clean error if it's not
    if Command::new("git").arg("help").output().is_err() {
        return Err(Error::GitNotInstalled);
    }

    // gc cloned git repos of crates and registries
    let mut total_size_before: u64 = 0;
    let mut total_size_after: u64 = 0;

    println!("\nRecompressing repositories. This may take some time...");
    // gc git repos of crates
    let (repos_before, repos_after) = gc_subdirs(git_repos_bare_dir, dry_run)?;
    total_size_before += repos_before;
    total_size_after += repos_after;

    println!("\nRecompressing registries. This may take some time...");
    let mut repo_index = registry_pkg_cache_dir.to_path_buf();
    // cd "../index"
    let _ = repo_index.pop();
    repo_index.push("index");
    // gc registries
    let (regs_before, regs_after) = gc_subdirs(&repo_index, dry_run)?;
    total_size_before += regs_before;
    total_size_after += regs_after;

    println!(
        "\nCompressed {} to {}",
        total_size_before
            .file_size(file_size_opts::DECIMAL)
            .unwrap(),
        size_diff_format(total_size_before, total_size_after, false)
    );
    Ok(())
}

fn fsck_repo(path: &Path) -> Result<(), Error> {
    // get name of the repo (last item of path)
    let repo_name = match path.iter().last() {
        Some(name) => name.to_str().unwrap().to_string(),
        None => "<unknown>".to_string(),
    };
    debug_assert_ne!(repo_name, "<unknown>", "unknown repo name: '{:?}'", &path);

    println!("Fscking '{}'", &repo_name);

    // if something went wrong and this is not actually a directory, return an error
    if !path.is_dir() {
        return Err(Error::GitRepoDirNotFound(path.into()));
    }

    let repo = match git2::Repository::open(path) {
        Ok(repo) => repo,
        Err(_e) => return Err(Error::GitRepoNotOpened(path.into())),
    };
    let repo_path = repo.path();

    if let Err(e) = Command::new("git")
        .arg("fsck")
        .arg("--no-progress")
        .arg("--strict")
        .current_dir(repo_path)
        .output()
    {
        return Err(Error::GitFsckFailed(path.into(), e));
    }

    Ok(())
}

#[allow(clippy::module_name_repetitions)]
pub(crate) fn git_fsck_everything(
    git_repos_bare_dir: &Path,
    registry_pkg_cache_dir: &Path,
) -> Result<(), Error> {
    // gc repos and registries inside cargo cache

    fn fsck_subdirs(path: &Path) {
        if path.is_file() {
            panic!(
                "fsck_subdirs() tried to fsck file instead of directory: '{}'",
                path.display()
            );
        } else if !path.is_dir() {
            return;
        }

        let mut git_repos: Vec<_> = fs::read_dir(path)
            .unwrap()
            .map(|x| x.unwrap().path())
            .collect();
        // sort git repos in alphabetical order
        git_repos.sort();

        for repo in git_repos {
            // compress
            match fsck_repo(&repo) {
                // run gc
                Ok(_) => {}
                Err(error) => match error {
                    Error::GitFsckFailed(_, _)
                    | Error::GitRepoDirNotFound(_)
                    | Error::GitRepoNotOpened(_) => {
                        eprintln!("{}", error);
                        continue;
                    }

                    _ => unreachable!(),
                },
            };
        }
    } // fn fsck_subdirs

    // make sure git is actually installed (#94), throw clean error if it's not
    if Command::new("git").arg("help").output().is_err() {
        return Err(Error::GitNotInstalled);
    }

    println!("\nFscking repositories. This may take some time...");
    // fsck git repos of crates
    fsck_subdirs(git_repos_bare_dir);

    println!("\nFscking registries. This may take some time...");
    let mut repo_index = registry_pkg_cache_dir.to_path_buf();
    // cd "../index"
    let _ = repo_index.pop();
    repo_index.push("index");
    // fsck registries
    fsck_subdirs(&repo_index);
    Ok(())
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
            .arg("gitrepo_gc")
            .current_dir("target")
            .output();
        assert!(
            git_init.is_ok(),
            "git_init did not succeed: '{:?}'",
            git_init
        );
        // create a file and add some text
        let mut file = File::create("target/gitrepo_gc/testfile.txt").unwrap();
        file.write_all(b"Hello hello hello this is a test \n hello \n hello")
            .unwrap();
        let git_add = Command::new("git")
            .arg("add")
            .arg("testfile.txt")
            .current_dir("target/gitrepo_gc/")
            .output();
        assert!(git_add.is_ok(), "git add did not succeed: '{:?}'", git_add);
        let git_commit = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("commit msg")
            .current_dir("target/gitrepo_gc/")
            .output();
        assert!(
            git_commit.is_ok(),
            "git commit did not succeed: '{:?}'",
            git_commit
        );
        // create another commit
        let mut file2 = File::create("target/gitrepo_gc/testfile.txt").unwrap();
        file2
            .write_all(
                b"Hello hello hello this is a test \n bla bla bla bla bla  \n hello
        \n this is some more text\n
        lorem ipsum",
            )
            .unwrap();
        let git_add2 = Command::new("git")
            .arg("add")
            .arg("testfile.txt")
            .current_dir("target/gitrepo_gc/")
            .output();
        assert!(git_add2.is_ok(), "git add did not succeed: '{:?}'", git_add);
        let git_commit2 = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("another commit msg")
            .current_dir("target/gitrepo_gc/")
            .output();
        assert!(
            git_commit2.is_ok(),
            "git commit did not succeed: '{:?}'",
            git_commit2
        );

        let (dryrun_before, dryrun_after) = match gc_repo(
            &PathBuf::from("target/gitrepo_gc/"),
            true, /* dry run */
        ) {
            Ok((x, y)) => (x, y),
            _ => (0, 0),
        };
        // dryrun should not change sizes!
        assert_eq!(dryrun_before, 0);
        assert_eq!(dryrun_after, 0);

        let (before, after) = match gc_repo(
            &PathBuf::from("target/gitrepo_gc/"),
            false, /* dry run */
        ) {
            Ok((x, y)) => (x, y),
            _ => (0, 0),
        };
        assert!(
            !before > after,
            "git gc is funky: before: {}  after: {}",
            before,
            after
        );
    }

    #[test]
    fn test_fsck_repo() {
        // create a fake git repo in the target dir
        let git_init = Command::new("git")
            .arg("init")
            .arg("gitrepo_fsck")
            .current_dir("target")
            .output();
        assert!(
            git_init.is_ok(),
            "git_init did not succeed: '{:?}'",
            git_init
        );
        // create a file and add some text
        let mut file = File::create("target/gitrepo_fsck/testfile.txt").unwrap();
        file.write_all(b"Hello hello hello this is a test \n hello \n hello")
            .unwrap();
        let git_add = Command::new("git")
            .arg("add")
            .arg("testfile.txt")
            .current_dir("target/gitrepo_fsck/")
            .output();
        assert!(git_add.is_ok(), "git add did not succeed: '{:?}'", git_add);
        let git_commit = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("commit msg")
            .current_dir("target/gitrepo_fsck/")
            .output();
        assert!(
            git_commit.is_ok(),
            "git commit did not succeed: '{:?}'",
            git_commit
        );
        // create another commit
        let mut file2 = File::create("target/gitrepo_fsck/testfile.txt").unwrap();
        file2
            .write_all(
                b"Hello hello hello this is a test \n bla bla bla bla bla  \n hello
        \n this is some more text\n
        lorem ipsum",
            )
            .unwrap();
        let git_add2 = Command::new("git")
            .arg("add")
            .arg("testfile.txt")
            .current_dir("target/gitrepo_fsck/")
            .output();
        assert!(
            git_add2.is_ok(),
            "git add did not succeed: '{:?}'",
            git_add2
        );
        let git_commit2 = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("another commit msg")
            .current_dir("target/gitrepo_fsck/")
            .output();
        assert!(
            git_commit2.is_ok(),
            "git commit did not succeed: '{:?}'",
            git_commit2
        );

        let res = fsck_repo(&PathBuf::from("target/gitrepo_fsck/"));
        assert!(res.is_ok(), "Failed to fsck git repo: {:?}", res);
    }
}
