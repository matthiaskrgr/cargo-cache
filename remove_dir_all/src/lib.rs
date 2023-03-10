//! Reliably remove a directory and all of its children.
//!
//! This library provides an alternative implementation of
//! [`std::fs::remove_dir_all`] from the Rust std library. It varies in the
//! following ways:
//! - the `parallel` feature parallelises the deletion. This is useful when high
//!   syscall latency is occurring, such as on Windows (deletion IO accrues to
//!   the process), or network file systems of any kind. This is off by default.
//! - It tolerates files not being deleted atomically (this is a Windows
//!   specific behaviour).
//! - It resets the readonly flag on Windows as needed.
//!
//! Like `remove_dir_all` it assumes both that the caller has permission to
//! delete the files, and that they don't have permission to change permissions
//! to be able to delete the files: no ACL or chmod changes are made during
//! deletion. This is because hardlinks can cause such changes to show up and
//! affect the filesystem outside of the directory tree being deleted.
//!   
//! The extension trait [`RemoveDir`] can be used to invoke `remove_dir_all` on
//! an open [`File`](std::fs::File), where it will error if the file is not a directory,
//! and otherwise delete the contents. This allows callers to be more confident that
//! what is deleted is what was requested even in the presence of malicious
//! actors changing the filesystem concurrently.
//!
//! The functions [`remove_dir_all`], [`remove_dir_contents`], and [`ensure_empty_dir`]
//! are intrinsically sensitive to file system races, as the path to the
//! directory to delete can be substituted by an attacker inserting a symlink
//! along that path. Relative paths with one path component are the least
//! fragile, but using [`RemoveDir::remove_dir_contents`] is recommended.
//!
//! ## Features
//!
//! - parallel: When enabled, deletion of directories is parallised. (#parallel)[more details]
//! - log: Include some log messages about the deletion taking place.
//!
//! About the implementation. The implementation prioritises security, then
//! robustness (e.g. low resource situations), and then finally performance.
//!
//! ## Security
//!
//! On all platforms directory related race conditions are avoided by opening
//! paths and then iterating directory contents and deleting names in the
//! directory with _at style syscalls. This does not entirely address possible
//! races on unix style operating systems (but see the funlinkat call on
//! FreeBSD, which could if more widely adopted). It does prevent attackers from
//! replacing intermediary directories with symlinks in order to fool privileged
//! code into traversing outside the intended directory tree. This is the same
//! as the standard library implementation.
//!
//! This function is not designed to succeed in the presence of concurrent
//! actors in the tree being deleted - for instance, adding files to a directory
//! being deleted can prevent the directory being deleted for an arbitrary
//! period by extending the directory iterator indefinitely.
//!
//! Directory traversal only ever happens downwards. In future, to accommodate
//! very large directory trees (greater than file descriptor limits deep) the
//! same path may be traversed multiple times, and the quadratic nature of that
//! will be mitigated by a cache of open directories. See [#future-plans](Future
//! Plans)
//!
//! ## Robustness
//!
//! Every opened file has its type checked through the file handle, and then
//! unlinked or scanned as appropriate. Syscall overheads are minimised by
//! trust-but-verify of the node type metadata returned from directory scanning:
//! only names that appear to be directories get their contents scanned. The
//! consequence is that if an attacker replaces a non-directory with a
//! directory, or vice versa, an error will occur - but the `remove_dir_all` will
//! not escape from the directory tree. On Windows file deletion requires
//! obtaining a handle to the file, but again the kind metadata from the
//! directory scan is used to avoid re-querying the metadata. Symlinks are
//! detected by a failure to open a path with `O_NOFOLLOW`, they are unlinked with
//! no further processing.
//!
//! ## Serial deletion
//!
//! Serial deletion occurs recursively - open, read, delete
//! contents-except-for-directories, repeat.
//!
//! Parallel deletion builds on serial deletion by utilising a thread pool for
//! IO which can block:
//! - directory scanning
//! - calls to unlink and fstat
//! - file handle closing (yes, that can block)
//!
//! Parallel is usually a win, but some users may value compile time or size of
//! compiled code more, so the `parallel` feature is opt-in.
//!
//! We suggest permitting the end user to control this choice: when adding
//! remove-dir-all as a dependency to a library crate, expose a feature
//! "parallel" that sets `remove-dir-all/parallel`. This will permit the user of
//! your library to control the parallel feature inside `remove_dir_all`
//!
//! e.g.
//!
//! ```Cargo.toml
//! [features]
//! default = []
//! parallel = ["remove_dir_all/parallel"]
//! ...
//! [dependencies]
//! remove_dir_all = {version = "0.8"}
//!
//! ## Future Plans
//!  Open directory handles are kept in
//! a lg-spaced cache after the first 10 levels:
//! level10/skipped1/level12/skipped2/skipped3/skipped4/level16. If EMFILE is
//! encountered, no more handles are cached, and directories are opened by
//! re-traversing from the closest previously opened handle. Deletion should
//! succeed even only 4 file descriptors are available: one to hold the root,
//! two to iterate individual directories, and one to open-and-delete individual
//! files, though that will be quadratic in the depth of the tree, successfully
//! deleting leaves only on each iteration.
//!
//! IO Prioritisation:
//! 1) directory scanning when few paths are queued for deletion (to avoid
//!    ending up accidentally serial) - allowing keeping the other queues full.
//! 4) close/CloseHandle (free up file descriptors)
//! 2) rmdir (free up file descriptors)
//! 3) unlink/SetFileInformationByHandle (to free up directories so they can be
//!    rmdir'd)
//!
//! Scanning/unlinking/rmdiring is further biased by depth and lexicographic
//! order: this minimises the number of directories being worked on in parallel,
//! so very branchy trees are less likely to exhaust kernel resources or
//! application memory or thrash the open directory cache.

