use vulkano::sampler::{Filter, SamplerAddressMode};
use vulkano::format::{Format, R8G8B8A8Srgb};
use toolbelt::color::LinearColor;
use std::sync::Arc;
use vulkano::image::{ImmutableImage, Dimensions};


#[allow(non_camel_case_types)]
#[derive(Debug, Clone)]
pub enum Texture {
    RGBA8_Srgb(Arc<ImmutableImage<R8G8B8A8Srgb>>),
}

bitflags! {
  pub struct ChannelMask: u8 {
      const RED   = 1;
      const GREEN = 2;
      const BLUE  = 4;
      const ALPHA = 8;
  }
}

#[derive(Debug, Clone)]
pub enum CompressionMode {
    None,
    DXT1,
    DXT1Cutout,
    DXT5
}
impl std::fmt::Display for CompressionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatstr = match self {
            CompressionMode::None => "No compression",
            CompressionMode::DXT1 => "DXT1",
            CompressionMode::DXT1Cutout => "DXT1 w/ 1-bit Alpha",
            CompressionMode::DXT5 => "DXT5",
        };
        write!(f, "{}", formatstr)
    }
}

#[derive(Debug, Clone)]
pub enum TextureSize {
    _8x8 = 8,
    _16x16 = 16,
    _32x32 = 32,
    _64x64 = 64,
    _128x128 = 128,
    _256x256 = 256,
    _512x512 = 512,
    _1024x1024 = 1024,
    _2048x2048 = 2048,
    _4096x4096 = 4096,
    _8192x8192 = 8192,
}

#[derive(Debug, Clone)]
pub enum MipGenSettings {
    NoMipmaps,
    Linear,
    Nearest,
    Sharpen,
    Blur
}

#[derive(Debug, Clone)]
pub enum PowerOfTwoMode {
    None,
    PadToPowerOfTwo,
    PadToSquarePowerOfTwo,
}

#[derive(Debug, Clone)]
pub struct TextureMetadata {
    // info block:
    pub source_size: [u32; 2],
    pub max_ingame_size: [u32; 2],
    // bytes, uncompressed and compressed
    pub data_size: [u32; 2],
    pub has_channels: ChannelMask,
    pub format: Format,
    pub num_mips: u8,

    // compresion block:
    pub compression_mode: CompressionMode,
    pub include_channels: ChannelMask,
    pub max_texture_size: Option<TextureSize>,
    pub mip_gen_settings: MipGenSettings,
    pub lod_bias: u8,

    // texture block:
    pub power_of_two_mode: PowerOfTwoMode,
    pub padding_color: LinearColor,
    pub srgb: bool,
    pub x_axis_tiling: SamplerAddressMode,
    pub y_axis_tiling: SamplerAddressMode,
    pub invert_green: bool,
    pub filter: Filter,

    // adjustments block
    // TODO: texture adjustments
}
impl TextureMetadata {
    pub fn dimensions(&self) -> Dimensions {
        Dimensions::Dim2d {
            width: self.source_size[0],
            height: self.source_size[1]
        }
    }
}
impl Default for TextureMetadata {
    fn default() -> Self {
        Self {
            source_size: [0, 0],
            max_ingame_size: [0, 0],
            data_size: [0, 0],
            has_channels: ChannelMask::all(),
            format: Format::R8G8B8A8Srgb,
            num_mips: 0,
            compression_mode: CompressionMode::None,
            include_channels: ChannelMask::all(),
            max_texture_size: None,
            mip_gen_settings: MipGenSettings::NoMipmaps,
            lod_bias: 0,
            power_of_two_mode: PowerOfTwoMode::None,
            padding_color: LinearColor::BLACK,
            srgb: true,
            x_axis_tiling: SamplerAddressMode::Repeat,
            y_axis_tiling: SamplerAddressMode::Repeat,
            invert_green: false,
            filter: Filter::Linear,
        }
    }
}
