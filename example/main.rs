mod uio;
mod udma;
mod axidma;
mod vfrmbuf;
mod jpeg_encoder;

// use uio::Uio;
// use udma::Udma;
// use axidma::Adma;
// use vfrmbuf::Vfb;
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

    

    let mut driver = JpegEncoder::new("./hwinfo.json")?;

    let data = read_dat_file("input.dat").unwrap();

    
    driver.config();

    
    let start_time = Instant::now();
    
    driver.encode(&data);
    //driver.encode_file(&data,"output.jpg")?;
    
    let elapsed_time = start_time.elapsed();
    println!("Transfer completed in {:.3}ms", elapsed_time.as_secs_f64() * 1000.0); 
    
    Ok(())
}



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


