use std::{
    mem,
    os::unix::io::RawFd,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct FdWrap<'a> {
    fd: RawFd,
    uses: &'a mut AtomicUsize,
}

impl FdWrap<'_> {
    pub unsafe fn fd(&self) -> RawFd {
        self.fd
    }
}

impl<'a> Drop for FdWrap<'a> {
    fn drop(&mut self) {
        let count = self.uses.fetch_sub(1, Ordering::SeqCst);
        if count == 0 {
            panic!("SharedFd count dropped to zero")
        }
    }
}

pub struct SharedFd {
    fd: RawFd,
    uses: *mut AtomicUsize,
}

impl SharedFd {
    pub fn new(fd: RawFd) -> Self {
        let b = Box::new(AtomicUsize::new(1));
        let uses = Box::into_raw(b);
        Self { fd, uses }
    }
    pub fn get(&mut self) -> FdWrap {
        let uses = self.increment();
        FdWrap { fd: self.fd, uses }
    }
    pub fn release(self) -> Option<RawFd> {
        let me = mem::ManuallyDrop::new(self);
        if me.decrement() == 1 {
            Some(me.fd)
        } else {
            None
        }
    }
    fn increment(&self) -> &mut AtomicUsize {
        let uses = unsafe { &mut *self.uses };
        debug_assert!(uses.load(Ordering::SeqCst) < usize::MAX);
        uses.fetch_add(1, Ordering::SeqCst);
        uses
    }
    fn decrement(&self) -> usize {
        let count = unsafe { &mut *self.uses }.fetch_sub(1, Ordering::SeqCst);
        if count == 0 {
            panic!("SharedFd count dropped to zero")
        }
        count
    }
}

unsafe impl Send for SharedFd {}

impl Clone for SharedFd {
    fn clone(&self) -> Self {
        self.increment();
        Self {
            fd: self.fd,
            uses: self.uses,
        }
    }
}

impl Drop for SharedFd {
    fn drop(&mut self) {
        self.decrement();
    }
}
