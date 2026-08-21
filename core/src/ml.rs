use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use ndarray::Array4;
use ort::session::Session;
use ort::value::Tensor;

/// Diretório onde os modelos ONNX ficam (core/models).
pub fn models_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models")
}

struct MlEngine {
  scrfd: Option<Mutex<Session>>,
  nima: Option<Mutex<Session>>,
  mobilefacenet: Option<Mutex<Session>>,
}

static ENGINE: OnceLock<Result<MlEngine, String>> = OnceLock::new();

fn init_engine() -> &'static Result<MlEngine, String> {
  ENGINE.get_or_init(|| build_engine().map_err(|e| e.to_string()))
}

fn build_engine() -> Result<MlEngine, Box<dyn std::error::Error>> {
  let dir = models_dir();
  let scrfd_path = dir.join("scrfd_2.5g_bnkps.onnx");
  let nima_path = dir.join("nima_mobilenet_aesthetic.onnx");
  let mfn_path = dir.join("mobilefacenet.onnx");

  let mut scrfd = None;
  if scrfd_path.exists() {
    match build_session(&scrfd_path) {
      Ok(s) => scrfd = Some(Mutex::new(s)),
      Err(e) => eprintln!("[ml] aviso: SCRFD nao carregou: {e}"),
    }
  } else {
    eprintln!("[ml] aviso: modelo SCRFD ausente em {}", scrfd_path.display());
  }

  let mut nima = None;
  if nima_path.exists() {
    match build_session(&nima_path) {
      Ok(s) => nima = Some(Mutex::new(s)),
      Err(e) => eprintln!("[ml] aviso: NIMA nao carregou: {e}"),
    }
  } else {
    eprintln!("[ml] aviso: modelo NIMA ausente em {}", nima_path.display());
  }

  let mut mobilefacenet = None;
  if mfn_path.exists() {
    match build_session(&mfn_path) {
      Ok(s) => mobilefacenet = Some(Mutex::new(s)),
      Err(e) => eprintln!("[ml] aviso: MobileFaceNet nao carregou: {e}"),
    }
  } else {
    eprintln!("[ml] aviso: modelo MobileFaceNet ausente em {}", mfn_path.display());
  }

  Ok(MlEngine { scrfd, nima, mobilefacenet })
}

fn build_session(model: &Path) -> Result<Session, Box<dyn std::error::Error>> {
  let mut builder = Session::builder()?;
  #[cfg(target_os = "macos")]
  {
    let cache = dirs::home_dir()
      .unwrap_or_default()
      .join("Library/Caches/OpenShoot/coreml");
    let _ = std::fs::create_dir_all(&cache);
    builder = builder.with_execution_providers([
      ort::ep::CoreML::default()
        .with_compute_units(ort::ep::coreml::ComputeUnits::All)
        .with_static_input_shapes(true)
        .with_model_cache_dir(cache.to_string_lossy())
        .build()
        .error_on_failure(),
    ])?;
  }
  let session = builder.commit_from_file(model)?;
  let inputs: Vec<String> = session
    .inputs()
    .iter()
    .map(|i| format!("{:?}", i.name()))
    .collect();
  let outputs: Vec<String> = session
    .outputs()
    .iter()
    .map(|o| format!("{:?}", o.name()))
    .collect();  crate::catalog::log_debug(&format!(
    "[ml] modelo {} | inputs={:?} outputs={:?}",
    model.file_name().unwrap_or_default().to_string_lossy(),
    inputs,
    outputs
  ));
  Ok(session)
}

