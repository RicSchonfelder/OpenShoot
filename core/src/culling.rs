use image::ImageReader;
use std::io::Cursor;
use std::path::Path;

/// Escala a imagem para uma altura-alvo mantendo aspect ratio, retornando
/// (width, height, pixels em grayscale f32 0..1).
fn to_gray_luma(path: &Path, target_height: u32) -> Result<(u32, u32, Vec<f32>), String> {
  // Usa o preview embutido (veloz) se existir; senão tenta o arquivo.
  let img = match crate::imageproc::read_embedded_jpeg(path) {
    Some(jpeg) => ImageReader::new(Cursor::new(jpeg))
      .with_guessed_format()
      .map_err(|e| e.to_string())?
      .decode()
      .map_err(|e| e.to_string())?,
    None => ImageReader::open(path)
      .map_err(|e| e.to_string())?
      .decode()
      .map_err(|e| e.to_string())?,
  };
  gray_luma_from(&img, target_height)
}

/// Cinza redimensionado a partir de uma imagem já decodificada (decode único no culling).
fn gray_luma_from(img: &image::DynamicImage, target_height: u32) -> Result<(u32, u32, Vec<f32>), String> {
  let img = img.to_luma8();
  let (w, h) = img.dimensions();
  if h == 0 {
    return Err("altura zero".to_string());
  }
  let scale = target_height as f64 / h as f64;
  let tw = (w as f64 * scale).max(1.0) as u32;
  let resized = image::imageops::resize(&img, tw, target_height, image::imageops::FilterType::Triangle);
  let (rw, rh) = resized.dimensions();
  let mut out = Vec::with_capacity((rw * rh) as usize);
  for y in 0..rh {
    for x in 0..rw {
      out.push(resized.get_pixel(x, y)[0] as f32 / 255.0);
    }
  }
  Ok((rw, rh, out))
}

/// Variância do Laplacian (métrica clássica de nitidez).
/// Imagens desfocadas têm baixa variância. Kernel: 0 1 0 / 1 -4 1 / 0 1 0.
fn laplacian_variance(gray: &[f32], w: u32, h: u32) -> f64 {
  if w < 3 || h < 3 {
    return 0.0;
  }
  let wu = w as usize;
  let mut sum = 0.0f64;
  let mut sum_sq = 0.0f64;
  let mut n = 0u64;
  for y in 1..(h - 1) {
    for x in 1..(w - 1) {
      let i = (y as usize) * wu + (x as usize);
      let c = gray[i] * -4.0
        + gray[i - 1]
        + gray[i + 1]
        + gray[i - wu]
        + gray[i + wu];
      let v = c as f64;
      sum += v;
      sum_sq += v * v;
      n += 1;
    }
  }
  if n == 0 {
    return 0.0;
  }
  let mean = sum / n as f64;
  sum_sq / n as f64 - mean * mean
}

/// Percentual de pixels extremos (muito escuros ou muito claros).
/// Penaliza subexposição e superexposição.
fn exposure_score(gray: &[f32]) -> f64 {
  if gray.is_empty() {
    return 1.0;
  }
  let mut dark = 0.0;
  let mut bright = 0.0;
  for &p in gray {
    if p < 0.05 {
      dark += 1.0;
    } else if p > 0.95 {
      bright += 1.0;
    }
  }
  let dark_frac = dark / gray.len() as f64;
  let bright_frac = bright / gray.len() as f64;
  // Sem excesso de pretos/parados: queremos poucos extremos.
  (1.0 - (dark_frac + bright_frac)).max(0.0)
}

/// Distribuição da luminância: imagens com histograma espalhado (bom contraste)
/// tendem a ser melhores que lavadas/sólidas.
fn histogram_spread(gray: &[f32]) -> f64 {
  if gray.is_empty() {
    return 0.0;
  }
  let mut buckets = [0u64; 16];
  for &p in gray {
    let b = ((p * 15.999) as usize).min(15);
    buckets[b] += 1;
  }
  let n = gray.len() as f64;
  let mut filled = 0.0;
  for &b in &buckets {
    if (b as f64) / n > 0.01 {
      filled += 1.0;
    }
  }
  filled / 16.0
}

/// Score composto 0..100 para culling heurístico (sem rede neural).
/// Pesos: nitidez 50%, exposição 25%, contraste 25%.
pub fn heuristic_score(path: &Path, target_height: u32) -> Result<f64, String> {
  let (w, h, gray) = to_gray_luma(path, target_height)?;
  Ok(heuristic_from_gray(w, h, &gray))
}

/// Score composto a partir de um RGB já decodificado — evita re-decodificar
/// o arquivo no culling (decode único, gap G3 do roadmap).
pub fn heuristic_score_rgb(
  rgb: &[u8],
  width: u32,
  height: u32,
  target_height: u32,
) -> Result<f64, String> {
  let img = image::RgbImage::from_raw(width, height, rgb.to_vec())
    .ok_or_else(|| "dimensões incompatíveis com o buffer".to_string())?;
  let (w, h, gray) = gray_luma_from(&image::DynamicImage::ImageRgb8(img), target_height)?;
  Ok(heuristic_from_gray(w, h, &gray))
}

fn heuristic_from_gray(w: u32, h: u32, gray: &[f32]) -> f64 {
  let sharp = laplacian_variance(gray, w, h);
  let expo = exposure_score(gray);
  let spread = histogram_spread(gray);

  // Laplacian variance escala com o tamanho; normalizamos por um teto empírico.
  let sharp_norm = (sharp / 1200.0).min(1.0);
  let score = sharp_norm * 50.0 + expo * 25.0 + spread * 25.0;
  score.max(0.0).min(100.0)
}

/// Score de nitidez puro (para mostrar no UI).
#[allow(dead_code)]
pub fn sharpness(path: &Path, target_height: u32) -> Result<f64, String> {
  let (w, h, gray) = to_gray_luma(path, target_height)?;
  Ok(laplacian_variance(&gray, w, h))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn laplacian_flat_is_low() {
    // imagem uniforme -> variância ~0
    let w = 32u32;
    let h = 32u32;
    let gray = vec![0.5f32; (w * h) as usize];
    assert!(laplacian_variance(&gray, w, h) < 1e-6);
  }

  #[test]
  fn laplacian_noise_is_high() {
    let w = 64u32;
    let h = 64u32;
    let mut gray = vec![0.5f32; (w * h) as usize];
    // ruído aleatório determinístico
    let mut seed = 42u64;
    for v in gray.iter_mut() {
      seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
      let r = ((seed >> 33) & 0xFFFF) as f64 / 65535.0;
      *v = 0.5 + ((r - 0.5) * 0.2) as f32;
    }
    let v = laplacian_variance(&gray, w, h);
    assert!(v > 0.001, "esperado variancia > 0, got {v}");
  }

  #[test]
  fn exposure_penalizes_extremes() {
    let dark = vec![0.0f32; 100];
    assert!(exposure_score(&dark) < 0.1);
    let neutral = vec![0.5f32; 100];
    assert!((exposure_score(&neutral) - 1.0).abs() < 0.01);
  }
}
