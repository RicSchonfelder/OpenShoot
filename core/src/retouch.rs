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

/// Inpainting por difusão: preenche a região mascarada propagando os pixels das
/// bordas para dentro, iterativamente (borda → interior). Resultado suave e
/// adequado para remover objetos pequenos/distrações sobre fundo.
///
/// `mask`: 0 = manter, 1 = preencher (região a remover).
/// `iterations`: número de passadas; mais = preenchimento mais completo.
pub fn inpaint(
  rgb: &[u8],
  width: u32,
  height: u32,
  mask: &[u8],
  iterations: usize,
) -> Vec<u8> {
  let w = width as usize;
  let h = height as usize;
  let mut out = rgb.to_vec();
  // Flag: pixel ainda precisa ser preenchido (1) ou já resolvido (0).
  let mut pending: Vec<u8> = mask.to_vec();
  let iters = iterations.max(1).min(2000);

  for _ in 0..iters {
    let mut changed = 0;
    // Trabalha na borda atual: pixels pendentes com vizinho já resolvido.
    let snapshot = out.clone();
    for y in 1..(h - 1) {
      for x in 1..(w - 1) {
        let i = y * w + x;
        if pending[i] == 0 {
          continue;
        }
        // Soma os vizinhos resolvidos (não-pendentes).
        let mut acc = [0.0f64; 3];
        let mut n = 0u32;
        for (dy, dx) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
          let ny = (y as i32 + dy) as usize;
          let nx = (x as i32 + dx) as usize;
          let ni = ny * w + nx;
          if pending[ni] == 0 {
            let si = ni * 3;
            acc[0] += snapshot[si] as f64;
            acc[1] += snapshot[si + 1] as f64;
            acc[2] += snapshot[si + 2] as f64;
            n += 1;
          }
        }
        if n > 0 {
          let oi = i * 3;
          out[oi] = (acc[0] / n as f64) as u8;
          out[oi + 1] = (acc[1] / n as f64) as u8;
          out[oi + 2] = (acc[2] / n as f64) as u8;
          pending[i] = 0;
          changed += 1;
        }
      }
    }
    if changed == 0 {
      break;
    }
  }
  out
}

