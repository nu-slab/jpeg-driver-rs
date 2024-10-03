use libc::{open, read, close, mmap, munmap, O_RDWR, O_SYNC, PROT_READ, PROT_WRITE, MAP_SHARED};
use xipdriver_rs::axigpio::AxiGpio;
use std::ffi::CString;
use std::fs::File;
use std::io::{self, Read,Write};
use std::ptr;
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Arc,Mutex};
use anyhow::{anyhow, Result, Context};
use std::path::Path;
use log::info;

//use std::sync::atomic::{AtomicPtr, Ordering};

//u-dma-bufのOwner
#[derive(PartialEq,Clone,Copy)]
pub enum Owner{
    Cpu = 0,
    Device = 1
}

// From<u32>を実装してu32からOwnerへの変換を可能にする
impl From<u32> for Owner {
    fn from(value: u32) -> Self {
        match value {
            0 => Owner::Cpu,
            1 => Owner::Device,
            _ => panic!("Unknown owner value"), // ここでエラーハンドリングをすることも可能
        }
    }
}


pub struct Udma {
    pub name: String,
    pub fd: RawFd,
    //pub buf: *mut u8,
    pub buf: Arc<Mutex<AtomicPtr<u8>>>,  // Arc<Mutex<AtomicPtr<u32>>>でスレッド間共有を可能に
    pub phys_addr: u32,
    pub size: usize,

    pub sync_direction: File,
    pub sync_for_cpu: File,
    pub sync_for_device: File,
}

impl Udma {
    pub fn new(buf_name:&str) -> Result<Self>{
        Udma::open(buf_name)
    }
    
    pub fn open(buf_name: &str) -> Result<Self> {
        info!("{}",buf_name);
        let phys_addr = Udma::get_phys_addr(buf_name)?;
        let size = Udma::get_udma_size(buf_name)?;
        
        let filename = format!("/dev/{}", buf_name);
        let c_filename = CString::new(filename).unwrap();

        //let fd = unsafe { open(c_filename.as_ptr(), O_RDWR | O_SYNC) };
        let fd = unsafe { open(c_filename.as_ptr(), O_RDWR) };
        if fd < 0 {
            return Err(anyhow::Error::from(std::io::Error::last_os_error()));
       } 
        
        let buf = unsafe {
            mmap(
                ptr::null_mut(),
                size,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                fd,
                0,
            )
        };

        if buf == libc::MAP_FAILED {
            unsafe { close(fd) };
            return Err(anyhow::Error::from(std::io::Error::last_os_error()));
        }

        //手動でのキャッシュ制御のためのファイル
        
        let direction_name = format!("/sys/class/u-dma-buf/{}/sync_direction", buf_name);
        let direction_path = Path::new(&direction_name);
        let mut sync_direction = File::create(direction_path)?;

        // let mut sync_direction = match File::create(direction_path)
        // {
        //     Ok(name) => name,  // 成功時はそのまま返す
        //     Err(e) => {
        //     info!("Error occurred: {}", e);  // エラー時にログを出力
        //     return Err(anyhow::Error::new(e));  // std::io::Errorをanyhow::Errorに変換して返す
        //     }
        // };

        let sfc_name = format!("/sys/class/u-dma-buf/{}/sync_for_cpu", buf_name);
        let sfc_path = Path::new(&sfc_name);
        let mut sync_for_cpu = File::create(sfc_path)?;
        
        let sfd_name = format!("/sys/class/u-dma-buf/{}/sync_for_device", buf_name);
        let sfd_path = Path::new(&sfd_name);
        let mut sync_for_device = File::create(sfd_path)?;
        
        Ok(Udma {
            name: buf_name.to_string(),
            fd,
            //buf: buf as *mut u8,
            buf:Arc::new(Mutex::new(AtomicPtr::new(buf as *mut u8))),
            phys_addr,
            size,
            sync_direction,
            sync_for_cpu,
            sync_for_device,
        })
    }

