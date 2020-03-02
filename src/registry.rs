use std::path::Path;
use walkdir::{WalkDir, DirEntry};
use std::fmt::{Display, Formatter, Error};
use hashbrown::HashMap;
use chrono::{DateTime, Local};
use toolbelt::color::LinearColor;
use vulkano::sampler::{SamplerAddressMode, Filter};
use vulkano::format::Format;
use image::{ImageDecoder, ColorType};
use vulkano::image::ImmutableImage;
use std::sync::Arc;
use vulkano::device::Queue;
use itertools::Itertools;

use crate::texture::{TextureMetadata, CompressionMode, MipGenSettings, PowerOfTwoMode, ChannelMask, Texture};
use crate::asset::{Asset, TextureAssetData, AssetData, FileTreeNode};


#[derive(Debug)]
pub enum AssetRegistryError {
    PathDoesNotExist(String),
    WalkDirError(walkdir::Error),
    Other(Error)
}

impl Display for AssetRegistryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            AssetRegistryError::PathDoesNotExist(path) => {
                write!(f, "Path does not exist: '{}'", path)?;
            },
            AssetRegistryError::WalkDirError(e) => {
                write!(f, "{}", e)?;
            },
            AssetRegistryError::Other(e) => {
                write!(f, "{}", e)?;
            }
        }
        Ok(())
    }
}
impl From<Error> for AssetRegistryError {
    fn from(e: Error) -> Self {
        AssetRegistryError::Other(e)
    }
}
impl From<walkdir::Error> for AssetRegistryError {
    fn from(e: walkdir::Error) -> Self {
        AssetRegistryError::WalkDirError(e)
    }
}


#[derive(Debug)]
pub struct AssetRegistry {
    pub base_path_relative: String,
    pub base_path_absolute: String,
    pub queue: Arc<Queue>,
    pub file_tree: FileTreeNode,
    pub cached_texture_arcs: HashMap<String, Texture>,
    pub uid_to_path: HashMap<u64, String>,
}

impl AssetRegistry {
    pub fn new(base_path_relative: &str, base_path_absolute: &str, queue: Arc<Queue>) -> Result<Self, AssetRegistryError> {
        if Path::new(base_path_relative).exists() {
            Ok(Self {
                queue,
                base_path_relative: base_path_relative.to_string(),
                base_path_absolute: base_path_absolute.to_string(),
                file_tree: FileTreeNode::Directory(HashMap::new()),
                cached_texture_arcs: HashMap::new(),
                uid_to_path: HashMap::new(),
            })
        }
        else {
            Err(AssetRegistryError::PathDoesNotExist(base_path_relative.to_string()))
        }
    }

