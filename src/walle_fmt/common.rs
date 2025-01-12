use std::fs::File;
use std::io::{Error, ErrorKind, Write};
use std::marker::PhantomData;
use std::path::Path;
use std::vec::Vec;

// use ::nom::combinator::map;
use binwrite::{BinWrite, WriterOption};
use nom::multi::length_count;
pub use nom::number::complete::*;
// pub use nom::*;
pub use nom_derive::*;
use num_traits::{cast, NumCast};
pub use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub trait HasReferences {
    fn hard_links(&self) -> Vec<u32>;
    fn soft_links(&self) -> Vec<u32>;
}

#[derive(BinWrite)]
#[binwrite(little)]
#[derive(Serialize, Deserialize, NomLE)]
#[nom(Exact)]
pub struct ResourceObjectZ {
    friendly_name_crc32: u32,
    #[nom(Cond = "i.len() != 0")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[binwrite(with(write_option))]
    pub crc32s: Option<PascalArray<u32>>,
}

impl HasReferences for ResourceObjectZ {
    fn hard_links(&self) -> Vec<u32> {
        vec![]
    }

    fn soft_links(&self) -> Vec<u32> {
        if let Some(crc32s) = &self.crc32s {
            crc32s.data.clone()
        } else {
            vec![]
        }
    }
}

#[derive(BinWrite)]
#[binwrite(little)]
#[derive(PartialEq, NomLE)]
pub struct FixedVec<T: BinWrite, const U: usize> {
    #[nom(Count(U))]
    pub data: Vec<T>,
}

impl<T: BinWrite, const U: usize> Serialize for FixedVec<T, U>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.data.serialize(serializer)
    }
}

impl<'de, T: BinWrite, const U: usize> Deserialize<'de> for FixedVec<T, U>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fv = FixedVec {
            data: Vec::deserialize(deserializer)?,
        };
        if fv.data.len() != U {
            panic!("FixedVec size does not match");
        }
        Ok(fv)
    }
}

pub type Vec2<T> = FixedVec<T, 2>;
pub type Vec2f = Vec2<f32>;

pub type Vec3<T> = FixedVec<T, 3>;
pub type Vec3f = Vec3<f32>;
pub type Vec3i32 = Vec3<i32>;

pub type Vec4<T> = FixedVec<T, 4>;
pub type Vec4f = Vec4<f32>;
pub type Quat = Vec4<f32>;

pub type Mat4f = FixedVec<f32, 16>;

#[derive(BinWrite)]
#[binwrite(little)]
#[derive(PartialEq, Serialize, Deserialize, NomLE)]
pub struct Rect {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

#[derive(BinWrite)]
#[binwrite(little)]
#[derive(PartialEq, Serialize, Deserialize, NomLE)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(BinWrite)]
#[binwrite(little)]
#[derive(PartialEq, Serialize, Deserialize, NomLE)]
pub struct SphereZ {
    pub center: Vec3f,
    pub radius: f32,
}

#[derive(BinWrite)]
#[binwrite(little)]
#[derive(PartialEq, Serialize, Deserialize, NomLE)]
pub struct RangeBeginEnd<T: BinWrite = u16> {
    pub begin: T,
    pub end: T,
}

#[derive(BinWrite)]
#[binwrite(little)]
#[derive(PartialEq, Serialize, Deserialize, NomLE)]
pub struct RangeBeginSize<T: BinWrite = u16> {
    pub begin: T,
    pub size: T,
}

#[derive(BinWrite)]
#[binwrite(little)]
#[derive(PartialEq, Serialize, Deserialize, NomLE)]
pub struct FadeDistances {
    pub x: f32,
    pub y: f32,
    pub fade_close: f32,
}

pub fn write_option<W, T>(
    option: &Option<T>,
    writer: &mut W,
    options: &WriterOption,
) -> Result<(), Error>
where
    W: Write,
    T: BinWrite,
{
    if let Some(value) = option {
        BinWrite::write_options(value, writer, options)
    } else {
        Ok(())
    }
}

#[derive(NomLE)]
pub struct PascalArray<T> {
    #[nom(LengthCount(nom::number::complete::le_u32))]
    pub data: Vec<T>,
}

impl<T> BinWrite for PascalArray<T>
where
    T: BinWrite,
{
    fn write_options<W: Write>(&self, writer: &mut W, options: &WriterOption) -> Result<(), Error> {
        BinWrite::write_options(&(self.data.len() as u32), writer, options)?;
        BinWrite::write_options(&self.data, writer, options)
    }
}

impl<T> PascalArray<T> {
    pub fn len(self: &Self) -> usize {
        self.data.len()
    }
}

impl<T> Serialize for PascalArray<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.data.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for PascalArray<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(PascalArray {
            data: Vec::deserialize(deserializer)?,
        })
    }
}

