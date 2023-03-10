use std::{
    ffi::OsStr,
    fs::File,
    io::{ErrorKind, Result},
    path::Path,
};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

mod io;
mod path_components;

cfg_if::cfg_if! {
    if #[cfg(windows)] {
        mod win;
        pub(crate) use win::WindowsIo as OsIo;
    } else {
        mod unix;
        pub(crate) use unix::UnixIo as OsIo;
    }
}

impl super::RemoveDir for std::fs::File {
    fn remove_dir_contents(&mut self, debug_root: Option<&Path>) -> Result<()> {
        // thunk over to the free version adding in the os-specific IO trait impl
        let debug_root = match debug_root {
            None => PathComponents::Path(Path::new("")),
            Some(debug_root) => PathComponents::Path(debug_root),
        };
        _remove_dir_contents::<OsIo>(self, &debug_root)
    }
}

/// Entry point for deprecated function
pub(crate) fn _ensure_empty_dir_path<I: io::Io, P: AsRef<Path>>(path: P) -> Result<()> {
    // This is as TOCTOU safe as we can make it. Attacks via link replacements
    // in interior components of the path is still possible. if the create
    // succeeds, mission accomplished. if the create fails, open the dir
    // (subject to races again), and then proceed to delete the contents via the
    // descriptor.
    match std::fs::create_dir(&path) {
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            // Exists and is a dir. Open it
            let mut existing_dir = I::open_dir(path.as_ref())?;
            existing_dir.remove_dir_contents(Some(path.as_ref()))
        }
        otherwise => otherwise,
    }
}

// Deprecated entry point
pub(crate) fn _remove_dir_contents_path<I: io::Io, P: AsRef<Path>>(path: P) -> Result<()> {
    let mut d = I::open_dir(path.as_ref())?;
    _remove_dir_contents::<I>(&mut d, &PathComponents::Path(path.as_ref()))
}

/// exterior lifetime interface to dir removal
fn _remove_dir_contents<I: io::Io>(d: &mut File, debug_root: &PathComponents<'_>) -> Result<()> {
    let owned_handle = I::duplicate_fd(d)?;
    remove_dir_contents_recursive::<I>(owned_handle, debug_root)
}

/// deprecated interface
pub(crate) fn remove_dir_all_path<I: io::Io, P: AsRef<Path>>(path: P) -> Result<()> {
    let p = path.as_ref();
    // Opportunity 1 for races
    let d = I::open_dir(p)?;
    let debug_root = PathComponents::Path(if p.has_root() { p } else { Path::new(".") });
    remove_dir_contents_recursive::<OsIo>(d, &debug_root)?;
    // Opportunity 2 for races
    std::fs::remove_dir(&path)
}

use crate::RemoveDir;

use self::path_components::PathComponents;

// Core workhorse, heading towards this being able to be tasks.
#[allow(clippy::map_identity)]
fn remove_dir_contents_recursive<I: io::Io>(
    mut d: File,
    debug_root: &PathComponents<'_>,
) -> Result<()> {
    #[cfg(feature = "log")]
    log::trace!("scanning {}", &debug_root);
    // We take a os level clone of the FD so that there are no lifetime
    // concerns. It would *not* be ok to do readdir on one file twice
    // concurrently because of shared kernel state.
    let dirfd = I::duplicate_fd(&mut d)?;
    cfg_if::cfg_if! {
        if #[cfg(feature = "parallel")] {
            let iter = fs_at::read_dir(&mut d)?;
            let iter = iter.par_bridge();
        } else {
            let mut iter = fs_at::read_dir(&mut d)?;
        }
    }

    iter.try_for_each(|dir_entry| -> Result<()> {
        let dir_entry = dir_entry?;
        let name = dir_entry.name();
        if name == OsStr::new(".") || name == OsStr::new("..") {
            return Ok(());
        }
        let dir_path = Path::new(name);
        let dir_debug_root = PathComponents::Component(debug_root, dir_path);
        // Windows optimised: open everything always, which is not bad for
        // linux, and portable to OS's and FS's that don't expose inode type in
        // the readdir entries.

        let mut opts = fs_at::OpenOptions::default();
        opts.read(true)
            .write(fs_at::OpenOptionsWriteMode::Write)
            .follow(false);

        let child_result = opts.open_dir_at(&dirfd, name);
        let is_dir = match child_result {
            Err(e) if !I::is_eloop(&e) => return Err(e),
            Err(_) => false,
            Ok(child_file) => {
                let metadata = child_file.metadata()?;
                let is_dir = metadata.is_dir();
                I::clear_readonly(&child_file, &dir_debug_root, &metadata)?;

                if is_dir {
                    remove_dir_contents_recursive::<I>(child_file, &dir_debug_root)?;
                    #[cfg(feature = "log")]
                    log::trace!("rmdir: {}", &dir_debug_root);
                    let opts = fs_at::OpenOptions::default();
                    opts.rmdir_at(&dirfd, name).map_err(|e| {
                        #[cfg(feature = "log")]
                        log::debug!("error removing {}", dir_debug_root);
                        e
                    })?;
                }
                is_dir
            }
        };
        if !is_dir {
            #[cfg(feature = "log")]
            log::trace!("unlink: {}", &dir_debug_root);
            opts.unlink_at(&dirfd, name).map_err(|e| {
                #[cfg(feature = "log")]
                log::debug!("error removing {}", dir_debug_root);
                e
            })?;
        }

        #[cfg(feature = "log")]
        log::trace!("removed {}", dir_debug_root);
        Ok(())
    })?;
    #[cfg(feature = "log")]
    log::trace!("scanned {}", &debug_root);
    Ok(())
}