    pub fn rescan(&mut self) -> Result<(), AssetRegistryError> {
        for entry in WalkDir::new(&self.base_path_relative).into_iter()
                                                  .filter_map(Result::ok)
                                                  .filter(|e| !e.file_type().is_dir())
        {
            let path_segments: Vec<String> = entry.path()
                .to_str()
                .unwrap()
                .to_string()
                .replace("\\", "/")
                .split("/")
                .map(|s| s.to_string())
                .skip(1)
                .collect();
            let segments_copy = path_segments.clone();
            let all_except_last = path_segments.len() - 1;
            let path_segments: Vec<String> = path_segments.into_iter().take(all_except_last).collect();
            let dir_node = self.get_node_and_create_if_none(path_segments);

            // assuming it doesn't exist by default
            let mut should_process = true;

            // search asset directory entry for file
            let mut new_id = None;
            match dir_node {
                FileTreeNode::File(_) => unreachable!(),
                FileTreeNode::Directory(ref mut map) => {
                    'outer: for (_, value) in map.iter() {
                        match value {
                            FileTreeNode::Directory(_) => continue,
                            FileTreeNode::File(asset) => {
                                if Path::new(&asset.path).file_name().unwrap() == entry.file_name() {
                                    // found file with the same name
                                    let file_time = entry.metadata().unwrap().modified().expect("This platform doesn't support file timestamps!");
                                    let file_time = DateTime::<Local>::from(file_time);
                                    if asset.timestamp != file_time {
                                        // timestamps are different, reprocess (true by default)
                                    }
                                    else {
                                        // else timestamps are the same, don't reprocess
                                        should_process = false;
                                    }
                                    break 'outer;
                                }
                            }
                        }
                    }
                    // if not found or newer timestamp
                    if should_process {
                        let filename = entry.file_name().to_str().unwrap().to_string();
                        match process_file(&entry) {
                            Some(new_asset) => {
                                new_id = Some(new_asset.uid);
                                map.insert(filename.clone(), FileTreeNode::File(new_asset));
                            },
                            None => {} // unsupported file
                        }

                    }
                }
            }
            if let Some(id) = new_id {
                self.uid_to_path.insert(id, segments_copy.join("/"));
            }
        }
        Ok(())
    }

    fn get_node_and_create_if_none(&mut self, path_segments: Vec<String>) -> &mut FileTreeNode {
        let mut iter = path_segments.iter();
        let mut current_node = &mut self.file_tree;
        while let Some(segment) = iter.next() {
            match current_node {
                FileTreeNode::File(_) => panic!("Directory node already exists as a file. This shouldn't be possible"),
                FileTreeNode::Directory(ref mut map) => {
                    if !map.contains_key(segment.as_str()) {
                        map.insert(segment.clone(), FileTreeNode::Directory(HashMap::new()));
                    }
                    current_node = map.get_mut(segment.as_str()).unwrap();
                }
            }
        }
        current_node
    }

    pub fn get_assets_in_directory(&self, path: &str) -> Option<Vec<&Asset>> {
        let pathstr = path.to_string().replace("\\", "/");
        let mut split = pathstr.split("/").into_iter().peekable();
        let mut current_node = &self.file_tree;
        while let Some(segment) = split.next() {
            match current_node {
                FileTreeNode::File(_) => {
                    // if current_node is ever a file, we reached a file before the destination
                    return None;
                },
                FileTreeNode::Directory(map) => {
                    if split.peek().is_none() {
                        // found target dir, return assets
                        let mut results = Vec::new();
                        for (_, node) in map.iter() {
                            match node {
                                FileTreeNode::Directory(_) => {},
                                FileTreeNode::File(asset) => results.push(asset)
                            }
                        }
                        return Some(results)
                    }
                    else {
                        match map.get(segment) {
                            Some(node) => {
                                current_node = node;
                            },
                            None => {
                                // requested path segment not found
                                return None;
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_asset(&self, path: &str) -> Option<&Asset> {
        let pathstr = path.to_string().replace("\\", "/");
        let pathstr = pathstr.trim_start_matches(&self.base_path_absolute);
        let mut split = pathstr.split("/").into_iter().filter(|s| s.len() > 0).peekable();
        let mut current_node = &self.file_tree;
        while let Some(segment) = split.next() {
            match current_node {
                FileTreeNode::File(_) => {
                    // if current_node is ever a file, we reached a different file before the destination
                    return None;
                },
                FileTreeNode::Directory(map) => {
                    match map.get(segment) {
                        Some(node) => {
                            current_node = node;
                            if let FileTreeNode::File(asset) = current_node {
                                if split.peek().is_none() {
                                    return Some(&asset);
                                }
                            }
                        },
                        None => {
                            // requested path segment not found
                            return None;
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_path_from_id(&self, id: u64) -> Option<&String> {
        self.uid_to_path.get(&id)
    }

    pub fn get_texture(&mut self, path: &str) -> Option<Texture> {
        match self.get_asset(path) {
            Some(asset) => {
                match &asset.data {
                    AssetData::Texture(tex_data) => {
                        match self.cached_texture_arcs.get(&path.to_string()) {
                            Some(a) => Some(a.clone()),
                            None => {
                                match tex_data.settings.format {
                                    Format::R8G8B8A8Srgb => {
                                        let (img, future) = ImmutableImage::from_iter(tex_data.data.iter().cloned(),
                                                                  tex_data.settings.dimensions(),
                                                                  vulkano::format::R8G8B8A8Srgb,
                                                                  self.queue.clone()).unwrap();
                                        self.cached_texture_arcs.insert(path.to_string(), Texture::RGBA8_Srgb(img.clone()));
                                        drop(future);
                                        Some(Texture::RGBA8_Srgb(img))
                                    },
                                    _ => unimplemented!()
                                }
                            }
                        }
                    },
                }
            },
            None => None
        }
    }
}

fn process_file(entry: &DirEntry) -> Option<Asset> {
    let filename = entry.file_name().to_str().unwrap().to_string();
    if let Some(ext) = entry.path().extension() {
        let ext = ext.to_str().unwrap();
        if ["png", "jpg", "tga", "dds"].contains(&ext) {
            return process_texture(entry, &filename, ext);
        }
    }
    None
}

// TODO: extract asset processors to another module
fn process_texture(entry: &DirEntry, filename: &str, ext: &str) -> Option<Asset> {
    match ext {
        "png" => {
            // TODO: handle errors here
            let reader = image::png::PNGDecoder::new(std::fs::File::open(entry.path()).unwrap()).unwrap();
            let dimensions = [reader.dimensions().0 as u32, reader.dimensions().1 as u32];
            let timestamp = DateTime::<Local>::from(entry.metadata().unwrap().modified().unwrap());

            let format;
            let has_channels;
            let include_channels;
            let colortype = reader.colortype();
            match colortype {
                ColorType::RGB(8) => {
                    format = Format::R8G8B8A8Srgb;
                    has_channels = ChannelMask::RED | ChannelMask::GREEN | ChannelMask::BLUE;
                    include_channels = has_channels
                },
                ColorType::RGBA(8) => {
                    format = Format::R8G8B8A8Srgb;
                    has_channels = ChannelMask::all();
                    include_channels = has_channels
                },
                colortype => {
                    println!("Unsupported color type: {} - {:?}", filename, colortype);
                    return None;
                }
            }

            let mut result_data = Vec::new();
            let imgdata = reader.read_image().unwrap();
            let bytes = imgdata.into_iter();
            match colortype {
                ColorType::RGB(8) => {
                    for rgb in &bytes.chunks(3) {
                        result_data.extend(rgb);
                        result_data.push(255u8);
                    }
                },
                ColorType::RGBA(8) => {
                    result_data.extend(bytes);
                },
                _ => unreachable!()
            }
            let id: u64 = rand::random();

            let texture_data = TextureMetadata {
                source_size: dimensions,
                max_ingame_size: dimensions,
                data_size: [result_data.len() as u32, 0],
                has_channels,
                format,
                num_mips: 0,
                compression_mode: CompressionMode::None,
                include_channels,
                max_texture_size: None,
                mip_gen_settings: MipGenSettings::NoMipmaps,
                lod_bias: 0,
                power_of_two_mode: PowerOfTwoMode::None,
                padding_color: LinearColor::BLACK,
                srgb: true,
                x_axis_tiling: SamplerAddressMode::Repeat,
                y_axis_tiling: SamplerAddressMode::Repeat,
                invert_green: false,
                filter: Filter::Linear
            };

            Some(Asset::new(filename, timestamp, id, None, AssetData::Texture(
                TextureAssetData::new(texture_data, result_data))
            ))
        },
        _ => None
    }
}
