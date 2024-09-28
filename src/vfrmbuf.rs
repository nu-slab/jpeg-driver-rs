use libc::{open, close, mmap, munmap, O_RDWR, PROT_READ, PROT_WRITE, MAP_SHARED};
use std::ffi::CString;
use std::fs::File;
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

// レジスタオフセット定義
const FRMBUF_CTRL: usize = 0x0000;
const FRMBUF_WIDTH: usize = 0x0010;
const FRMBUF_HEIGHT: usize = 0x0018;
const FRMBUF_STRIDE: usize = 0x0020;
const FRMBUF_FORMAT: usize = 0x0028;
const FRMBUF_P1BUFFER: usize = 0x0030;

// vfb_t 構造体のRust版
pub struct Vfb {
    fd: RawFd,
    mem: *mut u32,
    pub buf:Udma
}

impl Vfb {

    pub fn new(hw_info: &serde_json::Value) -> Result<Self>{
        //vfrmbufのハードウェア情報を取得
        let hw_object = json_as_map!(hw_info);
        let uio_name = json_as_str!(hw_object["uio"]);
        let udmabuf_name = json_as_str!(hw_object["udmabuf"][0]);

        //uioをオープン
        let dev_name = Vfb::check_vfrmbuf_uio_num(uio_name)?;
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
        
        Ok(Vfb {
            fd,
            mem: mem as *mut u32,
            buf: udmabuf
        })

        
        //Vfb::open(name)
    }
    /// デバイスをオープンしてメモリをマッピング
    // pub fn open(name: &str) -> io::Result<Self> {
    //     let dev_name = Vfb::check_vfrmbuf_uio_num(name)?;
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

    //     Ok(Vfb {
    //         fd,
    //         mem: mem as *mut u32,
    //     })
    // }

    /// メモリとファイルディスクリプタをクローズ
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

    /// デバイスの名前を探す
    fn check_vfrmbuf_uio_num(name: &str) -> io::Result<String> {
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

    /// 物理アドレスを設定
    // pub fn set_phys_addr(&self, udmabuf_phys_addr: u32) {
    //     self.write_mem32(FRMBUF_P1BUFFER, udmabuf_phys_addr);
    // }
    pub fn set_phys_addr(&self) {
        self.write_mem32(FRMBUF_P1BUFFER, self.buf.phys_addr);
    }

    /// 画像フォーマットを設定
    pub fn set_format(&self, frame_width: usize, frame_height: usize) {
        let fmd_id = 20; // RGB8
        let mm_width_bytes = 1 * 8;
        let bpp_numerator = 3;
        let bpp_denominator = 1;

        let stride = (((frame_width * bpp_numerator) / bpp_denominator + mm_width_bytes - 1)
            / mm_width_bytes)
            * mm_width_bytes;

        self.write_mem32(FRMBUF_WIDTH, frame_width as u32);
        self.write_mem32(FRMBUF_HEIGHT, frame_height as u32);
        self.write_mem32(FRMBUF_STRIDE, stride as u32);
        self.write_mem32(FRMBUF_FORMAT, fmd_id as u32);
    }

    /// コントロールレジスタを読み込む
    pub fn read_ctrl(&self) -> u32 {
        self.read_mem32(FRMBUF_CTRL)
    }

    /// 幅を読み込む
    pub fn read_width(&self) -> u32 {
        self.read_mem32(FRMBUF_WIDTH)
    }

    /// 高さを読み込む
    pub fn read_height(&self) -> u32 {
        self.read_mem32(FRMBUF_HEIGHT)
    }

    /// 物理アドレスを読み込む
    pub fn read_addr(&self) -> u32 {
        self.read_mem32(FRMBUF_P1BUFFER)
    }

    /// フレームバッファを開始
    pub fn write_start(&self) {
        self.write_mem32(FRMBUF_CTRL, 0x01);
    }

    /// フレームバッファを停止
    pub fn stop(&self) {
        self.write_mem32(FRMBUF_CTRL, 0x00);
    }



    pub fn start(&mut self,img_buffer:&[u8]) -> Result<()>{
        //画像データをバッファに書き込み
        print!("vfrmbuf:");
        self.buf.write_to_buf(&img_buffer)?;

        //ownerをDevice(PL)にする
        print!("vfrmbuf2:");
        self.buf.change_owner(Owner::Device)?;

        self.write_start();

        Ok(())
    }
}
