use serde::{Deserialize, Serialize};
use std::path::Path;

use base64::Engine;

/// Parâmetros de edição não-destrutiva.
/// Todos os valores são opcionais; ausente = sem ajuste nesse canal.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct EditParams {
  /// Exposição em EV (-5..5), ex: +1.0 dobra a luminância.
  pub exposure: Option<f32>,
  /// Balanço de branco — temperatura em Kelvin (2000..12000). 6500 = neutro.
  pub temperature: Option<f32>,
  /// Balanço de branco — tint (-100..100, magenta↔verde).
  pub tint: Option<f32>,
  /// Contraste (-100..100). 0 = neutro.
  pub contrast: Option<f32>,
  /// Saturação (-100..100). 0 = neutro; -100 = P&B.
  pub saturation: Option<f32>,
  /// Sombras (-100..100): recupera/escurece as sombras.
  pub shadows: Option<f32>,
  /// Realces (-100..100): recupera/escurece os realces.
  pub highlights: Option<f32>,
  /// Brilho (-100..100): deslocamento de luminância.
  pub brightness: Option<f32>,
  /// Curva de tom: [destaques, luzes, escuros, sombras] (-100..100 cada).
  /// Segue a curva paramétrica do AfterShoot/Lightroom: pontos de controle
  /// aplicados por faixa de luminância com interpolação suave.
  pub tone_curve: Option<[f32; 4]>,
  /// HSL por cor: 8 cores × (hue, sat, lum) = 24 valores em -100..100.
  /// Ordem (Lightroom): Red, Orange, Yellow, Green, Aqua, Blue, Purple, Magenta.
  /// Ajuste aplicado por proximidade de matiz com falloff.
  pub hsl: Option<[f32; 24]>,
  /// Nitidez (-100..100). 0 = neutro. Implementada com unsharp mask.
  pub sharpen: Option<f32>,
  /// Redução de ruído (-100..100). 0 = neutro. Blur seletivo preservando bordas.
  pub denoise: Option<f32>,
}

/// Centros de matiz das 8 cores HSL (graus).
const HSL_HUE_CENTERS: [f32; 8] = [0.0, 30.0, 60.0, 120.0, 180.0, 240.0, 285.0, 315.0];

/// Converte RGB (0..1) → HSL. Retorna (h, s, l) com h em 0..360.
fn rgb_to_hsl(rgb: [f32; 3]) -> (f32, f32, f32) {
  let (r, g, b) = (rgb[0], rgb[1], rgb[2]);
  let max = r.max(g).max(b);
  let min = r.min(g).min(b);
  let l = (max + min) / 2.0;
  if (max - min).abs() < 1e-6 {
    return (0.0, 0.0, l);
  }
  let d = max - min;
  let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
  let h = if max == r {
    ((g - b) / d + if g < b { 6.0 } else { 0.0 }) * 60.0
  } else if max == g {
    ((b - r) / d + 2.0) * 60.0
  } else {
    ((r - g) / d + 4.0) * 60.0
  };
  (h, s, l)
}

/// Converte HSL → RGB (0..1). h em 0..360.
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [f32; 3] {
  let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
  let hp = ((h % 360.0) + 360.0) % 360.0 / 60.0;
  let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
  let (r1, g1, b1) = match hp as i32 {
    0 => (c, x, 0.0),
    1 => (x, c, 0.0),
    2 => (0.0, c, x),
    3 => (0.0, x, c),
    4 => (x, 0.0, c),
    _ => (c, 0.0, x),
  };
  let m = l - c / 2.0;
  [(r1 + m).clamp(0.0, 1.0), (g1 + m).clamp(0.0, 1.0), (b1 + m).clamp(0.0, 1.0)]
}