/// Pré-processa imagem para tensor NCHW (CxHxW) com letterbox (olhos SCRFD).
fn to_nchw(
  rgb: &[u8],
  width: u32,
  height: u32,
  side: usize,
  scale_norm: f32,
  offset_norm: f32,
) -> Array4<f32> {
  let mut tensor = Array4::<f32>::zeros((1, 3, side, side));
  let scale = (side as f32 / width as f32).min(side as f32 / height as f32);
  let nw = (width as f32 * scale).round() as i32;
  let nh = (height as f32 * scale).round() as i32;
  let ox = ((side as i32 - nw) / 2).max(0);
  let oy = ((side as i32 - nh) / 2).max(0);
  for y in 0..nh {
    for x in 0..nw {
      let sx = ((x as f32 / scale) as usize).min(width as usize - 1);
      let sy = ((y as f32 / scale) as usize).min(height as usize - 1);
      let i = (sy * width as usize + sx) * 3;
      let (cy, cx) = ((y + oy) as usize, (x + ox) as usize);
      tensor[[0, 0, cy, cx]] = (rgb[i] as f32 - offset_norm) / scale_norm;
      tensor[[0, 1, cy, cx]] = (rgb[i + 1] as f32 - offset_norm) / scale_norm;
      tensor[[0, 2, cy, cx]] = (rgb[i + 2] as f32 - offset_norm) / scale_norm;
    }
  }
  tensor
}

/// Pré-processa imagem para tensor NHWC (HxWxC) com letterbox.
/// Formato padrão de MobileNet/NIMA.
fn to_nhwc(
  rgb: &[u8],
  width: u32,
  height: u32,
  side: usize,
  scale_norm: f32,
  offset_norm: f32,
) -> Array4<f32> {
  let mut tensor = Array4::<f32>::zeros((1, side, side, 3));
  let scale = (side as f32 / width as f32).min(side as f32 / height as f32);
  let nw = (width as f32 * scale).round() as i32;
  let nh = (height as f32 * scale).round() as i32;
  let ox = ((side as i32 - nw) / 2).max(0);
  let oy = ((side as i32 - nh) / 2).max(0);
  for y in 0..nh {
    for x in 0..nw {
      let sx = ((x as f32 / scale) as usize).min(width as usize - 1);
      let sy = ((y as f32 / scale) as usize).min(height as usize - 1);
      let i = (sy * width as usize + sx) * 3;
      let (ty, tx) = ((y + oy) as usize, (x + ox) as usize);
      tensor[[0, ty, tx, 0]] = (rgb[i] as f32 - offset_norm) / scale_norm;
      tensor[[0, ty, tx, 1]] = (rgb[i + 1] as f32 - offset_norm) / scale_norm;
      tensor[[0, ty, tx, 2]] = (rgb[i + 2] as f32 - offset_norm) / scale_norm;
    }
  }
  tensor
}

