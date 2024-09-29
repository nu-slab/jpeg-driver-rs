
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
    page_size:usize,
    mem: *mut u32
}


impl Uio {

    pub fn new(uio_name:&str,page_size:usize) -> Result<Self>{
        let dev_name = Uio::check_uio_num(uio_name)?; 
        
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
                page_size,
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
                page_size,
                mem: mem  as *mut u32
            }
        )
    }

    pub fn close(&self) {
        unsafe {
            munmap(self.mem as *mut libc::c_void, self.page_size);
            libc::close(self.fd);
        }
    }

        /// デバイスの名前を探す
    fn check_uio_num(name: &str) -> io::Result<String> {
        let dir = std::fs::read_dir("/sys/class/uio")?;

        for entry in dir {
            let entry = entry?;
            if let Some(filename) = entry.file_name().to_str() {
                if filename != "." && filename != ".." {
                    let path = format!("/sys/class/uio/{}/name", filename);
                    let mut attr = String::new();
                    File::open(path)?.read_to_string(&mut attr)?;

                    if attr.trim() == name {
                        return Ok(filename.to_string());
                    }
                }
            }
        }

        Err(io::Error::new(io::ErrorKind::NotFound, "no uio device found"))
    }

    /// メモリに値を書き込み
    pub fn write_mem32(&self, addr: usize, val: u32) {
        unsafe {
            ptr::write_volatile(self.mem.add(addr / 4), val);
        }
    }

    /// メモリから値を読み取り
    pub fn read_mem32(&self, addr: usize) -> u32 {
        unsafe { ptr::read_volatile(self.mem.add(addr / 4)) }
    }
}
