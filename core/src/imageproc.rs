use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use base64::Engine;
use image::ImageReader;
use sha2::{Digest, Sha256};

pub struct FileMeta {
  pub path: std::path::PathBuf,
  pub sha256: String,
  pub file_size: i64,
  pub width: i64,
  pub height: i64,
  pub camera: String,
  pub taken_at: Option<String>,
  pub has_xmp: bool,
  pub preview_available: bool,
  pub indexed_at: String,
}

const RAIL_EXTENSIONS: &[&str] = &[
  "jpg", "jpeg", "png", "webp", "tif", "tiff", "bmp",
  // RAW
  "cr3", "cr2", "crw", "nef", "nrw", "arw", "sr2", "srf", "raf", "rw2", "dng",
  "orf", "pef", "x3f", "raw", "rwl", "3fr", "fff", "mef", "mos", "iiq",
];

pub fn is_photo_path(path: &Path) -> bool {
  path
    .extension()
    .and_then(|s| s.to_str())
    .map(|e| RAIL_EXTENSIONS.contains(&e.to_lowercase().as_str()))
    .unwrap_or(false)
}

fn sha256_file(path: &Path) -> Result<String, String> {
  let mut file = File::open(path).map_err(|e| e.to_string())?;
  let mut hasher = Sha256::new();
  std::io::copy(&mut file, &mut hasher).map_err(|e| e.to_string())?;
  Ok(base64::engine::general_purpose::STANDARD.encode(hasher.finalize()))
}

pub fn inspect_file(path: &Path) -> Result<FileMeta, String> {
  let file_size = std::fs::metadata(path)
    .map(|m| m.len() as i64)
    .map_err(|e| e.to_string())?;
  let sha256 = sha256_file(path)?;
  let indexed_at = chrono::Utc::now().to_rfc3339();
  let has_xmp = Path::new(&format!("{}.xmp", path.to_string_lossy())).exists();

  let mut meta = FileMeta {
    path: path.to_path_buf(),
    sha256,
    file_size,
    width: 0,
    height: 0,
    camera: String::new(),
    taken_at: None,
    has_xmp,
    preview_available: false,
    indexed_at,
  };

  // Try to extract dimensions + camera from EXIF (fast, no full decode).
  let dims_camera = read_exif_basic(path);
  if let Some((w, h)) = dims_camera.dims {
    meta.width = w;
    meta.height = h;
  }
  meta.camera = dims_camera.camera;
  meta.taken_at = dims_camera.taken_at;

  // Mark preview available only if we can actually decode it.
  meta.preview_available = extract_preview_bytes(path).map(|b| !b.is_empty()).unwrap_or(false);

  Ok(meta)
}

pub struct ExifBasic {
  pub dims: Option<(i64, i64)>,
  pub camera: String,
  pub taken_at: Option<String>,
}

pub fn read_exif_basic(path: &Path) -> ExifBasic {
  let mut result = ExifBasic {
    dims: None,
    camera: String::new(),
    taken_at: None,
  };
  if !path.exists() {
    return result;
  }
  let file = match File::open(path) {
    Ok(f) => f,
    Err(_) => return result,
  };
  let mut bufreader = BufReader::new(file);
  let exifreader = exif::Reader::new();
  if let Ok(exif) = exifreader.read_from_container(&mut bufreader) {
    // Dimensions: try several tags, take whatever combination is present.
    let mut w: Option<u32> = None;
    let mut h: Option<u32> = None;
    for (tag, is_width) in [
      (exif::Tag::PixelXDimension, true),
      (exif::Tag::ImageWidth, true),
    ] {
      if w.is_none() {
        if let Some(f) = exif.get_field(tag, exif::In::PRIMARY) {
          if let Some(v) = f.value.get_uint(0) {
            w = Some(v);
          }
        }
      }
      let _ = is_width;
    }
    for tag in [exif::Tag::PixelYDimension, exif::Tag::ImageLength] {
      if h.is_none() {
        if let Some(f) = exif.get_field(tag, exif::In::PRIMARY) {
          if let Some(v) = f.value.get_uint(0) {
            h = Some(v);
          }
        }
      }
    }
    if let (Some(w), Some(h)) = (w, h) {
      result.dims = Some((w as i64, h as i64));
    }

    if let Some(f) = exif.get_field(exif::Tag::Model, exif::In::PRIMARY) {
      result.camera = f.display_value().with_unit(&exif).to_string();
    } else if let Some(f) = exif.get_field(exif::Tag::Make, exif::In::PRIMARY) {
      result.camera = f.display_value().with_unit(&exif).to_string();
    }

    if let Some(f) = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY) {
      let v = f.display_value().with_unit(&exif).to_string();
      if !v.is_empty() {
        result.taken_at = Some(v);
      }
    }
  }
  result
}

