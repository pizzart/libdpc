use std::fs::File;
use std::io::{Error, ErrorKind, Write};
use std::path::Path;
use std::vec;

use binwrite::BinWrite;
use nom_derive::*;
use serde::{Deserialize, Serialize};

use crate::walle_fmt::common::{FixedVec, HasReferences, WALLEObjectFormatTrait};
use ddsfile::{D3DFormat, Dds};

#[derive(BinWrite)]
#[binwrite(little)]
#[derive(Serialize, Deserialize, NomLE)]
#[nom(Exact)]
struct BitmapZHeader {
    friendly_name_crc32: u32,
    link_count: u32,
    links: FixedVec<u8, 5>,
}

impl HasReferences for BitmapZHeader {
    fn hard_links(&self) -> Vec<u32> {
        vec![]
    }

    fn soft_links(&self) -> Vec<u32> {
        vec![]
    }
}

#[derive(BinWrite)]
#[binwrite(little)]
#[derive(Serialize, Deserialize, NomLE)]
#[nom(Exact)]
struct BitmapZ {
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    width: u32,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    height: u32,
    precalc_size: u32,
    flag: u16,
    format: u8,
    mipmap_count: u8,
    four: u8,
    // #[nom(Count = "128")]
    // dds_header: Vec<u8>,
    #[nom(Count = "i.len()")]
    data: Vec<u8>,
}

impl HasReferences for BitmapZ {
    fn hard_links(&self) -> Vec<u32> {
        vec![]
    }

    fn soft_links(&self) -> Vec<u32> {
        vec![]
    }
}

#[derive(Serialize, Deserialize)]
struct BitmapObject {
    bitmap_header: BitmapZHeader,
    bitmap: BitmapZ,
}

impl HasReferences for BitmapObject {
    fn hard_links(&self) -> Vec<u32> {
        vec![]
    }

    fn soft_links(&self) -> Vec<u32> {
        vec![]
    }
}

pub struct BitmapObjectFormat;

impl BitmapObjectFormat {
    pub fn new<'a>() -> &'a Self {
        &Self {}
    }
}

impl WALLEObjectFormatTrait for BitmapObjectFormat {
    fn pack(
        self: &Self,
        input_path: &Path,
        header: &mut Vec<u8>,
        body: &mut Vec<u8>,
    ) -> Result<(Vec<u32>, Vec<u32>), Error> {
        let json_path = input_path.join("object.json");
        let json_file = File::open(json_path)?;

        let mut object: BitmapObject = serde_json::from_reader(json_file)?;

        object.bitmap_header.write(header)?;

        let dds_path = input_path.join("data.dds");
        let mut dds_file = File::open(dds_path)?;

        let dds = Dds::read(&mut dds_file).unwrap();

        object.bitmap.width = dds.get_width();
        object.bitmap.height = dds.get_height();

        // object.bitmap.dds_header.clear();
        object.bitmap.data.clear();
        object.bitmap.write(body)?;

        dds.data.write(body).unwrap();

        Ok((
            object.bitmap_header.hard_links(),
            object.bitmap_header.soft_links(),
        ))
    }

    fn unpack(
        self: &Self,
        header: &[u8],
        body: &[u8],
        output_path: &Path,
    ) -> Result<(Vec<u32>, Vec<u32>), Error> {
        let json_path = output_path.join("object.json");
        let mut output_file = File::create(json_path)?;

        let bitmap_header = match BitmapZHeader::parse(&header) {
            Ok((_, h)) => h,
            Err(_) => return Err(Error::from(ErrorKind::Other)),
        };

        let bitmap = match BitmapZ::parse(body) {
            Ok((_, h)) => h,
            Err(_) => return Err(Error::from(ErrorKind::Other)),
        };

        let dds_path = output_path.join("data.dds");
        let mut output_dds_file = File::create(dds_path)?;

        let mut dds = Dds::new_d3d(
            bitmap.height,
            bitmap.width,
            None,
            if bitmap.data[87] == 49 {
                D3DFormat::DXT1
            } else {
                D3DFormat::DXT5
            },
            Some(0),
            None,
        )
        .unwrap();

        dds.data = bitmap.data[128..].to_vec();

        dds.write(&mut output_dds_file).unwrap();

        let object = BitmapObject {
            bitmap_header,
            bitmap,
        };

        output_file.write(serde_json::to_string_pretty(&object)?.as_bytes())?;

        Ok((
            object.bitmap_header.hard_links(),
            object.bitmap_header.soft_links(),
        ))
    }
}