//! ```

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(rust_2018_idioms)]
// See under "known problems" https://rust-lang.github.io/rust-clippy/master/index.html#mutex_atomic
#![allow(clippy::mutex_atomic)]

use std::{io::Result, path::Path};

use normpath::PathExt;

#[cfg(doctest)]
#[macro_use]
extern crate doc_comment;

#[cfg(doctest)]
doctest!("../README.md");

mod _impl;

/// Extension trait adding `remove_dir_all` support to [`std::fs::File`].
pub trait RemoveDir {
    /// Remove the contents of the dir.
    ///
    /// `debug_root`: identifies the directory contents being removed
    fn remove_dir_contents(&mut self, debug_root: Option<&Path>) -> Result<()>;
}

/// Makes `path` an empty directory: if it does not exist, it is created it as
/// an empty directory (as if with [`std::fs::create_dir`]); if it does exist, its
/// contents are deleted (as if with [`remove_dir_contents`]).
///
/// It is an error if `path` exists but is not a directory, including a symlink
/// to a directory.
///
/// This is subject to file system races: a privileged process could be attacked
/// by replacing parent directories of the supplied path with a link (e.g. to
/// /etc). Consider using [`RemoveDir::remove_dir_contents`] instead.
pub fn ensure_empty_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    _impl::_ensure_empty_dir_path::<_impl::OsIo, _>(path)
}

/// Deletes the contents of `path`, but not the directory itself. It is an error
/// if `path` is not a directory.
///
/// This is subject to file system races: a privileged process could be attacked
/// by replacing parent directories of the supplied path with a link (e.g. to
/// /etc). Consider using [`RemoveDir::remove_dir_contents`] instead.
pub fn remove_dir_contents<P: AsRef<Path>>(path: P) -> Result<()> {
    _impl::_remove_dir_contents_path::<_impl::OsIo, P>(path)
}