/// Extract the embedded JPEG preview bytes from a RAW/TIFF file using Exif
/// tags JPEGInterchangeFormat (offset) and JPEGInterchangeFormatLength (size).
///
/// Works for NEF/ARW/DNG (TIFF-based) and some others. For CR3/HEIF we fall
/// back to the full-file decode path (see `extract_preview_bytes`).
pub fn read_embedded_jpeg(path: &Path) -> Option<Vec<u8>> {
  if !path.exists() {
    return None;
  }
  let mut file = File::open(path).ok()?;
  let reader = exif::Reader::new();
  // We need to parse the exif from the file bytes. kamadak provides
  // read_from_container, but to locate the embedded JPEG we read the raw
  // ifd offset/length fields. Re-open via BufReader.
  let mut data = Vec::new();
  file.seek(SeekFrom::Start(0)).ok()?;
  file.read_to_end(&mut data).ok()?;
  let mut cursor = std::io::Cursor::new(&data);
  let exif = reader.read_from_container(&mut cursor).ok()?;

  let offset = match exif
    .get_field(exif::Tag::JPEGInterchangeFormat, exif::In::PRIMARY)
    .and_then(|f| f.value.get_uint(0))
  {
    Some(v) => v as u64,
    None => return None,
  };
  let length = match exif
    .get_field(exif::Tag::JPEGInterchangeFormatLength, exif::In::PRIMARY)
    .and_then(|f| f.value.get_uint(0))
  {
    Some(v) => v as usize,
    None => return None,
  };
  if length == 0 {
    return None;
  }
  let start = offset as usize;
  if start.checked_add(length)? > data.len() {
    return None;
  }
  Some(data[start..start + length].to_vec())
}

fn extract_preview_bytes(path: &Path) -> Result<Vec<u8>, String> {
  // Strategy 1: embedded JPEG preview from RAW/TIFF via exif offsets.
  if let Some(jpeg) = read_embedded_jpeg(path) {
    return Ok(jpeg);
  }
  // Strategy 2: full-file decode (JPG/PNG/WebP). For CR3/HEIF this does not
  // work yet — will be added in Fase 1+ via a dedicated container parser.
  let ext = path
    .extension()
    .and_then(|s| s.to_str())
    .unwrap_or("")
    .to_lowercase();
  match ext.as_str() {
    "cr3" | "cr2" | "nef" | "arw" | "dng" | "rw2" | "orf" | "raf" | "pef" => {
      // RAW without available embedded offset: not yet supported for preview.
      // We still return Ok(empty) so the record is marked available=False below
      // by checking length.
      Err("RAW decode sem suporte de preview embutido ainda".to_string())
    }
    _ => {
      // Normal image: full decode.
      let img = ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .decode()
        .map_err(|e| e.to_string())?;
      let mut out = std::io::Cursor::new(Vec::new());
      img
        .write_to(&mut out, image::ImageFormat::Jpeg)
        .map_err(|e| e.to_string())?;
      Ok(out.into_inner())
    }
  }
}

/// Generate a small thumbnail (default max 256px) as JPEG bytes from a photo.
pub fn thumbnail_for(path: &Path, max_dim: u32) -> Result<Vec<u8>, String> {
  let bytes = extract_preview_bytes(path)?;
  if bytes.is_empty() {
    return Err("sem preview disponivel".to_string());
  }
  let img = ImageReader::new(std::io::Cursor::new(bytes))
    .with_guessed_format()
    .map_err(|e| e.to_string())?
    .decode()
    .map_err(|e| e.to_string())?;
  let thumb = img.thumbnail(max_dim, max_dim);
  let mut out = std::io::Cursor::new(Vec::new());
  thumb
    .write_to(&mut out, image::ImageFormat::Jpeg)
    .map_err(|e| e.to_string())?;
  Ok(out.into_inner())
}

pub fn thumbnail_base64(path: &Path, max_dim: u32) -> Result<String, String> {
  let bytes = thumbnail_for(path, max_dim)?;
  Ok(format!(
    "data:image/jpeg;base64,{}",
    base64::engine::general_purpose::STANDARD.encode(bytes)
  ))
}
