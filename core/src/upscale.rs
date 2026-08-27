use std::io::Cursor;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use ndarray::Array4;
use ort::session::Session;
use ort::value::Tensor;

/// Modelo padrão de upscale (qualidade alta, usado pelo Upscayl como
/// "Ultramix Balanced"). Deve existir em `core/models/<NOME>.onnx`.
/// Baixável de openmodeldb.info (4x-UltraSharp / 4x-Nomos8k-SHARP).
pub fn upscale_default_model() -> &'static str {
  "4x-UltraSharp"
}

/// Escala nativa dos modelos ESRGAN empacotados (4x). Escalas 2x/3x são
/// obtidas por pós-redimensionamento do resultado 4x.
const NATIVE_SCALE: u32 = 4;

/// Indica se o modelo ONNX de upscale está disponível no disco.
pub fn upscale_model_available(model: &str) -> bool {
  crate::ml::models_dir().join(format!("{model}.onnx")).exists()
}

struct UpscaleState {
  session: Option<Arc<Mutex<Session>>>,
  model_path: std::path::PathBuf,
}

static UPSCALE: OnceLock<Mutex<UpscaleState>> = OnceLock::new();

/// Carrega (e cacheia) a sessão ONNX de upscale. Retorna `None` se o modelo
/// não existir ou falhar ao carregar — o chamador deve usar o fallback simples.
fn get_upscale_session(model: &str) -> Option<Arc<Mutex<Session>>> {
  let path = crate::ml::models_dir().join(format!("{model}.onnx"));
  let cell = UPSCALE.get_or_init(|| {
    Mutex::new(UpscaleState {
      session: None,
      model_path: path.clone(),
    })
  });
  let mut st = cell.lock().ok()?;
  // Reconstrói se o modelo pedido mudou.
  if st.model_path != path {
    st.session = None;
    st.model_path = path.clone();
  }
  if st.session.is_none() {
    if path.exists() {
      match crate::ml::build_session(&path) {
        Ok(s) => st.session = Some(Arc::new(Mutex::new(s))),
        Err(e) => crate::catalog::log_debug(&format!("[upscale] modelo {model} falhou: {e}")),
      }
    } else {
      crate::catalog::log_debug(&format!("[upscale] modelo {model}.onnx ausente em {}", path.display()));
    }
  }
  st.session.clone()
}

/// Decodifica a imagem em RGB full-res (sem cap). Reusa o preview JPEG embutido
/// para RAW quando disponível.
pub fn decode_full_rgb(path: &Path) -> Option<(Vec<u8>, u32, u32)> {
  let img = crate::imageproc::read_embedded_jpeg(path)
    .and_then(|j| {
      image::ImageReader::new(Cursor::new(j))
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()
    })
    .or_else(|| {
      image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()
    })?;
  let rgb = img.to_rgb8();
  let (w, h) = rgb.dimensions();
  Some((rgb.into_raw(), w, h))
}

/// Fallback simples (recurso mais simples): upscale por interpolção bicúbica
/// via crate `image`. Usado quando o modelo ONNX não está disponível ou a
/// inferência falha — mantém a funcionalidade sem dependência de GPU/modelo.
fn simple_upscale(rgb: &[u8], w: u32, h: u32, scale: u32) -> Vec<u8> {
  let img = match image::RgbImage::from_raw(w, h, rgb.to_vec()) {
    Some(i) => i,
    None => return rgb.to_vec(),
  };
  let r = image::imageops::resize(
    &img,
    w * scale,
    h * scale,
    image::imageops::FilterType::CatmullRom,
  );
  r.into_raw()
}

/// Upscale de uma imagem RGB. Usa o modelo ESRGAN (tiling VRAM-safe com
/// feather nas bordas) quando disponível; caso contrário cai no fallback
/// bicúbico. `target_scale` (1..4) define a dimensão de saída.
pub fn upscale_rgb(rgb: &[u8], w: u32, h: u32, model: &str, target_scale: u32) -> Vec<u8> {
  let scale = target_scale.clamp(1, NATIVE_SCALE);

  let session = get_upscale_session(model);
  let native = match session {
    Some(s) => match s.lock() {
      Ok(mut g) => upscale_tiled(rgb, w, h, &mut g),
      // Lock poisonado: trata como indisponível e usa o fallback.
      Err(_) => simple_upscale(rgb, w, h, scale),
    },
    None => {
      // Sem modelo: fallback bicúbico direto na escala pedida.
      simple_upscale(rgb, w, h, scale)
    }
  };

  if scale == NATIVE_SCALE {
    return native;
  }
  // Escalas 2x/3x: pós-redimensiona o resultado 4x (mais barato que novos modelos).
  let nw = w * NATIVE_SCALE;
  let nh = h * NATIVE_SCALE;
  let img = match image::RgbImage::from_raw(nw, nh, native) {
    Some(i) => i,
    None => return simple_upscale(rgb, w, h, scale),
  };
  image::imageops::resize(
    &img,
    w * scale,
    h * scale,
    image::imageops::FilterType::CatmullRom,
  )
  .into_raw()
}

