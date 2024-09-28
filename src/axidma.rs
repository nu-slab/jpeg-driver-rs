use libc::{open, read, close, mmap, munmap, O_RDWR, PROT_READ, PROT_WRITE, MAP_SHARED};
use std::ffi::CString;
use std::fs::{DirEntry};
use std::io::{self, Read};
use std::ptr;
use std::os::unix::io::RawFd;
use serde::{Serialize, Deserialize};
use anyhow::{anyhow, Result, Context};

use crate::udma::{Udma,Owner};

use xipdriver_rs::json_as_map;
use xipdriver_rs::json_as_str;
use xipdriver_rs::json_as_u32;


const PAGE_SIZE: usize = 0x1000;

const MM2S_DMACR: usize = 0x00;
const MM2S_DMASR: usize = 0x04;
const MM2S_SA: usize = 0x18;
const MM2S_SA_MSB: usize = 0x1C;
const MM2S_LENGTH: usize = 0x28;
const S2MM_DMACR: usize = 0x30;
const S2MM_DMASR: usize = 0x34;
const S2MM_DA: usize = 0x48;
const S2MM_DA_MSB: usize = 0x4C;
const S2MM_LENGTH: usize = 0x58;

pub struct Adma {
    fd: RawFd,
    mem: *mut u32,
    pub buf: Udma
}

impl Adma {
    pub fn new(hw_info: &serde_json::Value) -> Result<Self> {
        //vfrmbufのハードウェア情報を取得
        let hw_object = json_as_map!(hw_info);
        let uio_name = json_as_str!(hw_object["uio"]);
        let udmabuf_name = json_as_str!(hw_object["udmabuf"][0]);

        //uioをオープン
        let dev_name = Adma::check_axi_dma_uio_num(uio_name)?;
        let filename = format!("/dev/{}", dev_name);
        let c_filename = CString::new(filename).unwrap();

        let fd = unsafe { open(c_filename.as_ptr(), O_RDWR) };
        if fd < 0 {
            return Err(anyhow::Error::from(std::io::Error::last_os_error()));
        }

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

        //u-dma-bufferをオープン
        let mut udmabuf = Udma::new(udmabuf_name)?;

        Ok(Adma {
            fd,
            mem: mem as *mut u32,
            buf: udmabuf
        })
        
        //Adma::open(name)
    }
    
    // pub fn open(name: &str) -> io::Result<Self> {
    //     let dev_name = Adma::check_axi_dma_uio_num(name)?;
    //     let filename = format!("/dev/{}", dev_name);
    //     let c_filename = CString::new(filename).unwrap();

    //     let fd = unsafe { open(c_filename.as_ptr(), O_RDWR) };
    //     if fd < 0 {
    //         return Err(io::Error::last_os_error());
    //     }

    //     let mem = unsafe {
    //         mmap(
    //             ptr::null_mut(),
    //             PAGE_SIZE,
    //             PROT_READ | PROT_WRITE,
    //             MAP_SHARED,
    //             fd,
    //             0,
    //         )
    //     };

    //     if mem == libc::MAP_FAILED {
    //         unsafe { close(fd) };
    //         return Err(io::Error::last_os_error());
    //     }

    //     Ok(Adma { fd, mem: mem as *mut u32 })
    // }

    pub fn close(&self) {
        unsafe {
            munmap(self.mem as *mut libc::c_void, PAGE_SIZE);
            close(self.fd);
        }
    }

    pub fn write_mem32(&self, addr: usize, val: u32) {
        unsafe {
            ptr::write_volatile(self.mem.add(addr / 4), val);
        }
    }

    pub fn read_mem32(&self, addr: usize) -> u32 {
        unsafe { ptr::read_volatile(self.mem.add(addr / 4)) }
    }

    fn check_axi_dma_uio_num(name: &str) -> io::Result<String> {
        let dir = std::fs::read_dir("/sys/class/uio")?;

        for entry in dir {
            let entry: DirEntry = entry?;
            if let Some(filename) = entry.file_name().to_str() {
                if filename != "." && filename != ".." {
                    let path = format!("/sys/class/uio/{}/name", filename);
                    let mut attr = String::new();
                    std::fs::File::open(path)?.read_to_string(&mut attr)?;

                    if attr.trim() == name {
                        return Ok(filename.to_string());
                    }
                }
            }
        }
        Err(io::Error::new(io::ErrorKind::NotFound, "no uio device found"))
    }

    pub fn mm2s_reset(&self) {
        let value = self.read_mem32(MM2S_DMACR) | 0x4;
        self.write_mem32(MM2S_DMACR, value);

        let value = self.read_mem32(MM2S_DMACR) & 0xFFFFFFFB;
        self.write_mem32(MM2S_DMACR, value);
    }

    pub fn set_mm2s_addr(&self, addr: u32) {
        self.write_mem32(MM2S_SA, addr);
    }

    pub fn mm2s_start(&self) {
        let value = self.read_mem32(MM2S_DMACR) | 0x1;
        self.write_mem32(MM2S_DMACR, value);
    }

    pub fn set_mm2s_length(&self, length: u32) {
        self.write_mem32(MM2S_LENGTH, length);
    }

    pub fn s2mm_reset(&self) {
        let value = self.read_mem32(S2MM_DMACR) | 0x4;
        self.write_mem32(S2MM_DMACR, value);

        let value = self.read_mem32(S2MM_DMACR) & 0xFFFFFFFB;
        self.write_mem32(S2MM_DMACR, value);
    }

    pub fn set_s2mm_addr(&self) {
        self.write_mem32(S2MM_DA, self.buf.phys_addr);
    }

    pub fn s2mm_start(&self) {
        let value = self.read_mem32(S2MM_DMACR) | 0x1;
        self.write_mem32(S2MM_DMACR, value);
    }

    pub fn set_s2mm_length(&self, length: u32) {
        self.write_mem32(S2MM_LENGTH, length);
    }

    pub fn read_ctrl(&self) -> u32 {
        self.read_mem32(S2MM_DMACR)
    }

    pub fn read_status(&self) -> u32 {
        self.read_mem32(S2MM_DMASR)
    }

    pub fn read_s2mm_length(&self) -> u32 {
        self.read_mem32(S2MM_LENGTH)
    }

    pub fn is_idle(&self) -> bool {
        (self.read_mem32(S2MM_DMASR) & 0x2) == 2
    }

    pub fn read_idle(&self) -> u32 {
        (self.read_mem32(S2MM_DMASR) & 0x2) 
    }

    pub fn read_s2mm_addr(&self) -> u32 {
        self.read_mem32(S2MM_DA)
    }


    pub fn start(&mut self) -> Result<()>{
        print!("AXIDMa:");
        self.buf.change_owner(Owner::Device)?;
        print!("AXIDMa:");
        self.s2mm_start();
        Ok(())
    }
}
