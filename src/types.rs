use std::path::PathBuf;

use clap::ValueEnum;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Clone, ValueEnum)]
pub enum Orientation {
    Portrait,
    Landscape,
    Square,
}

impl Orientation {
    pub fn as_api_param(&self) -> &'static str {
        match self {
            Orientation::Portrait => "Portrait",
            Orientation::Landscape => "Landscape",
            Orientation::Square => "Square",
        }
    }
}

#[derive(Deserialize)]
pub struct PageResponse<T> {
    pub content: Vec<T>,
    pub page: PageInfo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct PageInfo {
    pub size: u32,
    pub number: u32,
    pub total_elements: u64,
    pub total_pages: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    Video,
    Image,
    Text,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFile {
    pub id: Uuid,
    pub title: String,
    pub file_type: FileType,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration: Option<u64>,
    pub mime_type: String,
    #[serde(default)]
    pub people: Vec<Person>,
}

#[derive(Deserialize)]
pub struct Person {
    pub name: String,
    #[serde(rename = "type")]
    pub person_type: String,
}

impl VideoFile {
    pub fn is_image(&self) -> bool {
        matches!(self.file_type, FileType::Image)
    }

    pub fn content_url(&self, base_url: &str) -> String {
        let id = self.id.to_string();
        let first_char = &id[0..1];
        let ext = extension_for_mime(&self.mime_type);
        format!("{base_url}/{first_char}/{id}{ext}")
    }
}

pub fn extension_for_mime(mime: &str) -> &'static str {
    match mime {
        "video/mp4"
        | "video/x-m4v"
        | "audio/mp4"
        | "video/quicktime"
        | "application/x-matroska"
        | "application/octet-stream" => ".mp4",
        "video/webm" => ".webm",
        "image/jpeg" => ".jpeg",
        "image/png" => ".png",
        "image/gif" => ".gif",
        "image/webp" => ".webp",
        "image/avif" => ".avif",
        _ => ".mp4",
    }
}

pub struct ClipInfo {
    pub path: PathBuf,
    pub scaled_width: u32,
    pub performer_name: Option<String>,
    pub is_image: bool,
}