#[derive(NomLE)]
pub struct PascalString {
    #[nom(
        Map = "|x: Vec<u8>| String::from_utf8_lossy(&x[..]).to_string()",
        Parse = "|i| length_count(le_u32, le_u8)(i)"
    )]
    data: String,
}

impl BinWrite for PascalString {
    fn write_options<W: Write>(&self, writer: &mut W, options: &WriterOption) -> Result<(), Error> {
        BinWrite::write_options(&(self.data.len() as u32), writer, options)?;
        BinWrite::write_options(&self.data, writer, options)
    }
}

impl Serialize for PascalString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.data.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PascalString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(PascalString {
            data: String::deserialize(deserializer)?,
        })
    }
}

#[derive(NomLE)]
pub struct PascalStringNULL {
    #[nom(
        Map = "|x: Vec<u8>| String::from_utf8_lossy(&x[0..x.len() - 1]).to_string()",
        Parse = "|i| length_count(le_u32, le_u8)(i)"
    )]
    data: String,
}

impl BinWrite for PascalStringNULL {
    fn write_options<W: Write>(&self, writer: &mut W, options: &WriterOption) -> Result<(), Error> {
        BinWrite::write_options(&(self.data.len() as u32 + 1u32), writer, options)?;
        BinWrite::write_options(&self.data, writer, options)?;
        BinWrite::write_options(&[0u8], writer, options)
    }
}

impl Serialize for PascalStringNULL {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.data.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PascalStringNULL {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(PascalStringNULL {
            data: String::deserialize(deserializer)?,
        })
    }
}

#[derive(NomLE)]
pub struct FixedStringNULL<const U: usize> {
    #[nom(
        Map = "|x: &[u8]| String::from_utf8_lossy(x.split_at(x.iter().position(|&r| r == 0u8).unwrap()).0).to_string()",
        Take = "U"
    )]
    data: String,
}

impl<const U: usize> BinWrite for FixedStringNULL<U> {
    fn write_options<W: Write>(&self, writer: &mut W, options: &WriterOption) -> Result<(), Error> {
        BinWrite::write_options(&self.data, writer, options)?;
        BinWrite::write_options(&vec![0u8; U - self.data.len()], writer, options)
    }
}

impl<const U: usize> Serialize for FixedStringNULL<U> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.data.serialize(serializer)
    }
}

impl<'de, const U: usize> Deserialize<'de> for FixedStringNULL<U> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(FixedStringNULL {
            data: String::deserialize(deserializer)?,
        })
    }
}

#[derive(BinWrite)]
#[binwrite(little)]
#[derive(PartialEq, NomLE)]
pub struct NumeratorFloat<T: BinWrite + NumCast + Copy, const U: usize> {
    pub data: T,
}

impl<T: BinWrite + NumCast + Copy, const U: usize> Serialize for NumeratorFloat<T, U>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let converted: f32 = cast::<T, f32>(self.data).unwrap() / (U as f32);
        converted.serialize(serializer)
    }
}

impl<'de, T: BinWrite + NumCast + Copy, const U: usize> Deserialize<'de> for NumeratorFloat<T, U>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let converted = f32::deserialize(deserializer)?;
        Ok(NumeratorFloat {
            data: cast::<f32, T>(converted * (U as f32)).unwrap(),
        })
    }
}

#[derive(BinWrite)]
#[binwrite(little)]
#[derive(PartialEq, NomLE)]
pub struct VertexVectorComponent {
    pub data: u8,
}

impl Serialize for VertexVectorComponent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let converted: f32 = ((self.data as f32) / 255f32) * 2f32 - 1f32;
        converted.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for VertexVectorComponent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let converted = f32::deserialize(deserializer)?;
        Ok(VertexVectorComponent {
            data: (((converted + 1f32) / 2f32) * 255f32).round() as u8,
        })
    }
}

