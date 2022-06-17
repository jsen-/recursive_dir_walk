use crate::read_buf::{ReadBuf, ReadBuffer, SubReadBuffer};
use std::{
    alloc::{alloc, dealloc, Layout},
    mem, slice,
};

#[derive(Debug)]
pub struct Buffer {
    buf: ReadBuffer<'static>,
}

impl Buffer {
    pub fn alloc(capacity: usize) -> Self {
        let layout = Layout::from_size_align(capacity, mem::align_of::<u8>()).unwrap();
        let buf = unsafe {
            let ptr = alloc(layout);
            slice::from_raw_parts_mut(ptr.cast::<mem::MaybeUninit<u8>>(), layout.size())
        };
        Self {
            buf: ReadBuffer::new(buf),
        }
    }
}

impl ReadBuf for Buffer {
    type SubReadBuf<'s> = SubReadBuffer<'s, ReadBuffer<'static>>
    where
        Self: 's;

    fn uninit<'a>(&'a mut self) -> Self::SubReadBuf<'a> {
        self.buf.uninit()
    }
    fn init(&self) -> &[u8] {
        self.buf.init()
    }
    fn init_mut(&mut self) -> &mut [u8] {
        self.buf.init_mut()
    }
    fn data(&self) -> &[mem::MaybeUninit<u8>] {
        self.buf.data()
    }
    fn data_mut(&mut self) -> &mut [mem::MaybeUninit<u8>] {
        self.buf.data_mut()
    }
    fn len(&self) -> usize {
        self.buf.len()
    }
    fn init_len(&self) -> usize {
        self.buf.init_len()
    }
    fn uninit_len(&self) -> usize {
        self.buf.uninit_len()
    }
    fn clear(&mut self) {
        self.buf.clear()
    }
    unsafe fn set_init_len(&mut self, len: usize) {
        self.buf.set_init_len(len)
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        let layout =
            Layout::from_size_align(self.buf.data_mut().len(), mem::align_of::<u8>()).unwrap();
        unsafe {
            dealloc(self.buf.data_mut().as_mut_ptr().cast::<u8>(), layout);
        }
    }
}
