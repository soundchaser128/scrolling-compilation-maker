use std::{num::ParseIntError, path::PathBuf, sync::LazyLock, time::Duration};

use clap::ValueEnum;
use rand::prelude::IndexedRandom;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct AspectRatio {
    pub w: u32,
    pub h: u32,
}

impl AspectRatio {
    pub fn parse(s: &str) -> Result<AspectRatio, String> {
        let (w, h) = s
            .split_once(':')
            .ok_or_else(|| format!("expected W:H format, got '{s}'"))?;
        let w: u32 = w.trim().parse().map_err(|e: ParseIntError| e.to_string())?;
        let h: u32 = h.trim().parse().map_err(|e: ParseIntError| e.to_string())?;
        if w == 0 || h == 0 {
            return Err("aspect ratio components must be > 0".to_string());
        }
        Ok(AspectRatio { w, h })
    }

    /// Given a viewport height, return the crop width for this aspect ratio.
    pub fn crop_width(&self, height: u32) -> u32 {
        let mut w = (height as u64 * self.w as u64 / self.h as u64) as u32;
        w += w % 2; // round up to even
        w
    }
}

static ADJECTIVES: LazyLock<Vec<&str>> = LazyLock::new(|| {
    include_str!("../data/adjectives.txt")
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
});

static NOUNS: LazyLock<Vec<&str>> = LazyLock::new(|| {
    include_str!("../data/nouns.txt")
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
});

#[derive(Debug, Clone, Copy)]
enum IdFormat {
    Long,
    Short,
}

fn random_id(format: IdFormat) -> String {
    let mut rng = rand::rng();
    let adj1 = ADJECTIVES.choose(&mut rng).unwrap();
    let adj2 = ADJECTIVES.choose(&mut rng).unwrap();
    let noun = NOUNS.choose(&mut rng).unwrap();
    match format {
        IdFormat::Long => format!("{}-{}-{}", adj1, adj2, noun),
        IdFormat::Short => format!("{}{}{}", &adj1[0..2], &adj2[0..2], &noun[0..2]),
    }
}

/// Generate an output filename from CLI parameters or a random adjective-adjective-noun name.
pub fn generate_output_name(tags: &[String], people: &[String]) -> String {
    let mut parts: Vec<String> = Vec::new();

    for p in people {
        parts.push(slugify(p));
    }

    for t in tags {
        parts.push(slugify(t));
    }

    if parts.is_empty() {
        format!("{}.mp4", random_id(IdFormat::Long))
    } else {
        format!("{}-{}.mp4", parts.join("-"), random_id(IdFormat::Short))
    }
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Parse a human-friendly duration string like "30s", "1m", "1m30s", "90".
/// Plain numbers are treated as seconds.
pub fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();

    // Plain number = seconds
    if let Ok(secs) = s.parse::<u64>() {
        return Ok(Duration::from_secs(secs));
    }

    let mut total_secs: u64 = 0;
    let mut current = String::new();

    for c in s.chars() {
        if c.is_ascii_digit() {
            current.push(c);
        } else {
            let value: u64 = current.parse().map_err(|e: ParseIntError| e.to_string())?;
            current.clear();
            match c {
                'h' => total_secs += value * 3600,
                'm' => total_secs += value * 60,
                's' => total_secs += value,
                _ => return Err(format!("unknown duration unit '{c}', expected h/m/s")),
            }
        }
    }

    if !current.is_empty() {
        return Err(format!(
            "trailing number '{current}' without unit, use e.g. '{current}s'"
        ));
    }

    if total_secs == 0 {
        return Err("duration must be greater than 0".to_string());
    }

    Ok(Duration::from_secs(total_secs))
}

#[derive(Clone, ValueEnum)]
pub enum Orientation {
    Any,
    Portrait,
    Landscape,
    Square,
}

impl Orientation {
    pub fn as_api_param(&self) -> Option<&'static str> {
        match self {
            Orientation::Any => None,
            Orientation::Portrait => Some("Portrait"),
            Orientation::Landscape => Some("Landscape"),
            Orientation::Square => Some("Square"),
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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    Video,
    Image,
    Text,
}

#[derive(Deserialize, Debug)]
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
    pub popularity: f32,
    pub tags: Vec<String>,
}

#[derive(Deserialize, Debug)]
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
    /// Width after scaling to viewport height (preserving aspect ratio).
    pub scaled_width: u32,
    /// Width after optional crop (same as scaled_width if no crop).
    pub output_width: u32,
    pub performers: Vec<String>,
    pub tags: Vec<String>,
    pub popularity: f32,
    pub is_image: bool,
}

