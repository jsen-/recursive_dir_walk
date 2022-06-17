use std::io;

#[derive(Debug)]
pub enum MyError {
    Open(io::Error),
    GetDEnts64(io::Error),
    GetDirEntries(io::Error),
    Close(io::Error),
    FdOpenDir(io::Error),
    ReadDir(io::Error),
    OpenSubdir(io::Error),
    USleep(io::Error),
}

pub type MyResult<T> = Result<T, MyError>;