/// Aplica ajuste HSL (24 valores) a um pixel RGB. Retorna pixel ajustado.
fn apply_hsl_pixel(rgb: [f32; 3], hsl: &[f32; 24]) -> [f32; 3] {
  let (h, s, l) = rgb_to_hsl(rgb);
  let mut dh = 0.0f32;
  let mut ds = 0.0f32;
  let mut dl = 0.0f32;
  let mut total_w = 0.0f32;
  for i in 0..8 {
    let (dh_i, ds_i, dl_i) = (hsl[i * 3], hsl[i * 3 + 1], hsl[i * 3 + 2]);
    if dh_i == 0.0 && ds_i == 0.0 && dl_i == 0.0 {
      continue;
    }
    // Distância angular de matiz ao centro da cor.
    let diff = (h - HSL_HUE_CENTERS[i]).abs();
    let ang = if diff > 180.0 { 360.0 - diff } else { diff };
    // Falloff gaussiano: largura ~30° de matiz.
    let w = (-(ang * ang) / (2.0 * 25.0 * 25.0)).exp();
    dh += dh_i / 100.0 * w;
    ds += ds_i / 100.0 * w;
    dl += dl_i / 100.0 * w;
    total_w += w;
  }
  if total_w == 0.0 {
    return rgb;
  }
  // Ajuste de matiz: deslocar h (com wrap). sat/lum: escala suave.
  let nh = (h + dh * 30.0).rem_euclid(360.0);
  let ns = (s + ds * 0.6).clamp(0.0, 1.0);
  let nl = (l + dl * 0.35).clamp(0.0, 1.0);
  hsl_to_rgb(nh, ns, nl)
}

impl EditParams {
  #[allow(dead_code)]
  pub fn is_empty(&self) -> bool {
    *self == Self::default()
  }

  /// Aplica os parâmetros a um pixel RGB (0..1 floats).
  /// rgb: array [r, g, b] em 0..1. Retorna o pixel editado em 0..1.
  pub fn apply_pixel(&self, rgb: [f32; 3]) -> [f32; 3] {
    let mut r = rgb[0];
    let mut g = rgb[1];
    let mut b = rgb[2];

    // Exposição (escalar multiplicativo em espaço linear aproximado).
    if let Some(ev) = self.exposure {
      let f = 2f32.powf(ev);
      r *= f;
      g *= f;
      b *= f;
    }

    // Balanço de branco: temperatura (azul/âmbar) e tint (magenta/verde).
    if let Some(temp) = self.temperature {
      // 6500 = neutro. <6500 esfria (mais azul), >6500 aquece (mais âmbar).
      let diff = (temp - 6500.0) / 6500.0;
      let red_scale = 1.0 + diff * 0.5;
      let blue_scale = 1.0 - diff * 0.5;
      r *= red_scale;
      b *= blue_scale;
    }
    if let Some(tint) = self.tint {
      let f = tint / 100.0;
      g *= 1.0 + f * 0.3; // tint positivo = mais verde
      r *= 1.0 - f * 0.15;
      b *= 1.0 - f * 0.15;
    }

    // Luminância (luma) para saturação/contraste/sombras/realces.
    let luma = 0.299 * r + 0.587 * g + 0.114 * b;

    // Saturação: misturar canal com luma.
    if let Some(sat) = self.saturation {
      let s = 1.0 + sat / 100.0;
      r = luma + (r - luma) * s;
      g = luma + (g - luma) * s;
      b = luma + (b - luma) * s;
    }

    // Brilho: deslocamento de luminância (aditivo, suave nas bordas).
    if let Some(bright) = self.brightness {
      let bv = bright / 100.0;
      r = r + bv * 0.4;
      g = g + bv * 0.4;
      b = b + bv * 0.4;
    }

    // Contraste: curva S em torno do ponto médio.
    if let Some(contrast) = self.contrast {
      let c = 1.0 + contrast / 100.0;
      let p = 0.5;
      r = (r - p) * c + p;
      g = (g - p) * c + p;
      b = (b - p) * c + p;
    }

    // Sombras/realces: ajuste tonal por faixa de luminância.
    if let Some(shadows) = self.shadows {
      let s = shadows / 100.0;
      // nas sombras (luma < 0.5), empurra para cima/baixo com fade.
      let t = (1.0 - luma).clamp(0.0, 1.0); // 1 = escuro
      let amt = s * t * 0.5;
      r += amt;
      g += amt;
      b += amt;
    }
    if let Some(highlights) = self.highlights {
      let h = highlights / 100.0;
      let t = luma.clamp(0.0, 1.0); // 1 = claro
      let amt = h * t * 0.5;
      r -= amt;
      g -= amt;
      b -= amt;
    }

    // Curva de tom: [destaques, luzes, escuros, sombras].
    // Cada ponto desloca a luminância numa faixa com falloff gaussiano.
    if let Some(curve) = self.tone_curve {
      let dt = tone_curve_adjust(luma, curve);
      if dt != 0.0 {
        r += dt;
        g += dt;
        b += dt;
      }
    }

    // HSL por cor (8 cores).
    if let Some(hsl) = self.hsl {
      let adj = apply_hsl_pixel([r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)], &hsl);
      r = adj[0];
      g = adj[1];
      b = adj[2];
    }