#[derive(BinWrite)]
#[binwrite(little)]
#[derive(Serialize, Deserialize, NomLE)]
#[nom(Exact)]
pub struct ObjectZ {
    link_crc32: u32,
    data_crc32: u32,
    #[nom(Cond = "i.len() != 90", Count = "data_crc32 as usize + 1")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[binwrite(with(write_option))]
    crc32s: Option<Vec<u32>>,
    rot: Quat,
    transform: Mat4f,
    radius: f32,
    flags: u32,
    object_type: u16,
}

impl HasReferences for ObjectZ {
    fn hard_links(&self) -> Vec<u32> {
        vec![]
    }

    fn soft_links(&self) -> Vec<u32> {
        if let Some(crc32s) = &self.crc32s {
            crc32s.clone()
        } else if self.data_crc32 != 0 {
            vec![self.data_crc32]
        } else {
            vec![]
        }
    }
}

pub trait WALLEObjectFormatTrait {
    fn pack(
        self: &Self,
        input_path: &Path,
        header: &mut Vec<u8>,
        body: &mut Vec<u8>,
    ) -> Result<(Vec<u32>, Vec<u32>), Error>;
    fn unpack(
        self: &Self,
        header: &[u8],
        body: &[u8],
        output_path: &Path,
    ) -> Result<(Vec<u32>, Vec<u32>), Error>;
}

pub struct WALLEObjectFormat<T, U> {
    x: PhantomData<T>,
    y: PhantomData<U>,
}

impl<T, U> WALLEObjectFormat<T, U> {
    pub fn new<'a>() -> &'a Self {
        &Self {
            x: PhantomData,
            y: PhantomData,
        }
    }
}

impl<T, U> WALLEObjectFormatTrait for WALLEObjectFormat<T, U>
where
    for<'a> T: Parse<&'a [u8]> + Serialize + Deserialize<'a> + BinWrite + HasReferences,
    for<'a> U: Parse<&'a [u8]> + Serialize + Deserialize<'a> + BinWrite + HasReferences,
{
    fn pack(
        self: &Self,
        input_path: &Path,
        header: &mut Vec<u8>,
        body: &mut Vec<u8>,
    ) -> Result<(Vec<u32>, Vec<u32>), Error> {
        let json_path = input_path.join("object.json");
        let json_file = File::open(json_path)?;

        #[derive(Serialize, Deserialize)]
        struct Object<T, U> {
            header: T,
            body: U,
        }

        let object: Object<T, U> = serde_json::from_reader(json_file)?;

        object.header.write(header)?;
        object.body.write(body)?;

        let soft_links = [
            &object.header.soft_links()[..],
            &object.body.soft_links()[..],
        ]
        .concat();
        let hard_links = [
            &object.header.hard_links()[..],
            &object.body.hard_links()[..],
        ]
        .concat();

        Ok((hard_links, soft_links))
    }

    fn unpack(
        self: &Self,
        header: &[u8],
        body: &[u8],
        output_path: &Path,
    ) -> Result<(Vec<u32>, Vec<u32>), Error> {
        let json_path = output_path.join("object.json");
        let mut output_file = File::create(json_path)?;

        let header = match T::parse(&header) {
            Ok((_, h)) => h,
            Err(e) => return Err(Error::from(ErrorKind::Other)),
        };

        let body = match U::parse(&body) {
            Ok((_, h)) => h,
            Err(e) => return Err(Error::from(ErrorKind::Other)),
        };

        #[derive(Serialize, Deserialize)]
        struct Object<T, U> {
            header: T,
            body: U,
        }

        let object = Object { header, body };

        output_file.write(serde_json::to_string_pretty(&object)?.as_bytes())?;

        let soft_links = [
            &object.header.soft_links()[..],
            &object.body.soft_links()[..],
        ]
        .concat();
        let hard_links = [
            &object.header.hard_links()[..],
            &object.body.hard_links()[..],
        ]
        .concat();

        Ok((hard_links, soft_links))
    }
}

#[derive(BinWrite)]
#[binwrite(little)]
#[derive(Serialize, Deserialize, NomLE)]
pub struct DynSphere {
    sphere: SphereZ,
    flags: u32,
    dyn_sphere_name: u32,
}

#[derive(BinWrite)]
#[binwrite(little)]
#[derive(Serialize, Deserialize, NomLE)]
pub struct DynBox {
    mat: Mat4f,
    flags: u32,
    dyn_box_name: u32,
}
