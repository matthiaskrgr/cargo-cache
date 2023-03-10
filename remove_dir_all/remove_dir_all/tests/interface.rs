//! Tests of the contract behaviour. Not intended to prove freedom from races
//! etc, but rather the interface which if changed will affect callers.

use std::{
    fs::{self},
    path::Path,
};

use tempfile::TempDir;
use test_log::test;

macro_rules! assert_not_found {
    ($path:expr) => {{
        match fs::metadata($path) {
            Ok(_) => panic!(
                "did not expect to retrieve metadata for {}",
                $path.display()
            ),
            Err(ref err) if err.kind() != ::std::io::ErrorKind::NotFound => {
                panic!(
                    "expected path {} to be NotFound, was {:?}",
                    $path.display(),
                    err
                )
            }
            _ => {}
        }
    }};
}

#[track_caller]
fn assert_empty(path: &Path) {
    assert_eq!(
        [(); 0],
        fs::read_dir(path).unwrap().map(|_e| ()).collect::<Vec<_>>()[..]
    );
}

#[track_caller]
fn assert_exists(path: &Path) {
    fs::symlink_metadata(path).unwrap();
}

// ensure_dir_empty

#[allow(deprecated)]
#[test]
fn ensure_empty_dir_missing_dir() {
    let tempdir = TempDir::new().unwrap();
    let path = tempdir.path().join("newdir");
    remove_dir_all::ensure_empty_dir(&path).unwrap();
    assert_empty(&path);
}

#[allow(deprecated)]
#[test]
fn ensure_empty_dir_existing_dir() {
    let tempdir = TempDir::new().unwrap();
    let path = tempdir.path().join("newdir");
    fs::create_dir(&path).unwrap();
    remove_dir_all::ensure_empty_dir(&path).unwrap();
    assert_empty(&path);
}

#[allow(deprecated)]
#[test]
fn ensure_empty_dir_not_empty() {
    let tempdir = TempDir::new().unwrap();
    let path = tempdir.path().join("newdir");
    fs::create_dir(&path).unwrap();
    log::trace!("{path:?}");
    fs::write(path.join("child"), b"aa").unwrap();
    remove_dir_all::ensure_empty_dir(&path).unwrap();
    assert_empty(&path);
}

#[allow(deprecated)]
#[test]
fn ensure_empty_dir_is_file() {
    let tempdir = TempDir::new().unwrap();
    let path = tempdir.path().join("newfile");
    fs::write(&path, b"aa").unwrap();
    remove_dir_all::ensure_empty_dir(&path).unwrap_err();
    assert_exists(&path);
}

#[allow(deprecated)]
#[test]
fn ensure_empty_dir_is_filelink() {
    let tempdir = TempDir::new().unwrap();
    let path = tempdir.path().join("newlink");
    #[cfg(windows)]
    std::os::windows::fs::symlink_file("target", &path).unwrap();
    #[cfg(not(windows))]
    std::os::unix::fs::symlink("target", &path).unwrap();
    remove_dir_all::ensure_empty_dir(&path).unwrap_err();
    assert_exists(&path);
}

#[allow(deprecated)]
#[cfg(windows)]
#[test]
fn ensure_empty_dir_is_dirlink() {
    let tempdir = TempDir::new().unwrap();
    let path = tempdir.path().join("newlink");
    std::os::windows::fs::symlink_dir("target", &path).unwrap();
    remove_dir_all::ensure_empty_dir(&path).unwrap_err();
    assert_exists(&path);
}

#[allow(deprecated)]
#[test]
fn removes_empty() {
    let tempdir = TempDir::new().unwrap();
    let path = tempdir.path().join("empty");
    fs::create_dir_all(&path).unwrap();
    assert!(fs::metadata(&path).unwrap().is_dir());

    remove_dir_all::remove_dir_all(&path).unwrap();
    assert_not_found!(&path);
}

#[allow(deprecated)]
#[test]
fn removes_files() {
    let tempdir = TempDir::new().unwrap();
    let path = tempdir.path().join("files");

    fs::create_dir_all(&path).unwrap();

    for i in 0..5 {
        let filename = format!("empty-{}.txt", i);
        let filepath = path.join(filename);

        {
            let mut _file = fs::File::create(&filepath);
        }

        assert!(fs::metadata(&filepath).unwrap().is_file());
    }

    remove_dir_all::remove_dir_all(&path).unwrap();
    assert_not_found!(&path);
}

#[allow(deprecated)]
#[test]
fn removes_dirs() {
    let tempdir = TempDir::new().unwrap();
    let path = tempdir.path().join("dirs");

    for i in 0..5 {
        let subpath = path.join(format!("{i}")).join("subdir");

        log::trace!("making dir {}", subpath.display());
        fs::create_dir_all(&subpath).unwrap();

        assert!(fs::metadata(&subpath).unwrap().is_dir());
    }

    remove_dir_all::remove_dir_all(&path).unwrap();
    assert_not_found!(&path);
}

#[allow(deprecated)]
#[test]
#[cfg(windows)]
fn removes_read_only() {
    let tempdir = TempDir::new().unwrap();
    let path = tempdir.path().join("readonly");

    for i in 0..5 {
        let subpath = path.join(format!("{}/subdir", i));

        fs::create_dir_all(&subpath).unwrap();

        let file_path = subpath.join("file.txt");
        {
            log::trace!("create: {}", file_path.display());
            let file = fs::File::create(&file_path).unwrap();

            if i % 2 == 0 {
                log::trace!("making readonly: {}", file_path.display());
                let metadata = file.metadata().unwrap();
                let mut permissions = metadata.permissions();
                permissions.set_readonly(true);

                fs::set_permissions(&file_path, permissions).unwrap();
            }
        }

        assert_eq!(
            i % 2 == 0,
            fs::metadata(&file_path).unwrap().permissions().readonly()
        );

        if i % 2 == 1 {
            log::trace!("making readonly: {}", subpath.display());
            let metadata = fs::metadata(&subpath).unwrap();

            let mut permissions = metadata.permissions();
            permissions.set_readonly(true);

            fs::set_permissions(&subpath, permissions).unwrap();

            assert!(fs::metadata(&subpath).unwrap().permissions().readonly());
        }
    }

    remove_dir_all::remove_dir_all(&path).unwrap();
    assert_not_found!(&path);
}

#[allow(deprecated)]
#[test]
fn removes_symlinks() {
    let tempdir = TempDir::new().unwrap();
    let root = tempdir.path().join("root");
    let link_name = root.join("some_link");
    let link_target = tempdir.path().join("target");
    fs::File::create(&link_target).unwrap();
    fs::create_dir(&root).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&link_target, &link_name).unwrap();
    #[cfg(not(windows))]
    std::os::unix::fs::symlink(&link_target, &link_name).unwrap();
    remove_dir_all::ensure_empty_dir(&root).unwrap();
    assert_exists(&root);
    assert_exists(&link_target);
}

// TODO: Should probably test readonly hard links...
