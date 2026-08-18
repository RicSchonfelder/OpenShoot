use std::path::Path;

use base64::Engine;

/// Motor de retoque básico.
///
/// v1: suavização de pele via detecção heurística de cor (espaço YCbCr) + blur
/// seletivo. Não-destrutivo: sempre opera sobre uma cópia, o original nunca muda.

/// Converte RGB (0..1) para YCbCr.
fn rgb_to_ycbcr(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
  let y = 0.299 * r + 0.587 * g + 0.114 * b;
  let cb = 128.0 + (-0.168736 * r - 0.331264 * g + 0.5 * b) * 255.0;
  let cr = 128.0 + (0.5 * r - 0.418688 * g - 0.081312 * b) * 255.0;
  (y * 255.0, cb, cr)
}

/// Probabilidade 0..1 de um pixel RGB ser pele (baseado em faixas YCbCr).
fn skin_probability(r: f32, g: f32, b: f32) -> f32 {
  let (y, cb, cr) = rgb_to_ycbcr(r, g, b);
  // Faixas clássicas de pele em YCbCr (Chai & Ngan).
  let in_cb = (77.0..=127.0).contains(&cb);
  let in_cr = (133.0..=173.0).contains(&cr);
  if !in_cb || !in_cr {
    return 0.0;
  }
  // Pontuação suave: proximidade ao centro das faixas.
  let cb_center = 102.0;
  let cr_center = 153.0;
  let d_cb = 1.0 - (cb - cb_center).abs() / 25.0;
  let d_cr = 1.0 - (cr - cr_center).abs() / 20.0;
  let base = (d_cb + d_cr) / 2.0;
  // Luminância muito escura/clara reduz a confiança.
  let lum = if (40.0..=245.0).contains(&y) { 1.0 } else { 0.3 };
  (base * lum).clamp(0.0, 1.0)
}

/// Gera máscara de pele (0..1) para uma imagem RGB8.
fn skin_mask(rgb: &[u8], width: u32, height: u32) -> Vec<f32> {
  let mut mask = vec![0.0f32; rgb.len() / 3];
  for (i, px) in rgb.chunks_exact(3).enumerate() {
    mask[i] = skin_probability(
      px[0] as f32 / 255.0,
      px[1] as f32 / 255.0,
      px[2] as f32 / 255.0,
    );
  }
  // Suaviza a máscara (3x3 blur) para evitar bordas duras.
  let w = width as usize;
  let h = height as usize;
  let mut out = mask.clone();
  for y in 1..(h - 1) {
    for x in 1..(w - 1) {
      let i = y * w + x;
      let mut acc = 0.0;
      acc += mask[i - w - 1] + mask[i - w] + mask[i - w + 1];
      acc += mask[i - 1] + mask[i] + mask[i + 1];
      acc += mask[i + w - 1] + mask[i + w] + mask[i + w + 1];
      out[i] = acc / 9.0;
    }
  }
  out
}

/// Aplica suavização de pele (blur seletivo por máscara).
/// intensity 0..1 controla a força. Retorna a imagem retocada RGB8.
pub fn retouch_skin(rgb: &[u8], width: u32, height: u32, intensity: f32) -> Vec<u8> {
  let intensity = intensity.clamp(0.0, 1.0);
  if intensity <= 0.001 || rgb.is_empty() {
    return rgb.to_vec();
  }
  let w = width as usize;
  let h = height as usize;
  let mask = skin_mask(rgb, width, height);

  // Cria versão borrada (box blur 5x5) da imagem.
  let radius = 2usize;
  let mut blurred = rgb.to_vec();
  for y in 0..h {
    for x in 0..w {
      let mut acc = [0.0f32; 3];
      let mut n = 0u32;
      for dy in -(radius as i32)..=(radius as i32) {
        for dx in -(radius as i32)..=(radius as i32) {
          let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as usize;
          let nx = (x as i32 + dx).clamp(0, w as i32 - 1) as usize;
          let si = (ny * w + nx) * 3;
          acc[0] += rgb[si] as f32;
          acc[1] += rgb[si + 1] as f32;
          acc[2] += rgb[si + 2] as f32;
          n += 1;
        }
      }
      let i = (y * w + x) * 3;
      blurred[i] = (acc[0] / n as f32) as u8;
      blurred[i + 1] = (acc[1] / n as f32) as u8;
      blurred[i + 2] = (acc[2] / n as f32) as u8;
    }
  }

  // Blend: out = orig + (blur - orig) * mask * intensity.
  let mut out = rgb.to_vec();
  for i in 0..mask.len() {
    let m = mask[i] * intensity;
    if m <= 0.001 {
      continue;
    }
    let bi = i * 3;
    for c in 0..3 {
      let orig = rgb[bi + c] as f32;
      let bl = blurred[bi + c] as f32;
      out[bi + c] = (orig + (bl - orig) * m).clamp(0.0, 255.0) as u8;
    }
  }
  out
}

/// Retoca uma foto do disco: carrega preview, aplica suavização de pele e
/// retorna thumbnail JPEG retocado (base64).
pub fn retouch_skin_thumbnail_base64(
  path: &Path,
  intensity: f32,
  max_dim: u32,
) -> Result<String, String> {
  let img = match crate::imageproc::read_embedded_jpeg(path) {
    Some(jpeg) => image::ImageReader::new(std::io::Cursor::new(jpeg))
      .with_guessed_format()
      .map_err(|e| e.to_string())?
      .decode()
      .map_err(|e| e.to_string())?,
    None => image::ImageReader::open(path)
      .map_err(|e| e.to_string())?
      .decode()
      .map_err(|e| e.to_string())?,
  };
  let rgb = img.to_rgb8();
  let (w, h) = rgb.dimensions();
  let retouched = retouch_skin(rgb.as_raw(), w, h, intensity);
  let img = image::RgbImage::from_raw(w, h, retouched).ok_or("falha ao reconstruir")?;
  let dynimg = image::DynamicImage::ImageRgb8(img);
  let thumb = dynimg.thumbnail(max_dim, max_dim);
  let mut out = std::io::Cursor::new(Vec::new());
  thumb
    .write_to(&mut out, image::ImageFormat::Jpeg)
    .map_err(|e| e.to_string())?;
  Ok(format!(
    "data:image/jpeg;base64,{}",
    base64::engine::general_purpose::STANDARD.encode(out.into_inner())
  ))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn skin_probability_high_for_skin_tone() {
    // Tom de pele típico (R>G>B, médio).
    let p = skin_probability(0.8, 0.6, 0.5);
    assert!(p > 0.5, "esperado alta probabilidade de pele, got {p}");
  }

  #[test]
  fn skin_probability_low_for_blue() {
    let p = skin_probability(0.0, 0.0, 1.0);
    assert!(p < 0.1, "azul não deve ser pele, got {p}");
  }

  #[test]
  fn skin_probability_low_for_green_grass() {
    let p = skin_probability(0.1, 0.6, 0.1);
    assert!(p < 0.3, "verde não deve ser pele, got {p}");
  }

  #[test]
  fn retouch_skin_keeps_size() {
    let w = 16u32;
    let h = 16u32;
    let rgb = vec![200u8; (w * h * 3) as usize];
    let out = retouch_skin(&rgb, w, h, 0.5);
    assert_eq!(out.len(), rgb.len());
  }

  #[test]
  fn zero_intensity_returns_original() {
    let rgb = vec![10u8, 20, 30, 40, 50, 60];
    let out = retouch_skin(&rgb, 1, 2, 0.0);
    assert_eq!(out, rgb);
  }
}