/// Deteta faces (SCRFD) numa imagem RGB. Retorna bboxes normalizadas (0..1).
/// Decodifica as três escalas (stride 8/16/32) com NMS.
pub fn detect_faces(
  rgb: &[u8],
  width: u32,
  height: u32,
  threshold: f32,
) -> Result<Vec<[f32; 4]>, String> {
  let engine = init_engine();
  let engine = engine.as_ref().map_err(|e| e.clone())?;
  let session = engine.scrfd.as_ref().ok_or_else(|| "SCRFD nao disponivel".to_string())?;
  let mut guard = session.lock().map_err(|e| e.to_string())?;

  const SIDE: usize = 640;
  let tensor = to_nchw(rgb, width, height, SIDE, 128.0, 127.5);

  // Fator de escala do letterbox (imagem -> 640x640).
  let scale = (SIDE as f32 / width as f32).min(SIDE as f32 / height as f32);
  let nw = width as f32 * scale;
  let nh = height as f32 * scale;
  let ox = (SIDE as f32 - nw) / 2.0;
  let oy = (SIDE as f32 - nh) / 2.0;

  let input = Tensor::from_array(tensor).map_err(|e| e.to_string())?;
  let outputs = guard
    .run(ort::inputs![input])
    .map_err(|e| format!("erro SCRFD: {e}"))?;

  // Coletar outputs por escala: score_s (fg), bbox_s.
  // SCRFD: score_s shape [1, Hs, Ws, 2] (bg/fg); bbox_s [1, Hs, Ws, 4] (offsets).
  let mut scores: Vec<(u32, Vec<f32>)> = Vec::new(); // (stride, dados)
  let mut boxes: Vec<(u32, Vec<f32>)> = Vec::new();
  for (name, val) in outputs.iter() {
    let n = name.to_string();
    let stride = if n.contains("_8") { Some(8u32) }
      else if n.contains("_16") { Some(16) }
      else if n.contains("_32") { Some(32) }
      else { None };
    if let Some(s) = stride {
      if let Ok((_, arr)) = val.try_extract_tensor::<f32>() {
        let data = arr.to_vec();
        if n.contains("score") {
          scores.push((s, data));
        } else if n.contains("bbox") {
          boxes.push((s, data));
        }
      }
    }
  }
  if scores.is_empty() || boxes.is_empty() {
    return Err("outputs SCRFD por escala nao encontrados".to_string());
  }

  // Decodificar cada escala.
  // SCRFD outputs: score_s [1, N, 1] (fg), bbox_s [1, N, 4] (offsets), com
  // N = (SIDE/stride)^2 * num_anchors (num_anchors=2 para este modelo).
  let mut candidates: Vec<[f32; 4]> = Vec::new(); // bbox em pixels (letterbox 640)
  let mut candidate_scores: Vec<f32> = Vec::new();
  for (s, sdata) in &scores {
    let bdata = boxes.iter().find(|(bs, _)| bs == s).map(|(_, d)| d);
    let Some(bdata) = bdata else { continue };
    let stride_f = *s as f32;
    let num_anchors = 2usize;
    let cells_per_anchor = (SIDE as usize / *s as usize).pow(2);
    let cols = SIDE as usize / *s as usize;
    let total = bdata.len() / 4;
    if total != cells_per_anchor * num_anchors {
      crate::catalog::log_debug(&format!(
        "[scrfd] aviso: stride {s} total={total} esperado={}",
        cells_per_anchor * num_anchors
      ));
    }
    for cell in 0..cells_per_anchor {
      let row = cell / cols;
      let col = cell % cols;
      for a in 0..num_anchors {
        let idx = (cell * num_anchors + a) * 4;
        let sidx = cell * num_anchors + a;
        if sidx >= sdata.len() {
          continue;
        }
        let fg = sdata[sidx];
        if fg < threshold {
          continue;
        }
        let cx = col as f32 * stride_f + stride_f / 2.0 - 0.5;
        let cy = row as f32 * stride_f + stride_f / 2.0 - 0.5;
        let (dx1, dy1, dx2, dy2) = (
          bdata[idx],
          bdata[idx + 1],
          bdata[idx + 2],
          bdata[idx + 3],
        );
        let x1 = cx - dx1 * stride_f;
        let y1 = cy - dy1 * stride_f;
        let x2 = cx + dx2 * stride_f;
        let y2 = cy + dy2 * stride_f;
        candidates.push([x1, y1, x2, y2]);
        candidate_scores.push(fg);
      }
    }
  }

  // NMS (Intersection-over-Union) não supressivo.
  let keep = nms(&candidates, &candidate_scores, 0.4);

  // Remover o offset do letterbox e normalizar para 0..1 na imagem original.
  let mut faces = Vec::new();
  for idx in keep {
    let b = &candidates[idx];
    let x1 = ((b[0] - ox) / (scale * width as f32)).max(0.0).min(1.0);
    let y1 = ((b[1] - oy) / (scale * height as f32)).max(0.0).min(1.0);
    let x2 = ((b[2] - ox) / (scale * width as f32)).max(0.0).min(1.0);
    let y2 = ((b[3] - oy) / (scale * height as f32)).max(0.0).min(1.0);
    if (x2 - x1) > 0.001 && (y2 - y1) > 0.001 {
      faces.push([x1, y1, x2, y2]);
    }
  }
  Ok(faces)
}

