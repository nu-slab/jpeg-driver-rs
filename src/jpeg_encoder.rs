use crate::axidma::Adma;
use crate::vfrmbuf::Vfb;
use anyhow::{anyhow, Result, Context};

pub struct JpegEncoder{
    pub vfrmbuf:Vfb,
    pub adma:Adma
    
    // buf_vfrmbuf:Udma,
    // buf_adma:Udma
}

impl JpegEncoder{
    pub fn new(hw_json_path:&str) -> Result<Self>{
        //ハードウェア情報の読み込み
        println!("hw json");
        let hw_json = xipdriver_rs::hwinfo::read(hw_json_path)?;

        let jpeg_hier = "jpeg_encoder";

        //ハードウェア名を取得
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
        println!("vfrmbuf");
        //video frame buffer をオープン
        let mut vfrmbuf =  Vfb::new(&hw_json[jpeg_vfbr_name])?;
        println!("AXI DMA");
        //AXI DMAをオープン
        let mut adma = Adma::new(&hw_json[jpeg_dma_name])?;

        Ok(JpegEncoder{
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

        self.adma.start()?;
        self.vfrmbuf.start(&img_data)?;
        self.adma.set_s2mm_length(0x200000);

        while !self.adma.is_idle(){}

        
        Ok(self.adma.buf.read_from_buf(0x200000).unwrap())

    }

    // pub fn encode_file(&mut self,img_data: &[u8]){
    //     self.vfrmbuf.buf.write_to_buf(&img_data).unwrap();

    //     self.adma.s2mm_start();
    //     self.vfrmbuf.start();
    //     self.adma.set_s2mm_length(0x200000);

    //     while !self.adma.is_idle(){}

    //     let out = self.adma.buf.buf.clone();

    // }
}
