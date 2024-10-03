use crate::uio::Uio;
use crate::axidma::Adma;
use crate::vfrmbuf::Vfb;
use xipdriver_rs::json_as_map;
use xipdriver_rs::json_as_str;
use xipdriver_rs::json_as_u32;
use anyhow::{anyhow, Result, Context};
use log::info;
use std::fs::File;
use std::io::Write;

const PAGE_SIZE:usize = 0x1000;

pub struct JpegEncoder{
    pub uio:Uio,
    pub vfrmbuf:Vfb,
    pub adma:Adma
    
    // buf_vfrmbuf:Udma,
    // buf_adma:Udma
}

impl JpegEncoder{
    pub fn new(hw_json_path:&str) -> Result<Self>{
        //ハードウェア情報の読み込み
        let hw_json = xipdriver_rs::hwinfo::read(hw_json_path)?;

        let jpeg_hier = "jpeg_encoder";

        //ハードウェア名を取得
        let jpeg_uio_name = xipdriver_rs::hwinfo::match_hw(
            &hw_json,
            jpeg_hier,
            "jpeg_encoder"
        )?;        
        
        let uio_obj = json_as_map!(hw_json[jpeg_uio_name]);
        let uio_name = json_as_str!(uio_obj["uio"]);     
        
        let jpeg_vfbr_name = xipdriver_rs::hwinfo::match_hw(
            &hw_json,
            jpeg_hier,
            "v_frmbuf_rd"
        )?;
        
        let jpeg_dma_name = xipdriver_rs::hwinfo::match_hw(
            &hw_json,
            jpeg_hier,
            "axi_dma"
        )?;
        
        //uioをオープン
        let mut uio = Uio::new(&uio_name,PAGE_SIZE)?;

        //video frame buffer をオープン
        let mut vfrmbuf =  Vfb::new(&hw_json[jpeg_vfbr_name])?;

        //AXI DMAをオープン
        let mut adma = Adma::new(&hw_json[jpeg_dma_name])?;

        Ok(JpegEncoder{
            uio,
            vfrmbuf,
            adma
        })
            
    }

    pub fn config(&mut self){
        self.vfrmbuf.set_phys_addr();
        self.vfrmbuf.set_format(1280,720);

        self.adma.s2mm_reset();
        self.adma.set_s2mm_addr();
    }

    pub fn encode(&mut self,img_data: &[u8]) -> Result<Vec<u8>>{
        //self.vfrmbuf.buf.write_to_buf(&img_data).unwrap();

        //dma をスタート
        self.adma.start()?;
        //画像データ書き込み開始
        self.vfrmbuf.start(&img_data)?;
        //エンコードデータ読み込みスタート
        self.adma.set_s2mm_length(0x200000);

        //完了するまで待ち
        while !self.adma.is_idle(){}

        //エンコードデータのサイズを取得
        let len = self.uio.read_mem32(0x04) as usize;        
        
        
        Ok(self.adma.buf.read_from_buf(len).unwrap())

    }


    pub fn encode_file(&mut self,img_data: &[u8],o_file_name:&str)->Result<()>{

        //dma をスタート
        self.adma.start()?;
        //画像データ書き込み開始
        self.vfrmbuf.start(&img_data)?;
        //エンコードデータ読み込みスタート
        self.adma.set_s2mm_length(0x200000);

        //完了するまで待ち
        while !self.adma.is_idle(){}

        //エンコードデータのサイズを取得
        let len = self.uio.read_mem32(0x04) as usize;        
        let out = self.adma.buf.read_from_buf(len).unwrap();

        //ファイル出力
        let mut file=File::create(o_file_name).context("Failed open jpeg file")?;       
        file.write_all(&out).context("Failed output jpeg file")?;

        Ok(())
        

    }

}
