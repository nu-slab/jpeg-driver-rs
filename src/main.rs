mod uio;
mod udma;
mod axidma;
mod vfrmbuf;
mod jpeg_encoder;

use uio::Uio;
use udma::Udma;
use axidma::Adma;
use vfrmbuf::Vfb;
use jpeg_encoder::JpegEncoder;



use libc::{open, close, mmap, munmap, O_RDWR, PROT_READ, PROT_WRITE, MAP_SHARED};
use std::ffi::CString;
//use std::fs::File;
use std::io::Read;
use std::ptr;
use std::os::unix::io::RawFd;
use serde::{Serialize, Deserialize};

use std::fs::File;
use std::io::{self, BufRead, BufReader,Write};
use std::path::Path;
use std::time::Instant;
use anyhow::{anyhow, Result, Context};

const PAGE_SIZE: usize = 0x1000;

fn main() -> Result<()> {

    // open u-dma-buffer
    // let mut buf_vfrmbuf =  Udma::new("udmabuf_v_frmbuf_rd_0_0").unwrap();
    // let mut buf_adma     =  Udma::new("udmabuf_axi_dma_0_0").unwrap();

    // //open Vfrmbuf driver
    // let vfrmbuf_rd = Vfb::new("v_frmbuf_rd_0").unwrap();

    // //open Axi dma driver
    // let adma = Adma::new("axi_dma_0").unwrap(); 


    // vfrmbuf_rd.set_phys_addr(buf_vfrmbuf);
    // vfrmbuf_rd.set_format(1280,720);


    // adma.s2mm_reset();
    // println!("is:{} ,read:{}",adma.is_idle(),adma.read_idle());
    // adma.set_s2mm_addr(buf_adma.phys_addr);



    // let data = read_dat_file("input.dat").unwrap();


    // buf_vfrmbuf.write_to_buf(&data).unwrap();

    // println!("is:{} ,read:{}",adma.is_idle(),adma.read_idle());
    // let start_time = Instant::now();

    // adma.s2mm_start();
    // vfrmbuf_rd.start();
    // println!("addr:{},length{}",adma.read_s2mm_addr(),adma.read_s2mm_length());
    // println!("is:{} ,read:{}",adma.is_idle(),adma.read_idle());
    // adma.set_s2mm_length(0x200000);
    // println!("addr:{},length{}",adma.read_s2mm_addr(),adma.read_s2mm_length());
    // println!("{}",adma.read_status());
    // println!("{}",adma.is_idle());
    // while !adma.is_idle() {
    // }

    // println!("is:{} ,read:{}",adma.is_idle(),adma.read_idle());
    // let elapsed_time = start_time.elapsed();
    // println!("Transfer completed in {:.3}ms", elapsed_time.as_secs_f64() * 1000.0);

    // //buf_adma.write_jpeg("output.jpg").unwrap();
    // buf_adma.write_jpeg_to_file("output.jpg").unwrap();

    // println!("num:{}",data.len());
    // let mut file = File::create("output.bin").unwrap();
    // file.write(&data).unwrap();

    // vfrmbuf_rd.stop();
    // vfrmbuf_rd.close();
    // adma.close();
    // buf_vfrmbuf.close();
    // buf_adma.close()
    

    let mut driver = JpegEncoder::new("./hwinfo.json")?;

    let data = read_dat_file("input.dat").unwrap();

    //driver.vfrmbuf.buf.write_to_buf(&data).unwrap();
    //buf_vfrmbuf.write_to_buf(&data).unwrap();
    
    driver.config();
    driver.encode(&data);
    

    let filename = "/dev/uio5".to_string();
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


    unsafe {
        let ptr = mem.add(0x04) as * const u32;
        let hoge = ptr::read_volatile(ptr);
        //println!("{:x}",hoge);
        let out = driver.adma.buf.read_from_buf(hoge as usize).unwrap();
        //let out :Vec<u8> = vec![10;10];

        println!("Buffer size:{}",out.len());
        let mut F = File::create("output.jpg")?;
        
        F.write_all(&out)?;
        //F.flush()?;
    }
    
    
    //driver.adma.buf.write_jpeg_to_file("output.jpg").unwrap();

    
    
    
    Ok(())
}

// fn write_jpeg_to_file(data: &[u8], filename: &str) -> io::Result<()> {
//     let mut file = File::create(filename)?;
//     let mut bytes_written = 0;
//     let mut count = 0;
//     let size = data.len();

//     while bytes_written < size {
//         // ブロック単位で書き込む (ここでは1バイトごとに調整)
//         let bytes_to_write = 1.min(size - bytes_written);

//         // データを書き込み
//         let written = file.write(&data[bytes_written..bytes_written + bytes_to_write])?;
//         if written != bytes_to_write {
//             return Err(io::Error::new(
//                 io::ErrorKind::WriteZero,
//                 "Error writing to file",
//             ));
//         }

//         bytes_written += written;
//         count += 1;

//         // JPEGの終了マーカー (0xFF, 0xD9) を確認
//         if bytes_written + 1 < size && data[bytes_written] == 0xFF && data[bytes_written + 1] == 0xD9 {
//             break; // マーカーが見つかったら書き込みを停止
//         }
//     }

//     Ok(())
// }


fn read_dat_file(filename: &str) -> io::Result<Vec<u8>> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let mut buf = Vec::new();

    let mut count = 0;
    // ファイルを1行ずつ読み込み、RGBの値をVecに追加
    for line in reader.lines() {
        let line = line?;
        let values: Vec<&str> = line.split(',').collect();

        
        if values.len() == 3 {
            let r: u8 = values[0].trim().parse().unwrap_or(0);
            let b: u8 = values[1].trim().parse().unwrap_or(0);
            let g: u8 = values[2].trim().parse().unwrap_or(0);

            buf.push(r);
            buf.push(g);
            buf.push(b);
        }
        count = count + 1;
    }

    Ok(buf)
}


