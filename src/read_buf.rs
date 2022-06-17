use std::{fmt, io, mem::MaybeUninit};

pub trait Read2 {
    fn read_buf<'me, 'f, B: ReadBuf>(&'me mut self, buf: &'f mut B) -> io::Result<usize>;
}

pub trait ReadBuf {
    type SubReadBuf<'s>: ReadBuf
    where
        Self: 's;
    fn uninit(&mut self) -> Self::SubReadBuf<'_>;
    fn init(&self) -> &[u8];
    fn init_mut(&mut self) -> &mut [u8];
    // TODO: remove this method
    fn data(&self) -> &[MaybeUninit<u8>];
    fn data_mut(&mut self) -> &mut [MaybeUninit<u8>];
    fn len(&self) -> usize;
    fn init_len(&self) -> usize;
    fn uninit_len(&self) -> usize;
    fn clear(&mut self);
    unsafe fn set_init_len(&mut self, len: usize);
}

pub struct ReadBuffer<'a> {
    data: &'a mut [MaybeUninit<u8>],
    initialized: usize,
}
impl<'a> ReadBuffer<'a> {
    pub fn new(buf: &'a mut [MaybeUninit<u8>]) -> Self {
        Self {
            data: buf,
            initialized: 0,
        }
    }
}
impl fmt::Debug for ReadBuffer<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadBuffer")
            .field("initialized", &self.initialized)
            .field("data", &unsafe {
                MaybeUninit::slice_assume_init_ref(&self.data[0..self.initialized])
            })
            .finish()
    }
}

impl<'a> ReadBuf for ReadBuffer<'a> {
    type SubReadBuf<'s> = SubReadBuffer<'s, Self> where Self: 's;
    fn uninit(&mut self) -> Self::SubReadBuf<'_> {
        SubReadBuffer {
            offset: self.initialized,
            parent: self,
            initialized: 0,
        }
    }
    fn init(&self) -> &[u8] {
        unsafe { MaybeUninit::slice_assume_init_ref(&self.data[0..self.initialized]) }
    }
    fn init_mut(&mut self) -> &mut [u8] {
        unsafe { MaybeUninit::slice_assume_init_mut(&mut self.data[0..self.initialized]) }
    }
    fn data(&self) -> &[MaybeUninit<u8>] {
        self.data
    }
    fn data_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        self.data
    }
    fn len(&self) -> usize {
        self.data.len()
    }
    fn init_len(&self) -> usize {
        self.initialized
    }
    fn uninit_len(&self) -> usize {
        self.data.len() - self.initialized
    }
    unsafe fn set_init_len(&mut self, len: usize) {
        self.initialized = len;
    }
    fn clear(&mut self) {
        unsafe { self.set_init_len(0) }
    }
}

pub struct SubReadBuffer<'a, T: ReadBuf> {
    offset: usize,
    parent: &'a mut T,
    initialized: usize,
}

impl<T: ReadBuf> ReadBuf for SubReadBuffer<'_, T> {
    type SubReadBuf<'s> = SubReadBuffer<'s, Self> where Self: 's;
    fn uninit(&mut self) -> Self::SubReadBuf<'_> {
        SubReadBuffer {
            offset: self.parent.init_len() + self.initialized,
            parent: self,
            initialized: 0,
        }
    }
    fn init(&self) -> &[u8] {
        unsafe {
            MaybeUninit::slice_assume_init_ref(
                &self.parent.data()[self.offset..self.offset + self.initialized],
            )
        }
    }
    fn init_mut(&mut self) -> &mut [u8] {
        unsafe {
            MaybeUninit::slice_assume_init_mut(
                &mut self.parent.data_mut()[self.offset..self.offset + self.initialized],
            )
        }
    }
    fn data(&self) -> &[MaybeUninit<u8>] {
        &self.parent.data()[self.offset..]
    }
    fn data_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        &mut self.parent.data_mut()[self.offset..]
    }
    fn len(&self) -> usize {
        self.data().len()
    }
    fn init_len(&self) -> usize {
        self.initialized
    }
    fn uninit_len(&self) -> usize {
        self.parent.data().len() - self.offset - self.initialized
    }
    unsafe fn set_init_len(&mut self, len: usize) {
        self.initialized = len;
        self.parent.set_init_len(self.offset + len)
    }
    fn clear(&mut self) {
        unsafe { self.set_init_len(0) }
    }
}
