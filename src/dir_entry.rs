use std::{
    ffi::{CStr, OsStr},
    fmt,
    marker::PhantomData,
    mem,
    os::unix::ffi::OsStrExt,
    slice,
};

use crate::{buffer::Buffer, read_buf::ReadBuf};

#[derive(Debug, Default)]
#[repr(C)]
struct linux_dirent {
    /// inode number
    d_ino: libc::ino64_t,
    /// offset to next linux_dirent
    d_off: libc::off64_t,
    /// length of this linux_dirent
    d_reclen: libc::c_ushort,
    /// file type
    d_type: libc::c_uchar,
    /// filename (null-terminated)
    d_name: [libc::c_char; 1],
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntryType {
    Unknown = libc::DT_UNKNOWN as isize,
    Fifo = libc::DT_FIFO as isize,
    CharDev = libc::DT_CHR as isize,
    Dir = libc::DT_DIR as isize,
    BlockDev = libc::DT_BLK as isize,
    Regular = libc::DT_REG as isize,
    Symlink = libc::DT_LNK as isize,
    Socket = libc::DT_SOCK as isize,
}

impl fmt::Display for EntryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use EntryType::*;
        let s = match self {
            Unknown => "Unknown",
            Fifo => "Fifo",
            CharDev => "CharDev",
            Dir => "Dir",
            BlockDev => "BlockDev",
            Regular => "Regular",
            Symlink => "Symlink",
            Socket => "Socket",
        };
        f.pad(s)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Entry<'a> {
    pub inode: libc::ino64_t,
    pub ty: EntryType,
    pub name: &'a OsStr,
}

impl Entry<'_> {
    pub fn c_name(&self) -> &'_ CStr {
        // Safety
        // the file names in the buffer obtained from kernel are zero-terminated
        unsafe { CStr::from_ptr(self.name.as_bytes().as_ptr().cast::<i8>()) }
    }
}

pub struct DirEntryIter<'a> {
    ptr: *const u8,
    end: *const u8,
    _p: PhantomData<&'a mut Buffer>, // let's pretend we own the buffer
}

impl<'a> DirEntryIter<'a> {
    pub fn new(buf: &'a Buffer) -> Self {
        let init = buf.init();
        let ptr = init.as_ptr();
        Self {
            ptr,
            // Safety:
            // `end` points one-byte-past-the-allocation, which is allowed
            //
            end: unsafe {
                ptr.offset(
                    init.len()
                        .try_into()
                        .expect("buffer size does not fit into `isize`"),
                )
            },
            _p: PhantomData,
        }
    }
}

impl<'a> Iterator for DirEntryIter<'a> {
    type Item = Entry<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        fn get_name(name: *const u8, len: usize) -> &'static OsStr {
            let bytes = unsafe { slice::from_raw_parts(name, len) };
            OsStr::from_bytes(bytes)
        }
        fn get_type(code: u8) -> EntryType {
            unsafe { mem::transmute(code) }
        }

        loop {
            if self.ptr == self.end {
                return None;
            }
            let d = unsafe { &*self.ptr.cast::<linux_dirent>() };
            self.ptr = unsafe { self.ptr.offset(d.d_reclen as isize) };

            let mut name_ptr = d.d_name.as_ptr().cast::<u8>();
            let add_len = unsafe {
                match name_ptr.read() {
                    0 => {
                        return Some(Entry {
                            inode: d.d_ino,
                            name: get_name(d.d_name.as_ptr().cast::<u8>(), 0),
                            ty: get_type(d.d_type),
                        })
                    }
                    b'.' => {
                        name_ptr = name_ptr.offset(1);
                        match name_ptr.read() {
                            0 => continue,
                            b'.' => {
                                name_ptr = name_ptr.offset(1);
                                match name_ptr.read() {
                                    0 => continue,
                                    _ => 2,
                                }
                            }
                            _ => 1,
                        }
                    }
                    _ => 0,
                }
            };
            let name_len = unsafe { libc::strlen(name_ptr.cast::<i8>()) + add_len };
            return Some(Entry {
                inode: d.d_ino,
                name: get_name(d.d_name.as_ptr().cast::<u8>(), name_len),
                ty: get_type(d.d_type),
            });
        }
    }
}