/// Inference tiling com feather de alpha nas sobreposições para evitar o
/// "grid artifact" (mesmo problema tratado no ncnn do Upscayl). Por tile:
/// faz o decode da sub-região, roda o modelo (4x) e compõe no buffer de saída.
fn upscale_tiled(rgb: &[u8], w: u32, h: u32, session: &mut Session) -> Vec<u8> {
  let tile: usize = 256;
  let overlap: usize = (tile / 8).max(1);
  let step = tile - overlap;
  let s = NATIVE_SCALE as usize;

  let ow = (w as usize) * s;
  let oh = (h as usize) * s;
  let mut out = vec![0u8; ow * oh * 3];

  let mut y = 0usize;
  while y < h as usize {
    let mut x = 0usize;
    while x < w as usize {
      let sx0 = x;
      let sy0 = y;
      let sx1 = (x + tile).min(w as usize);
      let sy1 = (y + tile).min(h as usize);
      let tw = sx1 - sx0;
      let th = sy1 - sy0;
      if tw == 0 || th == 0 {
        x += step;
        continue;
      }

      // Pré-processa o tile (NCHW, /255 — Real-ESRGAN não usa mean/std).
      let mut t = Array4::<f32>::zeros((1, 3, th, tw));
      for ty in 0..th {
        for tx in 0..tw {
          let si = ((sy0 + ty) * w as usize + (sx0 + tx)) * 3;
          t[[0, 0, ty, tx]] = rgb[si] as f32 / 255.0;
          t[[0, 1, ty, tx]] = rgb[si + 1] as f32 / 255.0;
          t[[0, 2, ty, tx]] = rgb[si + 2] as f32 / 255.0;
        }
      }

      let input = match Tensor::from_array(t) {
        Ok(v) => v,
        Err(_) => {
          x += step;
          continue;
        }
      };
      let outputs = match session.run(ort::inputs![input]) {
        Ok(o) => o,
        Err(e) => {
          crate::catalog::log_debug(&format!("[upscale] erro inferencia: {e}"));
          x += step;
          continue;
        }
      };

      // Primeiro output: [1,3, th*4, tw*4] em 0..1 (flat Vec<f32> nesta versão).
      let hr = outputs
        .iter()
        .next()
        .and_then(|(_, v)| v.try_extract_tensor::<f32>().ok().map(|(_, arr)| arr.to_vec()));
      let Some(data) = hr else {
        x += step;
        continue;
      };
      let hh = th * s;
      let ww = tw * s;

      let has_prev_x = x > 0;
      let has_prev_y = y > 0;
      for ty in 0..th {
        for tx in 0..tw {
          let wx = edge_weight(tx, overlap, has_prev_x);
          let wy = edge_weight(ty, overlap, has_prev_y);
          let wgt = wx.min(wy);
          for c in 0..3 {
            for dy in 0..s {
              for dx in 0..s {
                let hv = data[(c * hh + (ty * s + dy)) * ww + (tx * s + dx)];
                let oy = (sy0 + ty) * s + dy;
                let ox = (sx0 + tx) * s + dx;
                let pidx = (oy * ow + ox) * 3 + c;
                let old = out[pidx] as f32;
                let nv = (hv * 255.0).clamp(0.0, 255.0);
                out[pidx] = (old * (1.0 - wgt) + nv * wgt) as u8;
              }
            }
          }
        }
      }

      x += step;
    }
    y += step;
  }
  out
}

/// Peso (0..1) do pixel NOVO na composição por tile. 1 no interior e nas
/// bordas externas da imagem; rampa na sobreposição com o tile anterior (onde
/// há um tile já escrito) para fundir as costuras.
fn edge_weight(local: usize, overlap: usize, has_prev: bool) -> f32 {
  if !has_prev {
    return 1.0;
  }
  if local < overlap {
    return (local as f32 / overlap as f32).clamp(0.0, 1.0);
  }
  1.0
}

/// Salva um buffer RGB em disco (jpeg com qualidade ou png).
pub fn save_rgb(
  path: &Path,
  rgb: Vec<u8>,
  w: u32,
  h: u32,
  fmt: &str,
  q: u8,
) -> Result<(), String> {
  let img = image::RgbImage::from_raw(w, h, rgb).ok_or("from_raw falhou")?;
  let dynimg = image::DynamicImage::ImageRgb8(img);
  if fmt == "png" {
    dynimg
      .save_with_format(path, image::ImageFormat::Png)
      .map_err(|e| e.to_string())
  } else {
    let mut buf = Cursor::new(Vec::new());
    {
      let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, q);
      dynimg
        .write_with_encoder(enc)
        .map_err(|e| e.to_string())?;
    }
    std::fs::write(path, buf.into_inner()).map_err(|e| e.to_string())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn grad(w: u32, h: u32) -> Vec<u8> {
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
      for x in 0..w {
        rgb.push((x as f32 / w as f32 * 255.0) as u8);
        rgb.push((y as f32 / h as f32 * 255.0) as u8);
        rgb.push(128);
      }
    }
    rgb
  }

  #[test]
  fn simple_upscale_doubles_dimensions() {
    let rgb = grad(32, 24);
    let out = simple_upscale(&rgb, 32, 24, 2);
    assert_eq!(out.len(), (32 * 2 * 24 * 2 * 3) as usize);
  }

  #[test]
  fn upscale_rgb_fallback_without_model() {
    // Sem modelo no disco -> deve cair no fallback e produzir dims 4x.
    let rgb = grad(16, 16);
    let out = upscale_rgb(&rgb, 16, 16, "modelo-inexistente-xyz", 4);
    assert_eq!(out.len(), (16 * 4 * 16 * 4 * 3) as usize);
  }
}
