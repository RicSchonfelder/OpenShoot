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
  let _ = (width, height);
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
}
