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

    print!("Recompressing '{}': ", repo_name);
    // if something went wrong and this is not actually a directory, return an error
    if !path.is_dir() {
        return Err((ErrorKind::GitRepoDirNotFound, format!("{}", path.display())));
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
        match Command::new("git")
            .arg("reflog")
            .arg("expire")
            .arg("--expire=1.minute")
            .arg("--all")
            .current_dir(repo_path)
            .output()
        {
            Ok(_) => {}
            Err(e) => return Err((ErrorKind::GitReflogFailed, format!("{:?}", e))),
        }
        // pack refs of branches/tags etc into one file
        match Command::new("git")
            .arg("pack-refs")
            .arg("--all")
            .arg("--prune")
            .current_dir(repo_path)
            .output()
        {
            Ok(_) => {}
            Err(e) => return Err((ErrorKind::GitPackRefsFailed, format!("{:?}", e))),
        }

        // recompress the repo from scratch and ignore all dangling objects
        match Command::new("git")
            .arg("gc")
            .arg("--aggressive")
            .arg("--prune=now")
            .current_dir(repo_path)
            .output()
        {
            Ok(_) => {}
            /* debug:
            println!("git gc error\nstatus: {}", out.status);
            println!("stdout:\n {}", String::from_utf8_lossy(&out.stdout));
            println!("stderr:\n {}", String::from_utf8_lossy(&out.stderr));
            //if out.status.success() {}
            } */
            Err(e) => return Err((ErrorKind::GitGCFailed, format!("{:?}", e))),
        }
        let repo_size_after = cumulative_dir_size(path).dir_size;
        println!(
            "{}",
            size_diff_format(repo_size_before, repo_size_after, false)
        );

        Ok((repo_size_before, repo_size_after))
    }
}

#[cfg_attr(feature = "cargo-clippy", allow(stutter))]
pub(crate) fn git_gc_everything(git_db_dir: &PathBuf, registry_cache_dir: &PathBuf, dry_run: bool) {
    // gc repos and registries inside cargo cache

    fn gc_subdirs(path: &PathBuf, dry_run: bool) -> (u64, u64) {
        // takes directory, finds all subdirectories and tries to gc those
        let mut size_sum_before: u64 = 0;
        let mut size_sum_after: u64 = 0;

        for entry in fs::read_dir(&path).unwrap() {
            let repo = entry.unwrap().path();
            let repostr = format!("{}", repo.display());
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
    } // fn

    // gc cloned git repos of crates or whatever
    if !git_db_dir.is_dir() {
        println!("WARNING:   {} is not a directory", git_db_dir.display());
        return;
    }
    let mut total_size_before: u64 = 0;
    let mut total_size_after: u64 = 0;

    println!("\nRecompressing repositories. Please be patient...");
    // gc git repos of crates
    let (repos_before, repos_after) = gc_subdirs(git_db_dir, dry_run);
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
