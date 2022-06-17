use std::{
    ffi::{CStr, OsStr, OsString},
    fmt,
    ops::Deref,
    path::Path,
};

#[derive(Clone)]
pub struct CPathBuf(Box<[u8]>);

impl CPathBuf {
    pub fn join(&self, name: &CStr) -> Self {
        let name = name.to_bytes_with_nul();
        // `self.path.len()` includes the zero character which we'll replace with '/' and append `name` and it's zero character
        let mut buf = Vec::with_capacity(self.0.len() + name.len());
        buf.extend(&self.0[..self.0.len() - 1]);
        buf.push(b'/');
        buf.extend(name);
        Self(buf.into_boxed_slice())
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.0[..self.0.len() - 1]
    }
}

impl AsRef<CStr> for CPathBuf {
    fn as_ref(&self) -> &CStr {
        &self
    }
}

impl Deref for CPathBuf {
    type Target = CStr;
    fn deref(&self) -> &CStr {
        unsafe { CStr::from_bytes_with_nul_unchecked(&self.0) }
    }
}

impl From<&str> for CPathBuf {
    fn from(src: &str) -> Self {
        let mut buf = Vec::with_capacity(src.len() + 1);
        buf.extend(src.as_bytes());
        buf.push(0);
        Self(buf.into_boxed_slice())
    }
}

impl From<&OsStr> for CPathBuf {
    fn from(src: &OsStr) -> Self {
        let buf = {
            #[cfg(target_family = "unix")]
            {
                use std::os::unix::ffi::OsStrExt;
                let src = src.as_bytes();
                let mut buf = Vec::with_capacity(src.len() + 1);
                buf.extend(src);
                buf.push(0);
                buf
            }
            #[cfg(target_family = "windows")]
            {
                // origin: https://stackoverflow.com/a/59224987
                let mut buf = Vec::new();
                use std::os::windows::ffi::OsStrExt;
                let it = src
                    .encode_wide()
                    .chain(Some(0))
                    .map(|b| {
                        let b = b.to_ne_bytes();
                        b.get(0).map(|s| *s).into_iter().chain(b.get(1).map(|s| *s))
                    })
                    .flatten();
                buf.extend(it);
                buf.shrink_to_fit();
                buf
            }
        };
        Self(buf.into_boxed_slice())
    }
}

impl fmt::Debug for CPathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = String::from_utf8_lossy(&self.0);
        f.write_str(&p)
    }
}

impl From<OsString> for CPathBuf {
    fn from(src: OsString) -> Self {
        src.into()
    }
}
impl From<&Path> for CPathBuf {
    fn from(src: &Path) -> Self {
        Self::from(OsStr::new(src))
    }
}
