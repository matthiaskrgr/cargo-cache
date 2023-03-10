//! Private trait to deal with OS variance

use std::{
    fmt::Debug,
    fs::{File, Metadata},
    io,
    path::Path,
};

use super::path_components::PathComponents;

pub(crate) trait Io {
    type UniqueIdentifier: PartialEq + Debug;

    fn duplicate_fd(f: &mut File) -> io::Result<File>;

    fn open_dir(p: &Path) -> io::Result<File>;
    fn unique_identifier(d: &File) -> io::Result<Self::UniqueIdentifier>;

    fn clear_readonly(
        f: &File,
        dir_debug_root: &'_ PathComponents<'_>,
        metadata: &Metadata,
    ) -> io::Result<()>;

    fn is_eloop(e: &io::Error) -> bool;
}