/// Non-Maximum Suppression (IOU) sobre bboxes. Retorna índices mantidos.
fn nms(boxes: &[[f32; 4]], scores: &[f32], iou_threshold: f32) -> Vec<usize> {
  let n = boxes.len();
  if n == 0 {
    return Vec::new();
  }
  // Ordenar por score decrescente.
  let mut order: Vec<usize> = (0..n).collect();
  order.sort_by(|a, b| scores[*b].partial_cmp(&scores[*a]).unwrap_or(std::cmp::Ordering::Equal));

  let mut area = vec![0.0f32; n];
  for i in 0..n {
    let b = &boxes[i];
    area[i] = ((b[2] - b[0]).max(0.0)) * ((b[3] - b[1]).max(0.0));
  }

  let mut keep = Vec::new();
  let mut suppressed = vec![false; n];
  for &i in &order {
    if suppressed[i] {
      continue;
    }
    keep.push(i);
    for &j in &order {
      if suppressed[j] {
        continue;
      }
      let a = boxes[i];
      let b = boxes[j];
      let xx1 = a[0].max(b[0]);
      let yy1 = a[1].max(b[1]);
      let xx2 = a[2].min(b[2]);
      let yy2 = a[3].min(b[3]);
      let iw = (xx2 - xx1).max(0.0);
      let ih = (yy2 - yy1).max(0.0);
      let inter = iw * ih;
      let union = area[i] + area[j] - inter;
      let iou = if union > 0.0 { inter / union } else { 0.0 };
      if iou > iou_threshold {
        suppressed[j] = true;
      }
    }
  }
  keep
}

/// Score estético 1..10 via NIMA. Entrada RGB u8.
pub fn aesthetic_score(rgb: &[u8], width: u32, height: u32) -> Result<f32, String> {
  let engine = init_engine();
  let engine = engine.as_ref().map_err(|e| e.clone())?;
  let session = engine.nima.as_ref().ok_or_else(|| "NIMA nao disponivel".to_string())?;
  let mut guard = session.lock().map_err(|e| e.to_string())?;

  const SIDE: usize = 224;
  // NIMA/mobilenet usa normalização pixel/255 (0..1).
  let tensor = to_nhwc(rgb, width, height, SIDE, 255.0, 0.0);

  let input = Tensor::from_array(tensor).map_err(|e| e.to_string())?;
  let outputs = guard
    .run(ort::inputs![input])
    .map_err(|e| format!("erro NIMA: {e}"))?;

  let mut logits: Option<Vec<f32>> = None;
  for (_, val) in outputs.iter() {
    if let Ok((_, arr)) = val.try_extract_tensor::<f32>() {
      logits = Some(arr.to_vec());
      break;
    }
  }
  let logits = logits.ok_or_else(|| "output NIMA nao extraido".to_string())?;
  if logits.is_empty() {
    return Err("logits vazio".to_string());
  }
  let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
  let exps: Vec<f32> = logits.iter().map(|l| (l - max).exp()).collect();
  let sum: f32 = exps.iter().sum();
  let mut mos = 0.0f32;
  for (i, e) in exps.iter().enumerate() {
    mos += (i + 1) as f32 * e / sum;
  }
  Ok(mos)
}

/// Extrai RGB (HxWx3) a partir do preview/arquivo, com lado máximo.
pub fn load_rgb(path: &Path, max_side: u32) -> Result<(Vec<u8>, u32, u32), String> {
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
  let scale = (max_side as f32 / w as f32).min(max_side as f32 / h as f32).min(1.0);
  let (nw, nh) = ((w as f32 * scale).max(1.0) as u32, (h as f32 * scale).max(1.0) as u32);
  let resized = image::imageops::resize(&rgb, nw, nh, image::imageops::FilterType::Triangle);
  let (rw, rh) = resized.dimensions();
  let bytes = resized.into_raw();
  Ok((bytes, rw, rh))
}

/// Verifica se os modelos ONNX estão disponíveis (baixados).
pub fn models_available() -> bool {
  models_dir().join("scrfd_2.5g_bnkps.onnx").exists()
    && models_dir().join("nima_mobilenet_aesthetic.onnx").exists()
}

