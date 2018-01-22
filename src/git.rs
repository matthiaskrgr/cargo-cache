extern crate git2;
extern crate humansize;

use std::fs;
use std::io::{stdout, Write};
use std::path::PathBuf;
use std::process::Command;

use humansize::{file_size_opts as options, FileSize};

use lib::*;

fn gc_repo(path: &PathBuf, dry_run: bool) -> Result<(u64, u64), (ErrorKind, String)> {
    let repo_name = match path.iter().last() {
        Some(name) => name.to_os_string().into_string().unwrap(),
        None => String::from("<unknown>"),
    };

    print!("Recompressing '{}': ", repo_name);
    if !path.is_dir() {
        return Err((ErrorKind::GitRepoDirNotFound, str_from_pb(path)));
    }

    // get size before
    let repo_size_before = cumulative_dir_size(path).dir_size;
    let sb_human_readable = repo_size_before.file_size(options::DECIMAL).unwrap();
    print!("{} => ", sb_human_readable);

    // we need to flush stdout manually for incremental print();
    let _ = stdout().flush();  // ignore errors

    if dry_run {
        println!("{} ({}{})", sb_human_readable, "+", 0);
        Ok((0, 0))
    } else {
        let repo = match git2::Repository::open(&path) {
            Ok(repo) => repo,
            Err(e) => return Err(((ErrorKind::GitRepoNotOpened), format!("{:?}", e))),
        };

        // delete all history of all checkouts and so on.
        // this will enable us to remove *all* dangling commits
        match Command::new("git")
            .arg("reflog")
            .arg("expire")
            .arg("--expire=1.minute")
            .arg("--all")
            .current_dir(repo.path())
            .output()
        {
            Ok(_) => {}
            Err(e) => return Err((ErrorKind::GitReflogFailed, format!("{:?}", e))),
        }
        // pack refs of branches/tags etc
        match Command::new("git")
            .arg("pack-refs")
            .arg("--all")
            .arg("--prune")
            .current_dir(repo.path())
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
            .current_dir(repo.path())
            .output()
        {
            Ok(_) => {}
            /* println!("git gc error\nstatus: {}", out.status);
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

pub fn run_gc(cargo_cache: &CargoCacheDirs, dry_run: bool) {
    let git_db = &cargo_cache.git_db;
    // gc cloned git repos of crates or whatever
    if !git_db.is_dir() {
        println!("WARNING:   {} is not a dir", git_db.display());
        return;
    }
    let mut total_size_before: u64 = 0;
    let mut total_size_after: u64 = 0;

    println!("\nRecompressing repositories. Please be patient...");
    // gc git repos of crates
    for entry in fs::read_dir(&git_db).unwrap() {
        let repo = entry.unwrap().path();
        let repostr = str_from_pb(&repo);
        let (before, after) = match gc_repo(&repo, dry_run) {
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
        total_size_before += before;
        total_size_after += after;
    }
    println!("Recompressing registries....");
    let mut repo_index = (&cargo_cache.registry_cache).clone();
    // cd "../index"
    repo_index.pop();
    repo_index.push("index/");
    for repo in fs::read_dir(repo_index).unwrap() {
        let repopath = repo.unwrap().path();
        let (before, after) = match gc_repo(&repopath, dry_run) {
            // run gc
            Ok((before, after)) => (before, after),
            Err((errorkind, msg)) => match errorkind {
                ErrorKind::GitGCFailed => {
                    println!("Warning, git gc failed, skipping '{}'", repopath.display());
                    println!("git error: '{}'", msg);
                    continue;
                }
                ErrorKind::GitRepoDirNotFound => {
                    println!("Git repo not found: {}", msg);
                    continue;
                }
                ErrorKind::GitRepoNotOpened => {
                    println!("Failed to parse git repo: '{}'", msg);
                    continue;
                }
                _ => unreachable!(),
            },
        };

        total_size_before += before;
        total_size_after += after;
    } // iterate over registries and gc

    println!(
        "Compressed {} to {}",
        total_size_before.file_size(options::DECIMAL).unwrap(),
        size_diff_format(total_size_before, total_size_after, false)
    );
}