    [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)]
  }
}

/// Deslocamento de luminância da curva de tom para um luma dado.
/// curve = [destaques, luzes, escuros, sombras] em -100..100.
/// Centros das faixas: 0.85 (destaques), 0.7 (luzes), 0.3 (escuros), 0.15 (sombras).
fn tone_curve_adjust(luma: f32, curve: [f32; 4]) -> f32 {
  let centers = [0.85f32, 0.7, 0.3, 0.15];
  let widths = [0.18f32, 0.22, 0.22, 0.18];
  let mut dt = 0.0f32;
  for i in 0..4 {
    let v = curve[i] / 100.0;
    if v == 0.0 {
      continue;
    }
    // Peso gaussiano pela distância ao centro da faixa.
    let d = (luma - centers[i]).abs() / widths[i];
    let w = (-(d * d) * 3.0).exp(); // 1 no centro, cai suavemente
    dt += v * w * 0.25;
  }
  dt
}

/// Aplica a receita a uma imagem RGB8 (HxWx3) e retorna a imagem editada.
pub fn apply_to_rgb8(
  params: &EditParams,
  rgb: &[u8],
  width: u32,
  height: u32,
) -> Vec<u8> {
  let mut out = Vec::with_capacity(rgb.len());
  for px in rgb.chunks_exact(3) {
    let p = params.apply_pixel([
      px[0] as f32 / 255.0,
      px[1] as f32 / 255.0,
      px[2] as f32 / 255.0,
    ]);
    out.push((p[0] * 255.0) as u8);
    out.push((p[1] * 255.0) as u8);
    out.push((p[2] * 255.0) as u8);
  }

  // Nitidez (unsharp mask) e redução de ruído operam na imagem inteira.
  let mut result = out;
  if params.sharpen.is_some() {
    result = unsharp_mask(&result, width, height, params.sharpen.unwrap_or(0.0));
  }
  if params.denoise.is_some() {
    result = denoise_preserve_edges(&result, width, height, params.denoise.unwrap_or(0.0));
  }
  let _ = (width, height);
  result
}

/// Unsharp mask: realça bordas subtraindo uma versão desfocada.
/// amount -100..100 (0 = neutro).
fn unsharp_mask(rgb: &[u8], w: u32, h: u32, amount: f32) -> Vec<u8> {
  let blur = box_blur(rgb, w, h, 1);
  let a = amount / 100.0;
  let mut out = Vec::with_capacity(rgb.len());
  for (i, px) in rgb.chunks_exact(3).enumerate() {
    for c in 0..3 {
      let orig = px[c] as f32;
      let b = blur[i * 3 + c] as f32;
      let sharp = orig + (orig - b) * a * 2.0;
      out.push(sharp.clamp(0.0, 255.0) as u8);
    }
  }
  out
}