/// Indica se o modelo de embedding facial (MobileFaceNet) está disponível.
pub fn embedding_available() -> bool {
  models_dir().join("mobilefacenet.onnx").exists()
}

/// Gera o embedding facial (vetor normalizado) de um rosto dentro da imagem.
/// `bbox` é normalizada (0..1). Recorta o rosto, redimensiona para 112x112
/// (entrada MobileFaceNet) e retorna o vetor de características.
pub fn face_embedding(
  rgb: &[u8],
  width: u32,
  height: u32,
  bbox: [f32; 4],
) -> Result<Vec<f32>, String> {
  let engine = init_engine().as_ref().map_err(|e| e.clone())?;
  let session = engine
    .mobilefacenet
    .as_ref()
    .ok_or_else(|| "MobileFaceNet nao disponivel".to_string())?;
  let mut guard = session.lock().map_err(|e| e.to_string())?;

  const SIDE: usize = 112;
  // Recorta o rosto com uma pequena margem.
  let (x0, y0, x1, y1) = (
    (bbox[0] * width as f32).max(0.0) as usize,
    (bbox[1] * height as f32).max(0.0) as usize,
    (bbox[2] * width as f32).min(width as f32) as usize,
    (bbox[3] * height as f32).min(height as f32) as usize,
  );
  let cw = (x1 - x0).max(1);
  let ch = (y1 - y0).max(1);
  // Margem de 20% ao redor do rosto (ajuda o alinhamento).
  let mx = (cw as f32 * 0.2) as usize;
  let my = (ch as f32 * 0.2) as usize;
  let sx0 = x0.saturating_sub(mx);
  let sy0 = y0.saturating_sub(my);
  let sx1 = (x1 + mx).min(width as usize);
  let sy1 = (y1 + my).min(height as usize);
  let cw2 = (sx1 - sx0).max(1);
  let ch2 = (sy1 - sy0).max(1);

  // Constrói o tensor NCHW 1x3x112x112 por amostragem bilinear simples.
  let mut tensor = Array4::<f32>::zeros((1, 3, SIDE, SIDE));
  for ty in 0..SIDE {
    for tx in 0..SIDE {
      let sx = ((tx as f32 + 0.5) / SIDE as f32 * cw2 as f32).floor() as usize;
      let sy = ((ty as f32 + 0.5) / SIDE as f32 * ch2 as f32).floor() as usize;
      let sx = sx.min(cw2 - 1);
      let sy = sy.min(ch2 - 1);
      let i = ((sy0 + sy) * width as usize + (sx0 + sx)) * 3;
      // MobileFaceNet: normaliza [-1, 1] com (pixel-127.5)/128.
      tensor[[0, 0, ty, tx]] = (rgb[i] as f32 - 127.5) / 128.0;
      tensor[[0, 1, ty, tx]] = (rgb[i + 1] as f32 - 127.5) / 128.0;
      tensor[[0, 2, ty, tx]] = (rgb[i + 2] as f32 - 127.5) / 128.0;
    }
  }

  let input = Tensor::from_array(tensor).map_err(|e| e.to_string())?;
  let outputs = guard
    .run(ort::inputs![input])
    .map_err(|e| format!("erro MobileFaceNet: {e}"))?;

  let mut emb: Option<Vec<f32>> = None;
  for (_, val) in outputs.iter() {
    if let Ok((_, arr)) = val.try_extract_tensor::<f32>() {
      emb = Some(arr.to_vec());
      break;
    }
  }
  let emb = emb.ok_or_else(|| "output MobileFaceNet nao extraido".to_string())?;

  // Normaliza L2 para comparar por cosseno.
  let norm: f32 = emb.iter().map(|v| v * v).sum::<f32>().sqrt();
  if norm > 1e-6 {
    Ok(emb.into_iter().map(|v| v / norm).collect())
  } else {
    Ok(emb)
  }
}

