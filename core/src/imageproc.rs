use std::fs::File;
use std::io::{BufReader, Read};
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
  if let Some((mut w, mut h)) = dims_camera.dims {
    // Orientação 5-8 = 90/270°: troca largura/altura para refletir o retrato.
    if (5..=8).contains(&dims_camera.orientation) {
      std::mem::swap(&mut w, &mut h);
    }
    meta.width = w;
    meta.height = h;
  }
  meta.camera = dims_camera.camera;
  meta.taken_at = dims_camera.taken_at;
  meta.orientation = dims_camera.orientation;

  // Mark preview available only if we can actually decode it.
  // RAW previews são resolvidos async via jpgfromraw-lib; aqui marcamos true
  // apenas para formatos comuns (o grid real decide ao gerar o thumbnail).
  let ext = path
    .extension()
    .and_then(|s| s.to_str())
    .unwrap_or("")
    .to_lowercase();
  let raw = is_raw_ext(&ext);
  if raw {
    // RAW: assume disponível (jpgfromraw-lib tenta async no thumbnail).
    meta.preview_available = true;
  } else {
    meta.preview_available = extract_preview_bytes_sync(path)
      .map(|b| !b.is_empty())
      .unwrap_or(false);
  }

  Ok(meta)
}

pub struct ExifBasic {
  pub dims: Option<(i64, i64)>,
  pub camera: String,
  pub taken_at: Option<String>,
  pub orientation: u16,
}

pub fn read_exif_basic(path: &Path) -> ExifBasic {
  let mut result = ExifBasic {
    dims: None,
    camera: String::new(),
    taken_at: None,
    orientation: 1,
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
    // Orientation (0x0112): 1-8. Sem ele, assume 1 (normal).
    if let Some(f) = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY) {
      if let Some(v) = f.value.get_uint(0) {
        result.orientation = v.clamp(1, 8) as u16;
      }
    }
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
/// tags JPEGInterchangeFormat (offset) and JPEGInterchangeFormatLength (size),
/// ou via parser CR3 (Canon, container BMFF/HEIF).
///
/// Iterates over ALL IFDs (primary + thumbnail/sub) — the embedded full-size
/// JPEG is often located in a SubIFD/thumbnail IFD, so we scan every field.
/// Works for NEF/ARW/DNG (TIFF-based). For CR3/HEIF usa o parser BMFF.
pub fn read_embedded_jpeg(path: &Path) -> Option<Vec<u8>> {
  if !path.exists() {
    return None;
  }
  let file = File::open(path).ok()?;
  let mut data = Vec::new();
  let mut reader = BufReader::new(file);
  reader.read_to_end(&mut data).ok()?;

  // CR3 (Canon, HEIF container): parser BMFF dedicado.
  if crate::cr3::looks_like_cr3(&data) {
    if let Some(jpeg) = crate::cr3::extract_cr3_preview(&data) {
      return Some(jpeg);
    }
    // Se não achou via parser BMFF, tenta o caminho TIFF (fallback).
  }

  let mut cursor = std::io::Cursor::new(&data);
  let exif = exif::Reader::new().read_from_container(&mut cursor).ok()?;

  // Find offset/length pairs anywhere in the field set (any IFD).
  let mut offset: Option<u64> = None;
  let mut length: Option<usize> = None;
  for field in exif.fields() {
    match field.tag {
      exif::Tag::JPEGInterchangeFormat => {
        offset = field.value.get_uint(0).map(|v| v as u64);
      }
      exif::Tag::JPEGInterchangeFormatLength => {
        length = field.value.get_uint(0).map(|v| v as usize);
      }
      _ => {}
    }
    // If we've found both in this pass, we can stop early only if they're
    // in the same IFD — to be safe, continue scanning the whole set, keeping
    // the last seen pair (they are usually adjacent in the same IFD).
  }

  let (start, len) = match (offset, length) {
    (Some(s), Some(l)) if l > 0 && (s as usize).checked_add(l)? <= data.len() => (s as usize, l),
    _ => return None,
  };
  // Validate that the slice actually starts with a JPEG SOI marker.
  if data[start..start + 2] != [0xFF, 0xD8] {
    return None;
  }
  Some(data[start..start + len].to_vec())
}

fn is_raw_ext(ext: &str) -> bool {
  matches!(
    ext,
    "cr3" | "cr2" | "crw" | "nef" | "nrw" | "arw" | "sr2" | "raf" | "rw2" | "dng" | "orf" | "pef"
      | "x3f" | "3fr" | "fff" | "mef" | "iiq" | "raw" | "rwl" | "mos"
  )
}

fn extract_preview_bytes_sync(path: &Path) -> Result<Vec<u8>, String> {
  // Strategy 1: embedded JPEG preview from RAW/TIFF via exif offsets
  // (works for NEF/ARW/DNG/CR2 and other TIFF-based RAW formats).
  if let Some(jpeg) = read_embedded_jpeg(path) {
    return Ok(jpeg);
  }
  // Strategy 2: full-file decode for normal image formats.
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

/// Lê a orientação EXIF (0x0112) de um buffer JPEG/TIFF (1..8).
fn buffer_orientation(bytes: &[u8]) -> image::metadata::Orientation {
  let mut cursor = std::io::Cursor::new(bytes);
  let ori = exif::Reader::new()
    .read_from_container(&mut cursor)
    .ok()
    .and_then(|exif| {
      exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
    })
    .unwrap_or(1);
  match ori {
    2 => image::metadata::Orientation::FlipHorizontal,
    3 => image::metadata::Orientation::Rotate180,
    4 => image::metadata::Orientation::FlipVertical,
    5 => image::metadata::Orientation::Rotate90AndFlipHorizontal,
    6 => image::metadata::Orientation::Rotate90,
    7 => image::metadata::Orientation::Rotate90AndFlipVertical,
    8 => image::metadata::Orientation::Rotate270,
    _ => image::metadata::Orientation::Normal,
  }
}

/// Gera um thumbnail (default max 256px) como JPEG bytes a partir de uma foto.
fn thumbnail_from_jpeg(bytes: &[u8], max_dim: u32) -> Result<Vec<u8>, String> {
  let ori = buffer_orientation(bytes);
  let img = ImageReader::new(std::io::Cursor::new(bytes))
    .with_guessed_format()
    .map_err(|e| e.to_string())?
    .decode()
    .map_err(|e| e.to_string())?;
  let img = img.apply_orientation(ori);
  let thumb = img.thumbnail(max_dim, max_dim);
  let mut out = std::io::Cursor::new(Vec::new());
  thumb
    .write_to(&mut out, image::ImageFormat::Jpeg)
    .map_err(|e| e.to_string())?;
  Ok(out.into_inner())
}

pub fn thumbnail_base64(path: &Path, max_dim: u32) -> Result<String, String> {
  let bytes = extract_preview_bytes_sync(path)?;
  let thumb = thumbnail_from_jpeg(&bytes, max_dim)?;
  Ok(format!(
    "data:image/jpeg;base64,{}",
    base64::engine::general_purpose::STANDARD.encode(thumb)
  ))
}
