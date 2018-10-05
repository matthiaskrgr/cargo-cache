use std::fs;
use std::path::PathBuf;

use humansize::{file_size_opts, FileSize};
use rayon::iter::*;
use walkdir::WalkDir;

use crate::library::*;

#[derive(Debug, Clone)]
struct FileDesc {
    name: String,
    version: String,
    size: u64,
}

impl FileDesc {
    fn new(path: &PathBuf, recursive: bool, checkouts: bool) -> Self {
        let last_item = path.to_str().unwrap().split('/').last().unwrap();

        let mut i = last_item.split('-').collect::<Vec<_>>();
        let name;
        let version;
        if checkouts {
            let mut paths = path.to_str().unwrap().split('/').collect::<Vec<&str>>();
            let last = paths.pop().unwrap();
            let last_but_one = paths.pop().unwrap();
            let last_but_2 = paths.pop().unwrap();

            i = vec![last_but_2, last_but_one, last];

            let string = last_but_one
                .split('/')
                .collect::<Vec<_>>()
                .pop()
                .unwrap()
                .to_string();
            let mut vec = string.split('-').collect::<Vec<_>>();
            let _ = vec.pop();
            name = vec.join("-").to_string();
            version = i.pop().unwrap().trim_right_matches(".crate").to_string();
        } else {
            version = i.pop().unwrap().trim_right_matches(".crate").to_string();
            name = i.join("-");
        }

        let size = if recursive {
            let walkdir = WalkDir::new(path.display().to_string());

            walkdir
                .into_iter()
                .map(|e| e.unwrap().path().to_owned())
                .filter(|f| f.exists())
                .collect::<Vec<_>>()
                .par_iter()
                .map(|f| {
                    fs::metadata(f)
                        .unwrap_or_else(|_| {
                            panic!("Failed to get metadata of file '{}'", &path.display())
                        })
                        .len()
                })
                .sum()
        } else {
            //  recursive ?
            fs::metadata(&path)
                .unwrap_or_else(|_| panic!("Failed to get metadata of file '{}'", &path.display()))
                .len()
        };

        Self {
            name,
            version,
            size,
        }
    } // fn new()
}

pub(crate) fn get_top_crates(limit: u32, ccd: &CargoCachePaths) -> String {
    // we now have all the sizes and names and version sorted

    let mut output = String::new();

    let sources = [
        &ccd.registry_sources,
        &ccd.registry_cache,
        &ccd.git_repos_bare,
        &ccd.git_checkouts,
    ];

    for cache_dir in &sources {
        // do not try to read nonexisting directory (issue #9)
        if !cache_dir.exists() {
            eprintln!(
                "Skipping '{}' because it doesn't exist.",
                cache_dir.display()
            );
            continue;
        }

        output.push_str(&format!("\nSummary for: {:?}\n", cache_dir));

        let recursive: bool = *cache_dir != &ccd.registry_cache;

        // if we check bare git repos or checkouts, we need to calculate sizes slightly different
        let is_git: bool = *cache_dir == &ccd.git_checkouts || *cache_dir == &ccd.git_repos_bare;

        let checkouts = *cache_dir == &ccd.git_checkouts;

        // get list of package all "...\.crate$" files and sort it
        let mut collection = Vec::new();
        if is_git {
            if checkouts {
                let crate_list = fs::read_dir(&cache_dir)
                    .unwrap()
                    .map(|cratepath| cratepath.unwrap().path())
                    .collect::<Vec<PathBuf>>();
                // need to take 2 levels into account
                let mut both_levels_vec: Vec<PathBuf> = Vec::new();
                for repo in crate_list {
                    for i in fs::read_dir(&repo)
                        .unwrap()
                        .map(|cratepath| cratepath.unwrap().path())
                    {
                        both_levels_vec.push(i);
                    }
                }
                collection.extend_from_slice(&both_levels_vec);
            } else {
                // not checkouts
                let crate_list = fs::read_dir(&cache_dir)
                    .unwrap()
                    .map(|cratepath| cratepath.unwrap().path())
                    .collect::<Vec<PathBuf>>();
                collection.extend_from_slice(&crate_list);
            }
        } else {
            for repo in fs::read_dir(cache_dir).unwrap() {
                let crate_list = fs::read_dir(&repo.unwrap().path())
                    .unwrap()
                    .map(|cratepath| cratepath.unwrap().path())
                    .collect::<Vec<PathBuf>>();

                collection.extend_from_slice(&crate_list);
            }
        }
        collection.sort();

        let collections_vec = collection
            .iter()
            .map(|path| FileDesc::new(path, recursive, checkouts))
            .collect::<Vec<_>>();

        let mut summary: Vec<String> = Vec::new();
        let mut current_name = String::new();
        let mut counter: u32 = 0;
        let mut total_size: u64 = 0;

        // first find out max_cratename_len
        let max_cratename_len = collections_vec.iter().map(|p| p.name.len()).max().unwrap();

        #[cfg_attr(feature = "cargo-clippy", allow(clippy::if_not_else))]
        collections_vec.into_iter().for_each(|pkg| {
            {
                if pkg.name != current_name {
                    // don't push the first empty string
                    if !current_name.is_empty() {
                        let total_size_hr = total_size.file_size(file_size_opts::DECIMAL).unwrap();
                        let average_crate_size = (total_size / u64::from(counter))
                            .file_size(file_size_opts::DECIMAL)
                            .unwrap();

                        if *cache_dir == &ccd.registry_sources {
                            summary.push(format!(
                                "{:0>20} {: <width$} src ckt: {: <3} {: <20}  total: {}\n",
                                total_size,
                                current_name,
                                counter,
                                format!("src avg: {: >9}", average_crate_size),
                                total_size_hr,
                                width = max_cratename_len
                            ));
                        } else if *cache_dir == &ccd.registry_cache {
                            summary.push(format!(
                                "{:0>20} {: <width$} archives: {: <3} {: <20}  total: {}\n",
                                total_size,
                                current_name,
                                counter,
                                format!("crate avg: {: >9}", average_crate_size),
                                total_size_hr,
                                width = max_cratename_len
                            ));
                        } else if *cache_dir == &ccd.git_repos_bare {
                            summary.push(format!(
                                "{:0>20} {: <width$} repo: {: <3} {: <20}  total: {}\n",
                                total_size,
                                current_name,
                                counter,
                                format!("repo avg: {: >9}", average_crate_size),
                                total_size_hr,
                                width = max_cratename_len
                            ));
                        } else if *cache_dir == &ccd.git_checkouts {
                            summary.push(format!(
                                "{:0>20} {: <width$} repo ckt: {: <3} {: <20}  total: {}\n",
                                total_size,
                                current_name,
                                counter,
                                format!("ckt avg: {: >9}", average_crate_size),
                                total_size_hr,
                                width = max_cratename_len
                            ));
                        } else {
                            unreachable!("unknown cache source dir summary requested!");
                        }
                    } // !current_name.is_empty()
                      // new package, reset counting
                    current_name = pkg.name;
                    counter = 1;
                    total_size = pkg.size;
                } else {
                    counter += 1;
                    total_size += pkg.size;
                }
            }
        });

        summary.sort();
        summary.reverse();

        for (c, i) in summary.into_iter().enumerate() {
            if c == limit as usize {
                break;
            }
            let i = &i[21..]; // remove first word used for sorting
            output.push_str(i);
        }
    }
    output
}
