use std::{
    ffi::c_void,
    fs::{File, Metadata, OpenOptions},
    io::{self, Result},
    mem::{size_of, MaybeUninit},
    os::windows::fs::OpenOptionsExt,
    os::windows::prelude::AsRawHandle,
    os::windows::prelude::*,
    path::Path,
};

use aligned::{Aligned, A8};
use windows_sys::Win32::{
    Foundation::{
        DuplicateHandle, DUPLICATE_SAME_ACCESS, ERROR_CANT_RESOLVE_FILENAME,
        ERROR_NOT_A_REPARSE_POINT, HANDLE,
    },
    Storage::FileSystem::{
        FileBasicInfo, FileIdInfo, GetFileInformationByHandleEx, SetFileInformationByHandle,
        FILE_ATTRIBUTE_NORMAL, FILE_BASIC_INFO, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_INFO, MAXIMUM_REPARSE_DATA_BUFFER_SIZE,
        REPARSE_GUID_DATA_BUFFER,
    },
    System::{
        Ioctl::FSCTL_GET_REPARSE_POINT,
        SystemServices::{IO_REPARSE_TAG_MOUNT_POINT, IO_REPARSE_TAG_SYMLINK},
        Threading::GetCurrentProcess,
        IO::DeviceIoControl,
    },
};

use super::{io::Io, path_components::PathComponents};

pub(crate) struct WindowsIo;

// basically FILE_ID_INFO but declared primitives to permit derives.
#[derive(Debug, PartialEq)]
pub(crate) struct VSNFileId {
    vsn: u64,
    file_id: [u8; 16],
}

impl Io for WindowsIo {
    fn duplicate_fd(f: &mut File) -> io::Result<File> {
        let mut new_handle: MaybeUninit<*mut c_void> = MaybeUninit::uninit();

        let result = unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                f.as_raw_handle() as HANDLE,
                GetCurrentProcess(),
                new_handle.as_mut_ptr() as *mut HANDLE,
                0,
                false as i32,
                DUPLICATE_SAME_ACCESS,
            )
        };
        if result == 0 {
            return Err(std::io::Error::last_os_error());
        }

        let new_handle = unsafe { new_handle.assume_init() };
        Ok(unsafe { File::from_raw_handle(new_handle) })
    }

    fn open_dir(p: &Path) -> Result<File> {
        let mut options = OpenOptions::new();
        options.read(true);
        options.custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
        let maybe_dir = options.open(p)?;
        if is_symlink(maybe_dir.as_raw_handle() as HANDLE)? {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Path is a directory link, not directory",
            ));
        }
        Ok(maybe_dir)
    }

    type UniqueIdentifier = VSNFileId;

    fn unique_identifier(d: &File) -> io::Result<Self::UniqueIdentifier> {
        let mut info = MaybeUninit::<FILE_ID_INFO>::uninit();
        let bool_result = unsafe {
            GetFileInformationByHandleEx(
                d.as_raw_handle() as HANDLE,
                FileIdInfo,
                info.as_mut_ptr() as *mut c_void,
                size_of::<FILE_ID_INFO>() as u32,
            )
        };
        if bool_result == 0 {
            return Err(io::Error::last_os_error());
        }
        let info = unsafe { info.assume_init() };
        Ok(VSNFileId {
            vsn: info.VolumeSerialNumber,
            file_id: info.FileId.Identifier,
        })
    }

    fn clear_readonly(
        f: &File,
        _dir_debug_root: &'_ PathComponents<'_>,
        metadata: &Metadata,
    ) -> io::Result<()> {
        if metadata.permissions().readonly() {
            // TODO use the FileDispositionEx interface to avoid resetting at all.
            #[cfg(feature = "log")]
            log::trace!("clearing permissions: {}", &_dir_debug_root);
            // set the readonly bit off. TODO: could read the times from the
            // directory listing iterator metadata. But as we've been asked to
            // delete, who cares?
            let mut info = FILE_BASIC_INFO {
                FileAttributes: FILE_ATTRIBUTE_NORMAL,
                CreationTime: 0,
                LastAccessTime: 0,
                LastWriteTime: 0,
                ChangeTime: 0,
            };
            use std::ffi::c_void;
            let result = unsafe {
                SetFileInformationByHandle(
                    f.as_raw_handle() as HANDLE,
                    FileBasicInfo,
                    &mut info as *mut FILE_BASIC_INFO as *mut c_void,
                    size_of::<FILE_BASIC_INFO>() as u32,
                )
            };
            if result == 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    fn is_eloop(e: &io::Error) -> bool {
        e.raw_os_error() == Some(ERROR_CANT_RESOLVE_FILENAME as i32)
    }
}

fn is_symlink(handle: HANDLE) -> Result<bool> {
    let mut reparse_buffer: Aligned<
        A8,
        [MaybeUninit<u8>; MAXIMUM_REPARSE_DATA_BUFFER_SIZE as usize],
    > = Aligned([MaybeUninit::<u8>::uninit(); MAXIMUM_REPARSE_DATA_BUFFER_SIZE as usize]);
    let mut out_size = 0;
    let bool_result = unsafe {
        DeviceIoControl(
            handle,
            FSCTL_GET_REPARSE_POINT,
            std::ptr::null(),
            0,
            // output buffer
            reparse_buffer.as_mut_ptr().cast(),
            // size of output buffer
            MAXIMUM_REPARSE_DATA_BUFFER_SIZE,
            // number of bytes returned
            &mut out_size,
            // OVERLAPPED structure
            std::ptr::null_mut(),
        )
    };
    if bool_result == 0 {
        let e = io::Error::last_os_error();
        if e.raw_os_error() != Some(ERROR_NOT_A_REPARSE_POINT as i32) {
            return Err(e);
        } else {
            return Ok(false);
        }
    }
    // Might be a non-link reparse point
    if out_size < size_of::<u32>() as u32 {
        // Success but not enough data to read the tag
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Insufficient data from DeviceIOControl",
        ));
    }
    let reparse_buffer = reparse_buffer.as_ptr().cast::<REPARSE_GUID_DATA_BUFFER>();
    Ok(unsafe {
        matches!(
            (*reparse_buffer).ReparseTag,
            IO_REPARSE_TAG_SYMLINK | IO_REPARSE_TAG_MOUNT_POINT
        )
    })
}
