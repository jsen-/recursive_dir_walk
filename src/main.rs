#![feature(generic_associated_types)]
#![feature(maybe_uninit_slice)]

pub mod buffer;
pub mod cpathbuf;
pub mod dir_entry;
pub mod error;
pub mod read_buf;
pub mod shared_fd;

use std::{
    cmp, env,
    ffi::CStr,
    io::{self, Write},
    os::unix::io::RawFd,
    path::Path,
    thread,
};

use buffer::Buffer;
use cpathbuf::CPathBuf;
use dir_entry::{DirEntryIter, EntryType};
use error::{MyError, MyResult};
use flume::RecvError;
use read_buf::ReadBuf;
use shared_fd::SharedFd;
use syscalls::{syscall3, Sysno};

unsafe fn getdents64(fd: RawFd, buf: &mut Buffer) -> Result<usize, MyError> {
    let len = syscall3(
        Sysno::getdents64,
        fd as usize,
        buf.data_mut().as_mut_ptr() as usize,
        buf.len(),
    )
    .map_err(|errno| MyError::GetDEnts64(io::Error::from_raw_os_error(errno.into_raw())))?;
    buf.set_init_len(len);
    Ok(len)
}
unsafe fn close(fd: RawFd) -> Result<(), MyError> {
    let ret = libc::close(fd);
    if ret < 0 {
        return Err(MyError::Close(io::Error::from_raw_os_error(ret)));
    }
    Ok(())
}
unsafe fn openat64(path: &CStr) -> Result<RawFd, MyError> {
    let ret = libc::openat64(
        libc::AT_FDCWD,
        path.as_ptr(),
        libc::O_CLOEXEC | libc::O_NOATIME | libc::O_NOFOLLOW | libc::O_RDONLY | libc::O_DIRECTORY,
    );
    if ret < 1 {
        return Err(MyError::OpenSubdir(io::Error::from_raw_os_error(ret)));
    }
    Ok(ret)
}

enum WorkRequest {
    Open(CPathBuf),
    ReadDir(CPathBuf, SharedFd),
    Close(CPathBuf, RawFd),
}
enum WorkResponse {
    Open(CPathBuf, MyResult<SharedFd>),
    ReadDir(CPathBuf, SharedFd, MyResult<Buffer>),
    Close(CPathBuf, MyResult<()>),
}

fn worker(req_recv: flume::Receiver<WorkRequest>, res_send: flume::Sender<WorkResponse>) {
    loop {
        match req_recv.recv() {
            Err(RecvError::Disconnected) => return,
            Ok(WorkRequest::Open(path)) => {
                let res = unsafe { openat64(&path) }.map(|fd| SharedFd::new(fd));
                res_send.send(WorkResponse::Open(path, res)).unwrap();
            }
            Ok(WorkRequest::ReadDir(path, mut fd)) => {
                let mut buf = Buffer::alloc(1024);
                let res = unsafe {
                    getdents64(fd.get().fd(), &mut buf).map(move |len| {
                        buf.set_init_len(len);
                        buf
                    })
                };
                res_send.send(WorkResponse::ReadDir(path, fd, res)).unwrap();
            }
            Ok(WorkRequest::Close(path, fd)) => {
                let res = unsafe { close(fd) };
                res_send.send(WorkResponse::Close(path, res)).unwrap();
            }
        };
    }
}

pub fn read_dir_multi_thread<P: AsRef<Path>>(path: P) -> Result<(), MyError> {
    const THREAD_COUNT: usize = 30;
    let path = path.as_ref();
    let mut in_progress = 1;
    let mut max = 0;
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    let (req_send, req_recv) = flume::unbounded();
    let (res_send, res_recv) = flume::unbounded();

    println!("{path:?}");
    let root = WorkRequest::Open(CPathBuf::from(path));
    req_send.send(root).unwrap();

    let mut threads = Vec::with_capacity(THREAD_COUNT);
    for _ in 0..THREAD_COUNT {
        let req_recv = req_recv.clone();
        let res_send = res_send.clone();
        threads.push(thread::spawn(move || worker(req_recv, res_send)));
    }

    loop {
        if in_progress == 0 {
            return Ok(());
        }
        let received = res_recv.recv().unwrap();
        max = cmp::max(max, in_progress);
        in_progress -= 1;
        match received {
            WorkResponse::Open(path, result) => match result {
                Ok(fd) => {
                    in_progress += 1;
                    req_send.send(WorkRequest::ReadDir(path, fd)).unwrap();
                }
                Err(err) => {
                    eprintln!("Error opening directory \"{path:?}\": {:?}", err);
                }
            },
            WorkResponse::ReadDir(path, fd, buffer) => {
                match buffer {
                    Ok(mut buf) => {
                        if buf.init().len() != 0 {
                            in_progress += 1;
                            req_send
                                .send(WorkRequest::ReadDir(path.clone(), fd.clone()))
                                .unwrap();
                            for entry in DirEntryIter::new(&mut buf) {
                                stdout.write(path.as_slice()).unwrap();
                                stdout.write(b"\n").unwrap();
                                if entry.ty == EntryType::Dir {
                                    in_progress += 1;
                                    req_send
                                        .send(WorkRequest::Open(path.join(entry.c_name())))
                                        .unwrap();
                                } else if entry.ty != EntryType::Regular {
                                    panic!("{entry:?}");
                                }
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("Error reading directory \"{path:?}\": {:?}", err);
                    }
                }
                if let Some(raw_fd) = fd.release() {
                    in_progress += 1;
                    req_send.send(WorkRequest::Close(path, raw_fd)).unwrap()
                }
            }
            WorkResponse::Close(path, result) => {
                if let Err(err) = result {
                    eprintln!("Error closing directory \"{path:?}\": {:?}", err);
                }
            }
        }
    }
}

fn main() {
    match env::args_os().skip(1).next() {
        None => eprintln!("Usage: recursive_dir_walk <root>"),
        Some(root) => {
            read_dir_multi_thread(root).unwrap();
        }
    }
}