    pub fn close(&self) {
        let buf= self.buf.lock().unwrap();
        unsafe {
            munmap(buf.load(Ordering::SeqCst) as *mut libc::c_void, self.size);
            close(self.fd);
        }
    }

    fn get_phys_addr(buf_name: &str) -> io::Result<u32> {
        let filename = format!("/sys/class/u-dma-buf/{}/phys_addr", buf_name);
        let mut file = File::open(filename)?;
        let mut attr = String::new();
        file.read_to_string(&mut attr)?;

        //0xを取り除く
        let trimmed_attr = attr.trim().trim_start_matches("0x");
        
        let phys_addr = u32::from_str_radix(trimmed_attr, 16).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "Failed to parse physical address")
        })?;

        println!("phys_addr: {}", phys_addr);
        Ok(phys_addr)
    }

    fn get_udma_size(buf_name: &str) -> io::Result<usize> {
        let filename = format!("/sys/class/u-dma-buf/{}/size", buf_name);
        let mut file = File::open(filename)?;
        let mut attr = String::new();
        file.read_to_string(&mut attr)?;

        let size = attr.trim().parse::<usize>().map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "Failed to parse size")
        })?;

        Ok(size)
    }

    //現在のバッファのオーナーを読み取り
    fn get_owner(&mut self) -> Result<Owner>{
        let so_name = format!("/sys/class/u-dma-buf/{}/sync_owner", self.name);
        let so_path = Path::new(&so_name);
        let mut sync_owner = File::open(so_path)?;
        
        let mut attr = String::new();
        sync_owner.read_to_string(&mut attr).context("Failed to read sync_owner file")?;

        let owner_value = attr.trim().parse::<u32>().context("Failed to parse owner as u32")?;
        let owner = Owner::from(owner_value);
        
        Ok(owner)
    }
 

    //バッファのオーナーを変更
    pub fn change_owner(&mut self,owner:Owner) -> Result<()>{

        // //すでにオーナーだったらエラー
        // if owner == self.get_owner()?{
        //     return Err(anyhow::Error::msg("Already an owner"));
        // }
//        let ow = self.get_owner()?;
        if owner == Owner::Device {
            write!(self.sync_for_device,"{}",1 as u8)
                .context("Failed to write to sync_for_device")?;
        }
        else{
            write!(self.sync_for_cpu,"{}",1 as u8)
                .context("Failed to write to sync_for_cpu")?;
        }
        let ow = self.get_owner()?;
        //オーナーが変わっているか確認
        if owner != ow{
            return Err(anyhow::Error::msg("Failed to change owner"));
        }
        Ok(())
    }
    


    pub fn write_to_buf(&mut self, data: &[u8]) -> Result<()> {
        let data_len = data.len();
        if data_len > self.size {            
            return Err(anyhow::Error::msg("Data size exceeds buffer size"));
        }

        //ownerをCPUにする
        self.change_owner(Owner::Cpu)?;

        let buf = self.buf.lock().unwrap();

        unsafe {
            // `copy_nonoverlapping` を使ってデータをコピー
            ptr::copy_nonoverlapping(data.as_ptr(), buf.load(Ordering::SeqCst), data_len);
        }

        Ok(())
    }


    pub fn read_from_buf(&mut self, len: usize) -> Result<Vec<u8>> {        
        if len > self.size {
            return Err(anyhow::Error::msg("Data size exceeds buffer size"));
        }

        let mut data = vec![0u8; len];

        //ownerをCPUにする
        //self.get_owner()?;
        self.change_owner(Owner::Cpu)?;

        let buf = self.buf.lock().unwrap();
        
        unsafe {
            // `copy_nonoverlapping` を使って `buf` からデータを読み出す
            ptr::copy_nonoverlapping(buf.load(Ordering::SeqCst), data.as_mut_ptr(), len);
        }

        Ok(data)
    }





}