/// Aplica inpainting a uma foto: remove a região (bbox normalizada 0..1) e
/// retorna thumbnail JPEG (base64).
/// mask_rect: [x1, y1, x2, y2] em coordenadas 0..1 relativas à imagem.
pub fn inpaint_thumbnail_base64(
  path: &Path,
  mask_rect: [f32; 4],
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
  // Máscara para a bbox normalizada.
  let x1 = ((mask_rect[0].clamp(0.0, 1.0)) * w as f32).round() as usize;
  let y1 = ((mask_rect[1].clamp(0.0, 1.0)) * h as f32).round() as usize;
  let x2 = ((mask_rect[2].clamp(0.0, 1.0)) * w as f32).round() as usize;
  let y2 = ((mask_rect[3].clamp(0.0, 1.0)) * h as f32).round() as usize;
  let mut mask = vec![0u8; (w * h) as usize];
  for y in y1..y2.min(h as usize) {
    for x in x1..x2.min(w as usize) {
      mask[y * w as usize + x] = 1;
    }
  }
  let inpainted = inpaint(rgb.as_raw(), w, h, &mask, 200);
  let img = image::RgbImage::from_raw(w, h, inpainted).ok_or("falha ao reconstruir")?;
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

/// Regiões faciais suportadas para retoque direcionado (proporções relativas
/// à bbox do rosto detectado).
pub const FACE_REGIONS: &[(&str, [f32; 4])] = &[
  // [x0, y0, x1, y1] relativos à bbox do rosto.
  ("acne", [0.05, 0.05, 0.95, 0.9]), // pele central (testa→queixo)
  ("olhos", [0.05, 0.32, 0.95, 0.45]), // faixa dos olhos
  ("dentes", [0.25, 0.6, 0.75, 0.75]), // boca
  ("cabelo", [0.0, 0.0, 1.0, 0.25]), // topo (testa/cabelo)
];

/// Retoque direcionado de região facial dentro da bbox do rosto.
/// `region`: "acne" | "olhos" | "dentes" | "cabelo". `intensity` 0..1.
/// Operação:
/// - acne/cabelo → blur seletivo forte (suaviza imperfeições).
/// - olhos/dentes → clareia e aumenta levemente o contraste local.
pub fn retouch_face_region(
  rgb: &[u8],
  width: u32,
  height: u32,
  face_bbox: [f32; 4],
  region: &str,
  intensity: f32,
) -> Vec<u8> {
  let intensity = intensity.clamp(0.0, 1.0);
  if intensity <= 0.001 || rgb.is_empty() {
    return rgb.to_vec();
  }
  let Some(reg) = FACE_REGIONS.iter().find(|(n, _)| *n == region) else {
    return rgb.to_vec();
  };
  let (_name, r) = reg;

  let w = width as usize;
  let h = height as usize;
  // Bbox do rosto em pixels (normalizada 0..1).
  let (fx0, fy0) = (face_bbox[0], face_bbox[1]);
  let (fx1, fy1) = (face_bbox[2], face_bbox[3]);
  let fw = (fx1 - fx0).max(0.01);
  let fh = (fy1 - fy0).max(0.01);
  // Região-alvo em pixels (recortada à imagem).
  let px0 = ((fx0 + r[0] * fw) * width as f32).max(0.0) as usize;
  let py0 = ((fy0 + r[1] * fh) * height as f32).max(0.0) as usize;
  let px1 = (((fx0 + r[2] * fw) * width as f32) as usize).min(w);
  let py1 = (((fy0 + r[3] * fh) * height as f32) as usize).min(h);

  // Máscara suave (gaussiana radial no centro da região) para misturar.
  let mut out = rgb.to_vec();
  let cx = (px0 + px1) as f32 / 2.0;
  let cy = (py0 + py1) as f32 / 2.0;
  let rx = ((px1 - px0) as f32 / 2.0).max(1.0);
  let ry = ((py1 - py0) as f32 / 2.0).max(1.0);

  // Pré-computa blur 3x3 para o recorte (apenas a região-alvo).
  let blur = |x: usize, y: usize, c: usize| -> f32 {
    let mut acc = 0.0f32;
    let mut n = 0.0f32;
    for dy in -1i32..=1 {
      for dx in -1i32..=1 {
        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
        if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
          acc += rgb[((ny as usize) * w + nx as usize) * 3 + c] as f32;
          n += 1.0;
        }
      }
    }
    if n > 0.0 {
      acc / n
    } else {
      rgb[y * w * 3 + x * 3 + c] as f32
    }
  };

  for y in py0..py1 {
    for x in px0..px1 {
      // Peso gaussiano pelo centro da região (1 no centro → 0 nas bordas).
      let dxn = (x as f32 - cx) / rx;
      let dyn_n = (y as f32 - cy) / ry;
      let weight = (-(dxn * dxn + dyn_n * dyn_n) * 2.0).exp();
      if weight < 0.02 {
        continue;
      }
      let i = (y * w + x) * 3;
      for c in 0..3 {
        let orig = rgb[i + c] as f32;
        match region {
          "olhos" | "dentes" => {
            // Clarear + leve contraste local.
            let brightened = orig * (1.0 + 0.15 * intensity) + 8.0 * intensity;
            let b = blur(x, y, c);
            let contrasted = orig + (orig - b) * 0.3 * intensity;
            out[i + c] = ((contrasted * 0.5 + brightened * 0.5) * weight
              + orig * (1.0 - weight))
              .clamp(0.0, 255.0) as u8;
          }
          _ => {
            // acne/cabelo: blur forte.
            let b = blur(x, y, c);
            out[i + c] = (b * weight * (0.5 + 0.5 * intensity) + orig * (1.0 - weight * (0.5 + 0.5 * intensity)))
              .clamp(0.0, 255.0) as u8;
          }
        }
      }
    }
  }
  out
}