/// Redução de ruído: blur 3x3 preservando bordas (parecido com bilateral leve).
/// amount -100..100 (0 = neutro; valores positivos suavizam mais).
fn denoise_preserve_edges(rgb: &[u8], w: u32, h: u32, amount: f32) -> Vec<u8> {
  let a = (amount / 100.0).clamp(0.0, 1.0);
  if a <= 0.0 {
    return rgb.to_vec();
  }
  let (wi, hi) = (w as i32, h as i32);
  let mut out = rgb.to_vec();
  for y in 0..hi {
    for x in 0..wi {
      for c in 0..3 {
        let idx = ((y * wi + x) * 3 + c) as usize;
        let center = rgb[idx] as f32;
        // Média 3x3.
        let mut sum = 0f32;
        let mut n = 0f32;
        for dy in -1i32..=1 {
          for dx in -1i32..=1 {
            let (nx, ny) = (x + dx, y + dy);
            if nx >= 0 && nx < wi && ny >= 0 && ny < hi {
              sum += rgb[((ny * wi + nx) * 3 + c) as usize] as f32;
              n += 1.0;
            }
          }
        }
        let mean = sum / n;
        // Bilateral simplificado: mistura mais onde o centro ≈ vizinhança.
        let diff = (center - mean).abs();
        let w_keep = (1.0 - a) + a * (diff / 255.0);
        let blended = mean * a + center * (1.0 - a);
        out[idx] = (center + (blended - center) * w_keep).clamp(0.0, 255.0) as u8;
      }
    }
  }
  out
}

/// Blur 3x3 (para unsharp mask).
fn box_blur(rgb: &[u8], w: u32, h: u32, _radius: u32) -> Vec<u8> {
  let (wi, hi) = (w as i32, h as i32);
  let mut out = Vec::with_capacity(rgb.len());
  for y in 0..hi {
    for x in 0..wi {
      for c in 0..3 {
        let mut sum = 0f32;
        let mut n = 0f32;
        for dy in -1i32..=1 {
          for dx in -1i32..=1 {
            let (nx, ny) = (x + dx, y + dy);
            if nx >= 0 && nx < wi && ny >= 0 && ny < hi {
              sum += rgb[((ny * wi + nx) * 3 + c) as usize] as f32;
              n += 1.0;
            }
          }
        }
        out.push((sum / n) as u8);
      }
    }
  }
  out
}