/// Score combinado de IA (0..100): NIMA (estética) + bônus por faces.
/// Retorna None se o ML não estiver disponível (fallback heurístico).
pub fn ml_quality_score(rgb: &[u8], width: u32, height: u32) -> Result<f64, String> {
  // NIMA -> 1..10
  let nima = match aesthetic_score(rgb, width, height) {
    Ok(n) => n,
    Err(e) => {
      crate::catalog::log_debug(&format!("[ml] NIMA falhou: {e}"));
      return Err(e);
    }
  };
  let nima_norm = (((nima - 1.0) / 9.0 * 100.0).clamp(0.0, 100.0)) as f64;

  // Faces: presença de rosto claro dá bônus pequeno.
  let mut face_bonus = 0.0f64;
  match detect_faces(rgb, width, height, 0.5) {
    Ok(faces) => {
      face_bonus = (faces.len() as f64).min(3.0) * 3.0;
    }
    Err(e) => {
      crate::catalog::log_debug(&format!("[ml] SCRFD falhou (ignorando): {e}"));
    }
  }

  let out = (nima_norm * 0.8 + face_bonus).clamp(0.0, 100.0);
  crate::catalog::log_debug(&format!("[ml] nima={nima:.2} nima_norm={nima_norm:.1} faces={face_bonus:.1} out={out:.1}"));
  Ok(out)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn nms_keeps_distinct_boxes() {
    // Duas caixas distantes -> ambas mantidas.
    let boxes = vec![[0.0, 0.0, 10.0, 10.0], [100.0, 100.0, 110.0, 110.0]];
    let scores = vec![0.9, 0.8];
    let keep = nms(&boxes, &scores, 0.4);
    assert_eq!(keep.len(), 2);
  }

  #[test]
  fn nms_suppresses_overlapping() {
    // Duas caixas sobrepostas com scores diferentes -> só a melhor.
    let boxes = vec![[0.0, 0.0, 10.0, 10.0], [1.0, 1.0, 11.0, 11.0]];
    let scores = vec![0.9, 0.7];
    let keep = nms(&boxes, &scores, 0.4);
    assert_eq!(keep.len(), 1);
    assert_eq!(keep[0], 0); // mantém o de maior score
  }

  #[test]
  fn nms_empty() {
    assert!(nms(&[], &[], 0.4).is_empty());
  }

  #[test]
  fn nms_orders_by_score() {
    // A caixa de maior score deve ser mantida, mesmo se não vier primeiro.
    let boxes = vec![[1.0, 1.0, 11.0, 11.0], [0.0, 0.0, 10.0, 10.0]];
    let scores = vec![0.5, 0.95];
    let keep = nms(&boxes, &scores, 0.4);
    assert_eq!(keep.len(), 1);
    assert_eq!(keep[0], 1);
  }

  #[test]
  fn embedding_dimension_and_norm() {
    if !embedding_available() {
      eprintln!("skip: MobileFaceNet ausente");
      return;
    }
    // Imagem sintética (gradiente) simulando um rosto na região central.
    let w = 112u32;
    let h = 112u32;
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
      for x in 0..w {
        rgb.push((x as f32 / w as f32 * 255.0) as u8);
        rgb.push((y as f32 / h as f32 * 255.0) as u8);
        rgb.push(128);
      }
    }
    let emb = face_embedding(&rgb, w, h, [0.1, 0.1, 0.9, 0.9]).expect("embedding");
    assert!(!emb.is_empty(), "embedding não pode ser vazio");
    // Norma L2 ~1 (normalizada).
    let norm: f32 = emb.iter().map(|v| v * v).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 0.1, "norma deve ser ~1, got {norm}");
    // Determinístico: mesma entrada → mesmo vetor.
    let emb2 = face_embedding(&rgb, w, h, [0.1, 0.1, 0.9, 0.9]).expect("embedding2");
    assert_eq!(emb, emb2, "embedding deve ser determinístico");
  }
}
