//! Very simple channel that implements read and write
//!
//! These channels are not thread or async safe. They can only be safely used in a single threaded context.
//!
//! The read half of the channel is the "owner" of the backing buffer. When the read half of the channel is dropped,
//! the backing buffer will also be dropped. The write half of the channel will return a broken pipe error for any
//! writes when the read half is no longer alive. However, the read half of the channel will return whatever is left
//! in the buffer if the write half is dropped.

use std::{rc::{Rc, Weak}, collections::VecDeque, cell::RefCell};
use std::io::{Read, Write};

/// Read half of the channel
pub struct ReadChannel {
    buffer: Rc<RefCell<VecDeque<u8>>>,
}

impl Read for ReadChannel {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut data = self.buffer.borrow_mut();
        let to_read = data.len().min(buf.len());
        for i in 0..to_read {
            buf[i] = data.pop_front().unwrap();
        }
        Ok(to_read)
    }
}

/// Write half of the channel
pub struct WriteChannel {
    buffer: Weak<RefCell<VecDeque<u8>>>,
}

impl Write for WriteChannel {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some(internal) = self.buffer.upgrade() {
            let mut data = internal.borrow_mut();
            data.extend(buf);
            Ok(buf.len())
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Reader no longer exists"))
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(_) = self.buffer.upgrade() {
            Ok(())
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Reader no longer exists"))
        }
    }
}

/// Create new read and write channels
pub fn new_io_channel() -> (ReadChannel, WriteChannel) {
    let buffer = Rc::new(RefCell::new(VecDeque::new()));
    let write = WriteChannel {
        buffer: Rc::downgrade(&buffer),
    };
    let read = ReadChannel {
        buffer,
    };
    (read, write)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basic_read_write() {
        let mut buf: [u8; 5] = [0; 5];
        let (mut read, mut write) = new_io_channel();
        write.write(&[1,2,3,4,5]).unwrap();
        let res = read.read(&mut buf);
        if let Ok(s) = res {
            assert_eq!(s, 5);
        } else {
            assert!(false);
        }
        assert_eq!(buf, [1,2,3,4,5]);
    }

    #[test]
    fn read_dropped() {
        let (read, mut write) = new_io_channel();
        drop(read);
        let res = write.write(&[1,2,3,4,5]);
        if let Err(e) = res {
            assert_eq!(e.kind(), std::io::ErrorKind::BrokenPipe);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn write_dropped() {
        let mut buf: [u8; 5] = [0; 5];
        let (mut read, write) = new_io_channel();
        drop(write);
        let res = read.read(&mut buf);
        if let Ok(s) = res {
            assert_eq!(s, 0);
        } else {
            assert!(false);
        }
    }
}