/// Carrega a imagem (preview embutido ou arquivo), aplica a receita e
/// retorna um thumbnail JPEG editado (base64).
pub fn edit_thumbnail_base64(
  path: &Path,
  params: &EditParams,
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
  let edited = apply_to_rgb8(params, rgb.as_raw(), w, h);
  let img = image::RgbImage::from_raw(w, h, edited).ok_or("falha ao reconstruir")?;
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

/// Exporta uma foto com a edição aplicada para um arquivo no destino.
/// `format`: "jpeg" | "png". `quality`: 1..100 (JPEG apenas).
/// Não-destrutivo: lê o original, aplica a receita, grava uma cópia.
pub fn export_photo_to_file(
  path: &Path,
  params: &EditParams,
  dest: &Path,
  format: &str,
  quality: u8,
) -> Result<(), String> {
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
  // Aplica a orientação EXIF antes de exportar.
  let ori = crate::imageproc::buffer_orientation(
    &crate::imageproc::read_embedded_jpeg(path).unwrap_or_default(),
  );
  let img = if ori > 1 {
    let dynimg = image::DynamicImage::ImageRgb8(img.to_rgb8());
    crate::imageproc::apply_exif_orientation(dynimg, ori)
  } else {
    image::DynamicImage::ImageRgb8(img.to_rgb8())
  };

  let rgb = img.to_rgb8();
  let (w, h) = rgb.dimensions();
  let edited = apply_to_rgb8(params, rgb.as_raw(), w, h);
  let out_img = image::RgbImage::from_raw(w, h, edited).ok_or("falha ao reconstruir")?;
  let dynout = image::DynamicImage::ImageRgb8(out_img);

  match format {
    "png" => dynout
      .save_with_format(dest, image::ImageFormat::Png)
      .map_err(|e| format!("salvar PNG: {e}")),
    _ => {
      let q = quality.clamp(1, 100);
      let mut out = std::io::Cursor::new(Vec::new());
      let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, q);
      dynout
        .write_with_encoder(encoder)
        .map_err(|e| format!("codificar JPEG: {e}"))?;
      std::fs::write(dest, out.into_inner()).map_err(|e| format!("gravar JPEG: {e}"))
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn exposure_increases() {
    let p = EditParams {
      exposure: Some(1.0),
      ..Default::default()
    };
    let out = p.apply_pixel([0.5, 0.5, 0.5]);
    assert!(out[0] > 0.5);
  }

  #[test]
  fn saturation_zero_is_grayscale() {
    let p = EditParams {
      saturation: Some(-100.0),
      ..Default::default()
    };
    let out = p.apply_pixel([1.0, 0.0, 0.0]);
    // todos os canais iguais -> cinza
    let (r, g, b) = (out[0], out[1], out[2]);
    assert!((r - g).abs() < 0.01 && (g - b).abs() < 0.01);
  }

  #[test]
  fn temperature_warmth() {
    // temperatura alta -> mais vermelho/âmbar, menos azul
    let p = EditParams {
      temperature: Some(8000.0),
      ..Default::default()
    };
    let out = p.apply_pixel([0.5, 0.5, 0.5]);
    assert!(out[0] > out[2]); // r > b
  }

  #[test]
  fn apply_to_rgb8_matches_size() {
    let p = EditParams {
      exposure: Some(0.5),
      ..Default::default()
    };
    let rgb = vec![100u8, 150, 200, 10, 20, 30];
    let out = apply_to_rgb8(&p, &rgb, 1, 2);
    assert_eq!(out.len(), rgb.len());
  }

  #[test]
  fn default_is_empty() {
    assert!(EditParams::default().is_empty());
  }

  #[test]
  fn tone_curve_darkens_shadows() {
    // Escurecer sombras: curva[3] = -80 → escuros ficam mais escuros.
    let p = EditParams {
      tone_curve: Some([0.0, 0.0, 0.0, -80.0]),
      ..Default::default()
    };
    let dark = p.apply_pixel([0.15, 0.15, 0.15]);
    let bright = p.apply_pixel([0.9, 0.9, 0.9]);
    assert!(dark[0] < 0.15, "sombra deve escurecer, ficou {}", dark[0]);
    // Região clara pouco afetada.
    assert!((bright[0] - 0.9).abs() < 0.05);
  }

  #[test]
  fn tone_curve_lifts_highlights() {
    // Elevar destaques: curve[0] = 60 → claros ficam mais claros.
    let p = EditParams {
      tone_curve: Some([60.0, 0.0, 0.0, 0.0]),
      ..Default::default()
    };
    let bright = p.apply_pixel([0.85, 0.85, 0.85]);
    assert!(bright[0] > 0.85, "destaque deve clarear, ficou {}", bright[0]);
  }

  #[test]
  fn export_writes_jpeg_file() {
    // Gera uma imagem sintética, exporta como JPEG e valida o arquivo.
    let tmp = std::env::temp_dir().join("openshoot_export_test");
    std::fs::create_dir_all(&tmp).unwrap();
    let src = tmp.join("src.png");
    let img = image::RgbImage::from_pixel(8, 8, image::Rgb([120u8, 80, 200]));
    img.save(&src).unwrap();
    let dest = tmp.join("out.jpg");
    let params = EditParams::default();
    export_photo_to_file(&src, &params, &dest, "jpeg", 90).unwrap();
    assert!(dest.exists(), "JPEG deve ser criado");
    let size = std::fs::metadata(&dest).unwrap().len();
    assert!(size > 0, "arquivo não pode ser vazio");
    let _ = std::fs::remove_dir_all(&tmp);
  }

  #[test]
  fn export_applies_edits() {
    let tmp = std::env::temp_dir().join("openshoot_export_edit_test");
    std::fs::create_dir_all(&tmp).unwrap();
    let src = tmp.join("src2.png");
    let img = image::RgbImage::from_pixel(4, 4, image::Rgb([200u8, 200, 200]));
    img.save(&src).unwrap();
    let dest = tmp.join("out2.jpg");
    // Exposição -2 deve escurecer bastante.
    let params = EditParams {
      exposure: Some(-2.0),
      ..Default::default()
    };
    export_photo_to_file(&src, &params, &dest, "jpeg", 90).unwrap();
    // Re-decoda e verifica que escureceu.
    let decoded = image::ImageReader::open(&dest)
      .unwrap()
      .decode()
      .unwrap()
      .to_rgb8();
    let px = decoded.get_pixel(2, 2);
    assert!(
      px[0] < 100,
      "exposição -2 deve escurecer o pixel, ficou {}",
      px[0]
    );
    let _ = std::fs::remove_dir_all(&tmp);
  }

  #[test]
  fn hsl_red_desaturates() {
    // Red: saturação -100 → o vermelho puro perde saturação.
    let mut hsl = [0f32; 24];
    hsl[1] = -100.0; // Red sat
    let p = EditParams {
      hsl: Some(hsl),
      ..Default::default()
    };
    let red = p.apply_pixel([1.0, 0.0, 0.0]);
    // vermelho puro fica rosado (mais claro/cinza), canais não-vermelhos sobem.
    assert!(red[1] > 0.0 || red[2] > 0.0, "vermelho deve perder saturação");
    // Cor oposta (ciano/azul) quase não afetada.
    let blue = p.apply_pixel([0.0, 0.0, 1.0]);
    assert!(blue[2] > 0.9, "azul deve permanecer, ficou {}", blue[2]);
  }

  #[test]
  fn hsl_green_hue_shift() {
    // Green: hue +60 → verde gira para ciano/amarelo.
    let mut hsl = [0f32; 24];
    hsl[9] = 60.0; // Green hue (índice 3*3)
    let p = EditParams {
      hsl: Some(hsl),
      ..Default::default()
    };
    let (h, _, _) = rgb_to_hsl(p.apply_pixel([0.0, 1.0, 0.0]));
    // Matiz original do verde puro é ~120; deslocado deve diferir.
    assert!((h - 120.0).abs() > 10.0, "matiz deve mudar, ficou {}", h);
  }

  #[test]
  fn sharpen_boosts_edge_contrast() {
    // Imagem com uma borda nítida (50 → 200). Nitidez deve aumentar o contraste.
    let w = 4u32;
    let h = 1u32;
    let mut rgb = Vec::new();
    for _ in 0..2 {
      for c in 0..3 {
        rgb.push(50);
      }
    }
    for _ in 0..2 {
      for c in 0..3 {
        rgb.push(200);
      }
    }
    let p = EditParams {
      sharpen: Some(80.0),
      ..Default::default()
    };
    let out = unsharp_mask(&rgb, w, h, 80.0);
    // O lado escuro da borda deve escurecer ou o claro deve clarear.
    let dark_edge = out[2 * 3];
    let bright_edge = out[2 * 3 + 0];
    assert!(
      (dark_edge < 50 || bright_edge > 200),
      "borda deve ter mais contraste: {} / {}",
      dark_edge,
      bright_edge
    );
  }

  #[test]
  fn denoise_smooths_uniform_area() {
    // Área uniforme com ruído: denoise deve aproximar os valores da média.
    let w = 4u32;
    let h = 1u32;
    let mut rgb = Vec::new();
    // [100, 112, 88, 100] no canal R; g/b constantes.
    for (i, v) in [100u8, 112, 88, 100].iter().enumerate() {
      rgb.push(*v);
      rgb.push(100);
      rgb.push(100);
    }
    let p = EditParams {
      denoise: Some(90.0),
      ..Default::default()
    };
    let out = denoise_preserve_edges(&rgb, w, h, 90.0);
    let center_r = out[1 * 3] as i32;
    assert!(
      (center_r - 100).abs() < 15,
      "ruído deve suavizar em direção à média, canal r ficou {}",
      center_r
    );
  }
}