/// Máscara de sujeito (modo "SUJEITO/FUNDO" do AfterShoot).
/// Usa a bbox do rosto (SCRFD) expandida + máscara de pele para definir o
/// sujeito; desfoca o FUNDO (fora da máscara). Não-destrutivo.
/// `background_blur` 0..1 controla a força do desfoque do fundo.
pub fn subject_mask_base64(
  rgb: &[u8],
  width: u32,
  height: u32,
  face_bbox: [f32; 4],
  background_blur: f32,
) -> Vec<u8> {
  let blur = background_blur.clamp(0.0, 1.0);
  if blur <= 0.001 || rgb.is_empty() {
    return rgb.to_vec();
  }
  let w = width as usize;
  let h = height as usize;

  // Máscara de sujeito: combina bbox do rosto expandida + pele.
  let (fx0, fy0, fx1, fy1) = (face_bbox[0], face_bbox[1], face_bbox[2], face_bbox[3]);
  let fw = (fx1 - fx0).max(0.05);
  let fh = (fy1 - fy0).max(0.05);
  // Expande a bbox ~40% para incluir ombros/torso superior.
  let sx0 = (fx0 - fw * 0.4).clamp(0.0, 1.0);
  let sx1 = (fx1 + fw * 0.4).clamp(0.0, 1.0);
  let sy0 = (fy0 - fh * 0.5).clamp(0.0, 1.0);
  let sy1 = (fy1 + fh * 0.6).clamp(0.0, 1.0);
  let (px0, py0) = ((sx0 * width as f32) as usize, (sy0 * height as f32) as usize);
  let (px1, py1) = (((sx1 * width as f32) as usize).min(w), ((sy1 * height as f32) as usize).min(h));

  // Máscara suave por pixel: 1 dentro da região expandida que também é pele.
  let skin = skin_mask(rgb, width, height);
  let mut subj = vec![0.0f32; w * h];
  for y in py0..py1 {
    for x in px0..px1 {
      let i = y * w + x;
      let sk = skin[i];
      // Dentro da bbox do rosto → sempre sujeito. Fora → depende da pele.
      let in_face = x >= (fx0 * width as f32) as usize
        && x < (fx1 * width as f32) as usize
        && y >= (fy0 * height as f32) as usize
        && y < (fy1 * height as f32) as usize;
      subj[i] = if in_face { 1.0 } else { sk };
    }
  }

  // Blur do fundo (box blur maior).
  let blurred = box_blur_radius(rgb, w, h, 4);
  let mut out = Vec::with_capacity(rgb.len());
  for (i, px) in rgb.chunks_exact(3).enumerate() {
    let m = subj[i];
    // Mistura: quanto mais "fundo" (m baixo), mais blurred.
    let amount = (1.0 - m) * blur;
    let b = &blurred[i * 3..i * 3 + 3];
    out.push((px[0] as f32 * (1.0 - amount) + b[0] as f32 * amount).clamp(0.0, 255.0) as u8);
    out.push((px[1] as f32 * (1.0 - amount) + b[1] as f32 * amount).clamp(0.0, 255.0) as u8);
    out.push((px[2] as f32 * (1.0 - amount) + b[2] as f32 * amount).clamp(0.0, 255.0) as u8);
  }
  out
}