/// Reliably removes a directory and all of its children.
///
/// ```rust
/// use std::fs;
/// use remove_dir_all::*;
///
/// fs::create_dir("./temp/").unwrap();
/// remove_dir_all("./temp/").unwrap();
/// ```
///
/// Note: calling this on a non-directory (e.g. a symlink to a directory) will
/// error.
///
/// [`RemoveDir::remove_dir_contents`] is somewhat safer and
/// recommended as the path based version is subject to file system races
/// determining what to delete: a privileged process could be attacked by
/// replacing parent directories of the supplied path with a link (e.g. to
/// /etc). Consider using [`RemoveDir::remove_dir_contents`] instead.
pub fn remove_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref().normalize()?;
    _impl::remove_dir_all_path::<_impl::OsIo, _>(path)
}

#[allow(deprecated)]
#[cfg(test)]
mod tests {
    //! functional tests for all platforms
    //!
    //! A note on safety: races are notoriously hard to secure merely via tests:
    //! these tests use a dedicated trait to allow sequencing attack operations,
    //! much the same as the test clock in Tokio programs. So these 'safe' tests
    //! are not actually attempting scheduling races, rather they are showing
    //! that the known attacks don't work. A fuzz based heuristic functional
    //! test would be a good addition to complement these tests.
    use super::Result;

    use std::fs::{self, File};
    use std::io;
    use std::path::PathBuf;

    use tempfile::TempDir;
    use test_log::test;

    use crate::ensure_empty_dir;
    use crate::remove_dir_all;
    use crate::remove_dir_contents;

    cfg_if::cfg_if! {
        if #[cfg(windows)] {
            const ENOTDIR:i32 = windows_sys::Win32::Foundation::ERROR_DIRECTORY as i32;
            const ENOENT:i32 = windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND as i32;
            const INVALID_INPUT:i32 = windows_sys::Win32::Foundation::ERROR_INVALID_PARAMETER as i32;
        } else {
            const ENOTDIR:i32 = libc::ENOTDIR;
            const ENOENT:i32 = libc::ENOENT;
            const INVALID_INPUT:i32 = libc::EINVAL;
        }
    }

    /// Expect a particular sort of failure
    fn expect_failure<T>(n: &[i32], r: io::Result<T>) -> io::Result<()> {
        match r {
            Err(e)
                if n.iter()
                    .map(|n| Option::Some(*n))
                    .any(|n| n == e.raw_os_error()) =>
            {
                Ok(())
            }
            Err(e) => {
                println!("{e} {:?}, {:?}, {:?}", e.raw_os_error(), e.kind(), n);
                Err(e)
            }
            Ok(_) => Err(io::Error::new(
                io::ErrorKind::Other,
                "unexpected success".to_string(),
            )),
        }
    }

    struct Prep {
        _tmp: TempDir,
        ours: PathBuf,
        file: PathBuf,
    }

    /// Create test setup: t.mkdir/file all in a tempdir.
    fn prep() -> Result<Prep> {
        let tmp = TempDir::new()?;
        let ours = tmp.path().join("t.mkdir");
        let file = ours.join("file");
        let nested = ours.join("another_dir");
        fs::create_dir(&ours)?;
        fs::create_dir(&nested)?;
        File::create(&file)?;
        File::open(&file)?;
        Ok(Prep {
            _tmp: tmp,
            ours,
            file,
        })
    }

    #[test]
    fn mkdir_rm() -> Result<()> {
        let p = prep()?;

        expect_failure(&[ENOTDIR, INVALID_INPUT], remove_dir_contents(&p.file))?;

        remove_dir_contents(&p.ours)?;
        expect_failure(&[ENOENT], File::open(&p.file))?;

        remove_dir_contents(&p.ours)?;
        remove_dir_all(&p.ours)?;
        expect_failure(&[ENOENT], remove_dir_contents(&p.ours))?;
        Ok(())
    }

    #[test]
    fn ensure_rm() -> Result<()> {
        let p = prep()?;

        expect_failure(&[ENOTDIR, INVALID_INPUT], ensure_empty_dir(&p.file))?;

        ensure_empty_dir(&p.ours)?;
        expect_failure(&[ENOENT], File::open(&p.file))?;
        ensure_empty_dir(&p.ours)?;

        remove_dir_all(&p.ours)?;
        ensure_empty_dir(&p.ours)?;
        File::create(&p.file)?;

        Ok(())
    }
}
