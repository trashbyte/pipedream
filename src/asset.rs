use chrono::{DateTime, Local};
use hashbrown::HashMap;
use crate::texture::TextureMetadata;


#[derive(Debug)]
pub enum FileTreeNode {
    Directory(HashMap<String, FileTreeNode>),
    File(Asset),
}


// Asset types / internals /////////////////////////////////////////////////////////////////////////


#[derive(Debug)]
pub enum AssetData {
    Texture(TextureAssetData)
}

impl AssetData {
    pub fn asset_type(&self) -> AssetType {
        match self {
            AssetData::Texture(_) => AssetType::Texture,
        }
    }
}

#[derive(Debug)]
pub enum AssetType {
    Texture
}


// Asset main struct ///////////////////////////////////////////////////////////////////////////////


#[derive(Debug)]
pub struct Asset {
    pub path: String,
    pub timestamp: DateTime<Local>,
    pub uid: u64,
    pub thumbnail_id: Option<u64>,
    pub data: AssetData,
}

impl Asset {
    pub fn new(path: &str, timestamp: DateTime<Local>, uid: u64, thumbnail_id: Option<u64>, data: AssetData) -> Self {
        Self {
            path: path.to_string(),
            timestamp,
            uid,
            thumbnail_id,
            data
        }
    }
}


// Specific asset inner types //////////////////////////////////////////////////////////////////////


#[derive(Debug)]
pub struct TextureAssetData {
    pub settings: TextureMetadata,
    pub data: Vec<u8>,
}

impl TextureAssetData {
    pub fn new(settings: TextureMetadata, data: Vec<u8>) -> Self {
        Self { settings, data }
    }
}
