use std::io;
use binrw::BinRead;
use crate::error::XivError;

fn export_r8(width: u16, height: u16, data: &[u8]) -> Result<image::GrayImage, XivError> {
    let mut result = image::GrayImage::new(width as u32, height as u32);

    let mut cursor = io::Cursor::new(data);
    for y in 0..height {
        for x in 0..width {
            let l = u8::read_le(&mut cursor).map_err(|_| XivError::TexData)?;
            result.put_pixel(x as u32, y as u32, [l].into());
        }
    }
    Ok(result.into())
}

fn export_b5g5r5a1(width: u16, height: u16, data: &[u8]) -> Result<image::RgbaImage, XivError> {
    let mut result = image::RgbaImage::new(width as u32, height as u32);

    let mut cursor = io::Cursor::new(data);
    for y in 0..height {
        for x in 0..width {
            let p = u16::read_le(&mut cursor).map_err(|_| XivError::TexData)?;

            let b = ((p & 0b11111) * 8) as u8;
            let g = ((p >> 5 & 0b11111) * 8) as u8;
            let r = ((p >> 10 & 0b11111) * 8) as u8;
            let a = ((p >> 15 & 1) * 255) as u8;

            result.put_pixel(x as u32, y as u32, [r, g, b, a].into());
        }
    }
    Ok(result.into())
}

fn export_r8g8b8a8(width: u16, height: u16, data: &[u8]) -> Result<image::RgbaImage, XivError> {
    let mut result = image::RgbaImage::new(width as u32, height as u32);

    let mut cursor = io::Cursor::new(data);
    for y in 0..height {
        for x in 0..width {
            let r = u8::read_le(&mut cursor).map_err(|_| XivError::TexData)?;
            let g = u8::read_le(&mut cursor).map_err(|_| XivError::TexData)?;
            let b = u8::read_le(&mut cursor).map_err(|_| XivError::TexData)?;
            let a = u8::read_le(&mut cursor).map_err(|_| XivError::TexData)?;

            result.put_pixel(x as u32, y as u32, [r, g, b, a].into());
        }
    }
    Ok(result.into())
}

fn export_bc(fmt: texpresso::Format, width: u16, height: u16, data: &[u8]) -> Result<image::RgbaImage, XivError> {
    let mut decoded = vec![0u8; width as usize * height as usize * 4];
    fmt.decompress(&data, width as usize, height as usize, &mut decoded);
    export_r8g8b8a8(width, height, &decoded)
}

fn export_dxt1(width: u16, height: u16, data: &[u8]) -> Result<image::RgbaImage, XivError> {
    export_bc(texpresso::Format::Bc1, width, height, data)
}

fn export_dxt3(width: u16, height: u16, data: &[u8]) -> Result<image::RgbaImage, XivError> {
    export_bc(texpresso::Format::Bc2, width, height, data)
}

fn export_dxt5(width: u16, height: u16, data: &[u8]) -> Result<image::RgbaImage, XivError> {
    export_bc(texpresso::Format::Bc3, width, height, data)
}

pub type ImageData = Box<[u8]>;

pub struct Image {
    pub format: u32,
    pub width: u16,
    pub height: u16,
    pub layers: u16,
    pub count: u16,
    pub mipmaps: Box<[ImageData]>,
}

impl Image {
    pub fn export(&self) -> Result<image::DynamicImage, XivError> {
        const L8: u32 = 4400;
        const A8: u32 = 4401;
        // const A4R4G4B4: u32 = 5184;
        const B5G5R5A1: u32 = 5185;
        const R8G8B8A8: u32 = 5200;
        // const X8R8G8B8: u32 = 5201;
        // const R32F: u32 = 8528;
        // const G16R16F: u32 = 8784;
        // const G32R32F: u32 = 8800;
        // const A16B16G16R16F: u32 = 9312;
        // const A32B32G32R32F: u32 = 9328;
        const DXT1: u32 = 13344;
        const DXT3: u32 = 13360;
        const DXT5: u32 = 13361;
        // const D16: u32 = 16704;

        let w = self.width;
        let h = self.height;
        let data = self.mipmaps.first().ok_or(XivError::TexData)?;

        match self.format {
            L8 | A8 => export_r8(w, h, &data).map(From::from),
            B5G5R5A1 => export_b5g5r5a1(w, h, &data).map(From::from),
            R8G8B8A8 => export_r8g8b8a8(w, h, &data).map(From::from),
            DXT1 => export_dxt1(w, h, &data).map(From::from),
            DXT3 => export_dxt3(w, h, &data).map(From::from),
            DXT5 => export_dxt5(w, h, &data).map(From::from),
            _ => Err(XivError::TexFormat(self.format)),
        }
    }
}
