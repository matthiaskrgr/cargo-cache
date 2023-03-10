use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::prelude::FromRawFd;
use std::path::Path;
use std::{fs, os::unix::prelude::AsRawFd};

use cvt::cvt;
use libc::{self, fcntl, F_DUPFD_CLOEXEC};

use super::io::Io;

pub(crate) struct UnixIo;

impl Io for UnixIo {
    type UniqueIdentifier = ();

    fn duplicate_fd(f: &mut fs::File) -> io::Result<fs::File> {
        let source_fd = f.as_raw_fd();
        // F_DUPFD_CLOEXEC seems to be quite portable, but we should be prepared
        // to add in more codepaths here.
        let fd = cvt(unsafe { fcntl(source_fd, F_DUPFD_CLOEXEC, 0) })?;
        Ok(unsafe { File::from_raw_fd(fd) })
    }

    fn open_dir(p: &Path) -> io::Result<fs::File> {
        let mut options = OpenOptions::new();
        options.read(true);
        options.custom_flags(libc::O_NOFOLLOW);
        options.open(p)
    }

    fn unique_identifier(_d: &fs::File) -> io::Result<Self::UniqueIdentifier> {
        todo!()
    }

    fn clear_readonly(
        _f: &fs::File,
        _dir_debug_root: &'_ super::path_components::PathComponents<'_>,
        _metadata: &fs::Metadata,
    ) -> io::Result<()> {
        // can't delete contents of a directory without 'w' on the directory, so
        // you might expect to see logic here to check a directory. that said,
        // remove_dir_all doesn't concern itself with permissions; it does
        // concern itself with the readonly attribute on windows - but that is
        // not a file permission.
        Ok(())
    }

    fn is_eloop(e: &io::Error) -> bool {
        e.raw_os_error() == Some(libc::ELOOP)
    }
}