#[derive(Clone, ValueEnum)]
pub enum Codec {
    X264,
    Hevc,
    Av1,
}

#[derive(Clone, ValueEnum)]
pub enum Quality {
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(Clone, ValueEnum)]
pub enum Effort {
    Low,
    Medium,
    High,
}

pub struct EncodingArgs {
    pub codec: String,
    pub quality_flag: &'static str,
    pub quality_value: u32,
    pub preset_args: Vec<String>,
}

impl EncodingArgs {
    pub fn new(codec: &Codec, quality: &Quality, effort: &Effort, gpu: bool) -> Self {
        if gpu {
            if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
                Self::videotoolbox(codec, quality)
            } else {
                Self::nvenc(codec, quality, effort)
            }
        } else {
            Self::software(codec, quality, effort)
        }
    }

    fn software(codec: &Codec, quality: &Quality, effort: &Effort) -> Self {
        let (codec_name, crf) = match codec {
            Codec::X264 => (
                "libx264",
                match quality {
                    Quality::Low => 28,
                    Quality::Medium => 23,
                    Quality::High => 18,
                    Quality::VeryHigh => 14,
                },
            ),
            Codec::Hevc => (
                "libx265",
                match quality {
                    Quality::Low => 32,
                    Quality::Medium => 28,
                    Quality::High => 23,
                    Quality::VeryHigh => 18,
                },
            ),
            Codec::Av1 => (
                "libsvtav1",
                match quality {
                    Quality::Low => 38,
                    Quality::Medium => 32,
                    Quality::High => 26,
                    Quality::VeryHigh => 20,
                },
            ),
        };

        let preset = match codec {
            Codec::X264 | Codec::Hevc => match effort {
                Effort::Low => "ultrafast",
                Effort::Medium => "medium",
                Effort::High => "veryslow",
            },
            Codec::Av1 => match effort {
                Effort::Low => "10",
                Effort::Medium => "6",
                Effort::High => "2",
            },
        };

        EncodingArgs {
            codec: codec_name.to_string(),
            quality_flag: "-crf",
            quality_value: crf,
            preset_args: vec!["-preset".to_string(), preset.to_string()],
        }
    }

    fn nvenc(codec: &Codec, quality: &Quality, effort: &Effort) -> Self {
        let (codec_name, cq) = match codec {
            Codec::X264 => (
                "h264_nvenc",
                match quality {
                    Quality::Low => 32,
                    Quality::Medium => 26,
                    Quality::High => 20,
                    Quality::VeryHigh => 16,
                },
            ),
            Codec::Hevc => (
                "hevc_nvenc",
                match quality {
                    Quality::Low => 34,
                    Quality::Medium => 28,
                    Quality::High => 22,
                    Quality::VeryHigh => 18,
                },
            ),
            Codec::Av1 => (
                "av1_nvenc",
                match quality {
                    Quality::Low => 38,
                    Quality::Medium => 32,
                    Quality::High => 26,
                    Quality::VeryHigh => 20,
                },
            ),
        };

        let preset = match effort {
            Effort::Low => "p1",
            Effort::Medium => "p4",
            Effort::High => "p7",
        };

        EncodingArgs {
            codec: codec_name.to_string(),
            quality_flag: "-cq",
            quality_value: cq,
            preset_args: vec![
                "-preset".to_string(),
                preset.to_string(),
                "-rc".to_string(),
                "vbr".to_string(),
            ],
        }
    }

    fn videotoolbox(codec: &Codec, quality: &Quality) -> Self {
        let (codec_name, q) = match codec {
            Codec::X264 => (
                "h264_videotoolbox",
                match quality {
                    Quality::Low => 65,
                    Quality::Medium => 55,
                    Quality::High => 42,
                    Quality::VeryHigh => 30,
                },
            ),
            Codec::Hevc => (
                "hevc_videotoolbox",
                match quality {
                    Quality::Low => 65,
                    Quality::Medium => 55,
                    Quality::High => 42,
                    Quality::VeryHigh => 30,
                },
            ),
            Codec::Av1 => {
                // VideoToolbox doesn't support AV1 encoding; fall back to software
                return Self::software(codec, quality, &Effort::Medium);
            }
        };

        EncodingArgs {
            codec: codec_name.to_string(),
            quality_flag: "-q:v",
            quality_value: q,
            preset_args: vec![
                "-allow_sw".to_string(),
                "1".to_string(),
                "-realtime".to_string(),
                "0".to_string(),
            ],
        }
    }
}