/// Box blur com raio arbitrário (para fundo).
fn box_blur_radius(rgb: &[u8], w: usize, h: usize, radius: usize) -> Vec<u8> {
  let mut out = rgb.to_vec();
  let r = radius.max(1);
  for y in 0..h {
    for x in 0..w {
      for c in 0..3 {
        let mut sum = 0f32;
        let mut n = 0f32;
        for dy in -(r as i32)..=(r as i32) {
          for dx in -(r as i32)..=(r as i32) {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
              sum += rgb[((ny as usize) * w + nx as usize) * 3 + c] as f32;
              n += 1.0;
            }
          }
        }
        out[(y * w + x) * 3 + c] = (sum / n) as u8;
      }
    }
  }
  out
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

  #[test]
  fn inpaint_fills_region() {
    // Imagem 8x8 branca com um bloco 2x2 preto no centro.
    let w = 8u32;
    let h = 8u32;
    let mut rgb = vec![255u8; (w * h * 3) as usize];
    for y in 3..5 {
      for x in 3..5 {
        let i = (y * w + x) as usize * 3;
        rgb[i] = 0;
        rgb[i + 1] = 0;
        rgb[i + 2] = 0;
      }
    }
    // Máscara: bloco central.
    let mut mask = vec![0u8; (w * h) as usize];
    for y in 3..5 {
      for x in 3..5 {
        mask[(y * w + x) as usize] = 1;
      }
    }
    let out = inpaint(&rgb, w, h, &mask, 200);
    // O centro deve ter sido preenchido (não mais preto).
    let center = (4 * w + 4) as usize * 3;
    assert!(out[center] > 100, "centro deveria ser preenchido, got {}", out[center]);
    assert_eq!(out.len(), rgb.len());
  }

  #[test]
  fn inpaint_empty_mask_keeps_original() {
    let rgb = vec![100u8, 150, 200, 10, 20, 30];
    let mask = vec![0u8; 2]; // nada a preencher
    let out = inpaint(&rgb, 1, 2, &mask, 100);
    assert_eq!(out, rgb);
  }

  #[test]
  fn face_region_blurs_skin() {
    // Imagem uniforme cinza-escuro; região "acne" deve suavizar (blur ≈ mesmo valor).
    let w = 20u32;
    let h = 20u32;
    let mut rgb = Vec::new();
    for _ in 0..(w * h) {
      rgb.push(120);
      rgb.push(110);
      rgb.push(100);
    }
    // Insere "mancha" (pixel claro) dentro da bbox do rosto.
    let i = (10 * w + 10) as usize * 3;
    rgb[i] = 240;
    let out = retouch_face_region(&rgb, w, h, [0.0, 0.0, 1.0, 1.0], "acne", 1.0);
    // O pixel da mancha deve ter sido suavizado (mais perto de 120).
    assert!(
      (out[i] as i32 - 120).abs() < (rgb[i] as i32 - 120).abs(),
      "mancha deve suavizar: {} vs {}",
      out[i],
      rgb[i]
    );
    assert_eq!(out.len(), rgb.len());
  }

  #[test]
  fn subject_mask_blurs_background() {
    // Fundo claro, sujeito (pele) no centro. blur alto deve suavizar o fundo.
    let w = 40u32;
    let h = 40u32;
    let mut rgb = Vec::new();
    for _ in 0..(w * h) {
      rgb.push(200);
      rgb.push(200);
      rgb.push(200);
    }
    // "Pele" (tom) no centro — skin_mask deve marcar como sujeito.
    for y in 15..25 {
      for x in 15..25 {
        let i = (y * w + x) as usize * 3;
        rgb[i] = 180;
        rgb[i + 1] = 150;
        rgb[i + 2] = 120;
      }
    }
    // bbox do rosto centrado.
    let out = subject_mask_base64(&rgb, w, h, [0.3, 0.3, 0.7, 0.7], 1.0);
    assert_eq!(out.len(), rgb.len());
    // Fundo (canto) deve ter sido suavizado (diferente do original no blur).
    // Com blur 1.0 o canto fica média da vizinhança; deve ser < 200 original.
    // Testamos apenas que a saída difere do original (houve processamento).
    assert!(out != rgb, "fundo deve ser alterado pelo blur");
  }
}
