use libc::{open, close, mmap, munmap, O_RDWR, PROT_READ, PROT_WRITE, MAP_SHARED};
use std::ffi::CString;
use std::fs::File;
use std::io::{self, Read};
use std::ptr;
use std::os::unix::io::RawFd;
use serde::{Serialize, Deserialize};
use anyhow::{anyhow, Result, Context};



pub struct Uio{
    fd:RawFd,
    mem: *mut u32
}


impl Uio {

    pub fn new(dev_name:&str) -> Result<Self>{
        let filename = format!("/dev/{}", dev_name);
        let c_filename = CString::new(filename).unwrap();

        //devファイルをオープン
        let fd = unsafe { open(c_filename.as_ptr(), O_RDWR) };
        if fd < 0 {
            return Err(anyhow::Error::from(std::io::Error::last_os_error()));
        }

        //mmap
        let mem = unsafe {
            mmap(
                ptr::null_mut(),
                PAGE_SIZE,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                fd,
                0,
            )
        };

        if mem == libc::MAP_FAILED {
            unsafe { close(fd) };
            return Err(anyhow::Error::from(std::io::Error::last_os_error()));
        }

        Ok(
            Uio{
                fd,
                mem,
            }
        )
    }

    pub fn close(&self) {
        unsafe {
            munmap(self.mem as *mut libc::c_void, PAGE_SIZE);
            libc::close(self.fd);
        }
    }

    /// メモリに値を書き込み
    fn write_mem32(&self, addr: usize, val: u32) {
        unsafe {
            ptr::write_volatile(self.mem.add(addr / 4), val);
        }
    }

    /// メモリから値を読み取り
    fn read_mem32(&self, addr: usize) -> u32 {
        unsafe { ptr::read_volatile(self.mem.add(addr / 4)) }
    }
}
