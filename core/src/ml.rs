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

/// Igual a [`detect_faces`], mas também decodifica os 5 keypoints do modelo
/// SCRFD bnkps (kps_8/kps_16/kps_32): 2 olhos, nariz, 2 cantos da boca.
/// Os outputs kps_s têm formato [1, N, 10] e são offsets do centro da âncora,
/// normalizados pela mesma regra do bbox (offset * stride, depois remove o
/// letterbox e divide por scale*width/height).
#[allow(dead_code)]
pub fn detect_faces_with_kps(
  rgb: &[u8],
  width: u32,
  height: u32,
  threshold: f32,
) -> Result<Vec<crate::types::FaceWithKps>, String> {
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

  // Coletar outputs por escala: score_s (fg), bbox_s, kps_s.
  let mut scores: Vec<(u32, Vec<f32>)> = Vec::new();
  let mut boxes: Vec<(u32, Vec<f32>)> = Vec::new();
  let mut kpts: Vec<(u32, Vec<f32>)> = Vec::new();
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
        } else if n.contains("kps") {
          kpts.push((s, data));
        }
      }
    }
  }
  if scores.is_empty() || boxes.is_empty() || kpts.is_empty() {
    return Err("outputs SCRFD por escala (score/bbox/kps) nao encontrados".to_string());
  }

  // Decodificação idêntica à de detect_faces, com keypoints extras.
  // score_s [1,N,1], bbox_s [1,N,4], kps_s [1,N,10]; N=(SIDE/stride)^2*2 âncoras.
  let mut candidates: Vec<[f32; 4]> = Vec::new(); // bbox em pixels (letterbox 640)
  let mut candidate_kps: Vec<[[f32; 2]; 5]> = Vec::new();
  let mut candidate_scores: Vec<f32> = Vec::new();
  for (s, sdata) in &scores {
    let bdata = boxes.iter().find(|(bs, _)| bs == s).map(|(_, d)| d);
    let Some(bdata) = bdata else { continue };
    let kdata = kpts.iter().find(|(ks, _)| ks == s).map(|(_, d)| d);
    let Some(kdata) = kdata else { continue };
    let stride_f = *s as f32;
    let num_anchors = 2usize;
    let cells_per_anchor = (SIDE as usize / *s as usize).pow(2);
    let cols = SIDE as usize / *s as usize;
    for cell in 0..cells_per_anchor {
      let row = cell / cols;
      let col = cell % cols;
      for a in 0..num_anchors {
        let idx4 = (cell * num_anchors + a) * 4;
        let idx10 = (cell * num_anchors + a) * 10;
        let sidx = cell * num_anchors + a;
        if sidx >= sdata.len() || idx4 + 3 >= bdata.len() || idx10 + 9 >= kdata.len() {
          continue;
        }
        let fg = sdata[sidx];
        if fg < threshold {
          continue;
        }
        let cx = col as f32 * stride_f + stride_f / 2.0 - 0.5;
        let cy = row as f32 * stride_f + stride_f / 2.0 - 0.5;
        let (dx1, dy1, dx2, dy2) = (
          bdata[idx4],
          bdata[idx4 + 1],
          bdata[idx4 + 2],
          bdata[idx4 + 3],
        );
        let x1 = cx - dx1 * stride_f;
        let y1 = cy - dy1 * stride_f;
        let x2 = cx + dx2 * stride_f;
        let y2 = cy + dy2 * stride_f;
        // Keypoints: offset direto do centro da âncora (positivo), * stride.
        let mut kps_px = [[0.0f32; 2]; 5];
        for j in 0..5 {
          kps_px[j][0] = cx + kdata[idx10 + 2 * j] * stride_f;
          kps_px[j][1] = cy + kdata[idx10 + 2 * j + 1] * stride_f;
        }
        candidates.push([x1, y1, x2, y2]);
        candidate_kps.push(kps_px);
        candidate_scores.push(fg);
      }
    }
  }

  // NMS igual ao fluxo do bbox.
  let keep = nms(&candidates, &candidate_scores, 0.4);

  // Remove o offset do letterbox e normaliza para 0..1 na imagem original.
  let w = width as f32;
  let h = height as f32;
  let mut faces = Vec::new();
  for idx in keep {
    let b = &candidates[idx];
    let norm_x = |v: f32| ((v - ox) / (scale * w)).clamp(0.0, 1.0);
    let norm_y = |v: f32| ((v - oy) / (scale * h)).clamp(0.0, 1.0);
    if (b[2] - b[0]) <= 0.001 || (b[3] - b[1]) <= 0.001 {
      continue;
    }
    let mut kps_norm = [[0.0f64; 2]; 5];
    for j in 0..5 {
      kps_norm[j][0] = norm_x(candidate_kps[idx][j][0]) as f64;
      kps_norm[j][1] = norm_y(candidate_kps[idx][j][1]) as f64;
    }
    faces.push(crate::types::FaceWithKps {
      bbox: vec![
        norm_x(b[0]) as f64,
        norm_y(b[1]) as f64,
        norm_x(b[2]) as f64,
        norm_y(b[3]) as f64,
      ],
      kps: kps_norm
        .iter()
        .map(|k| vec![k[0], k[1]])
        .collect(),
    });
  }
  Ok(faces)
}

/// Heurística documentada de olhos abertos (0..1) a partir dos 5 keypoints
/// do SCRFD e dos pixels RGB originais.
///
/// Para cada olho (kps[0] e kps[1], normalizados 0..1):
/// 1. Largura da face aproximada = distância euclidiana entre os centros dos
///    olhos; a caixa do olho tem ~12% dessa largura, centrada no keypoint.
/// 2. Recorta a região em luma normalizada 0..1.
/// 3. Calcula:
///    - variância do Laplaciano (nitidez): íris/cílios/pálpebra aberta criam
///      bordas; olho fechado é pele suave -> variância baixa.
///    - variação vertical média |Δy|: olho fechado tem pouca transição
///      vertical (linha única da pálpebra); aberto tem borda superior/inferior.
///    - brilho médio: região muito escura não é mensurável -> zera confiança.
/// 4. Score do olho = clamp01(0.50*nitidez + 0.35*var_vertical + 0.15*brilho),
///    com tetos empíricos (SHARP_CEILING/VGRAD_CEILING) para mapear 0..1.
/// Retorna a média dos dois olhos.
#[allow(dead_code)]
pub fn eyes_open_score(kps: [[f32; 2]; 5], rgb: &[u8], width: u32, height: u32) -> f32 {
  const SHARP_CEILING: f32 = 0.02;
  const VGRAD_CEILING: f32 = 0.15;

  let w = width as f32;
  let h = height as f32;
  // Proxy de largura da face: distância entre os centros dos olhos (px).
  let dxp = (kps[0][0] - kps[1][0]) * w;
  let dyp = (kps[0][1] - kps[1][1]) * h;
  let face_w = (dxp * dxp + dyp * dyp).sqrt().max(1.0);
  // Caixa ~12% da largura da face centrada no olho.
  let half = (face_w * 0.06).max(2.0);

  let mut total = 0.0f32;
  for kp in &kps[0..2] {
    let cx = kp[0] * w;
    let cy = kp[1] * h;
    let (x0, y0) = (((cx - half).floor() as i32).max(0) as u32, ((cy - half).floor() as i32).max(0) as u32);
    let (x1, y1) = (
      ((cx + half).ceil() as i32).min(width as i32 - 1).max(0) as u32,
      ((cy + half).ceil() as i32).min(height as i32 - 1).max(0) as u32,
    );
    if x1 <= x0 || y1 <= y0 {
      continue;
    }
    let pw = (x1 - x0 + 1) as usize;
    let ph = (y1 - y0 + 1) as usize;
    let mut patch = vec![0.0f32; pw * ph];
    let mut sum = 0.0f32;
    for py in 0..ph as u32 {
      for px in 0..pw as u32 {
        let i = (((y0 + py) * width + (x0 + px)) * 3) as usize;
        let luma =
          (0.299 * rgb[i] as f32 + 0.587 * rgb[i + 1] as f32 + 0.114 * rgb[i + 2] as f32) / 255.0;
        patch[(py as usize) * pw + px as usize] = luma;
        sum += luma;
      }
    }
    let bright = sum / (pw * ph) as f32;

    // Variância do Laplaciano simples (vizinhança-4).
    let mut lap_vals: Vec<f32> = Vec::new();
    for y in 1..ph - 1 {
      for x in 1..pw - 1 {
        let c = patch[y * pw + x];
        let lap = 4.0 * c
          - patch[(y - 1) * pw + x]
          - patch[(y + 1) * pw + x]
          - patch[y * pw + (x - 1)]
          - patch[y * pw + (x + 1)];
        lap_vals.push(lap);
      }
    }
    let sharp = if lap_vals.is_empty() {
      0.0
    } else {
      let n = lap_vals.len() as f32;
      let mean = lap_vals.iter().sum::<f32>() / n;
      let var = lap_vals.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / n;
      var.max(0.0)
    };

    // Variação vertical média |Δy| entre linhas consecutivas.
    let mut vsum = 0.0f32;
    let mut vcount = 0u32;
    for y in 1..ph {
      for x in 0..pw {
        vsum += (patch[y * pw + x] - patch[(y - 1) * pw + x]).abs();
        vcount += 1;
      }
    }
    let vgrad = if vcount == 0 { 0.0 } else { vsum / vcount as f32 };

    let sharp_norm = (sharp / SHARP_CEILING).clamp(0.0, 1.0);
    let vgrad_norm = (vgrad / VGRAD_CEILING).clamp(0.0, 1.0);
    // Região escura/estourada não permite medir: fator reduz o score.
    let light = (bright / 0.25).clamp(0.0, 1.0);
    total += (0.50 * sharp_norm + 0.35 * vgrad_norm + 0.15 * light).clamp(0.0, 1.0);
  }
  (total / 2.0).clamp(0.0, 1.0)
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

  /// Imagem sintética RGB (gradiente horizontal + vertical).
  fn gradient_rgb(w: u32, h: u32) -> Vec<u8> {
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
  fn scrfd_kps_decode_runs_without_error() {
    // Skip se o modelo não foi baixado (CI sem pesos).
    if !models_dir().join("scrfd_2.5g_bnkps.onnx").exists() {
      eprintln!("skip: SCRFD ausente");
      return;
    }
    let w = 320u32;
    let h = 240u32;
    let rgb = gradient_rgb(w, h);
    // Só valida que roda sem erro e que a estrutura devolvida é consistente.
    let faces = detect_faces_with_kps(&rgb, w, h, 0.7).expect("detect_faces_with_kps");
    for f in &faces {
      assert_eq!(f.bbox.len(), 4, "bbox deve ter 4 coords");
      assert_eq!(f.kps.len(), 5, "devem ser 5 keypoints");
      for v in &f.bbox {
        assert!((0.0..=1.0).contains(v), "bbox fora de 0..1: {v}");
      }
      for kp in &f.kps {
        for c in kp {
          assert!((0.0..=1.0).contains(c), "kps fora de 0..1: {c}");
        }
      }
      // Olhos à esquerda do nariz, nariz acima da boca (ordem bnkps).
      assert!(f.kps[0][0] <= f.kps[2][0] || f.kps[1][0] <= f.kps[2][0]);
    }
  }

  #[test]
  fn eyes_open_scores_open_above_closed() {
    let w = 200u32;
    let h = 200u32;
    // Keypoints normalizados: olhos em (60,80) e (140,80); resto plausível.
    let kps = [
      [60.0 / w as f32, 80.0 / h as f32],
      [140.0 / w as f32, 80.0 / h as f32],
      [0.50, 0.55],
      [0.42, 0.65],
      [0.58, 0.65],
    ];
    // "Rosto fechado": pele uniforme.
    let closed = vec![190u8; (w * h * 3) as usize];
    // "Rosto aberto": xadrez de alto contraste dentro das caixas dos olhos
    // (simula íris/pálpebras com bordas -> nitidez alta).
    let mut open = closed.clone();
    let half = 6i32;
    for &[kx, ky] in &kps[0..2] {
      let cx = (kx * w as f32) as i32;
      let cy = (ky * h as f32) as i32;
      for dy in -half..=half {
        for dx in -half..=half {
          let x = cx + dx;
          let y = cy + dy;
          if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
            continue;
          }
          let i = ((y * w as i32 + x) * 3) as usize;
          let v = if (x + y) % 2 == 0 { 20 } else { 235 };
          open[i] = v;
          open[i + 1] = v;
          open[i + 2] = v;
        }
      }
    }
    let s_open = eyes_open_score(kps, &open, w, h);
    let s_closed = eyes_open_score(kps, &closed, w, h);
    assert!(
      (0.0..=1.0).contains(&s_open),
      "score aberto fora de 0..1: {s_open}"
    );
    assert!(
      (0.0..=1.0).contains(&s_closed),
      "score fechado fora de 0..1: {s_closed}"
    );
    assert!(
      s_open > s_closed + 0.15,
      "olho aberto ({s_open:.3}) deve pontuar bem acima do fechado ({s_closed:.3})"
    );
  }

  #[test]
  fn eyes_open_score_flat_dark_is_low() {
    let w = 64u32;
    let h = 64u32;
    let kps = [
      [0.25, 0.25],
      [0.75, 0.25],
      [0.5, 0.5],
      [0.4, 0.7],
      [0.6, 0.7],
    ];
    let dark = vec![10u8; (w * h * 3) as usize];
    let s = eyes_open_score(kps, &dark, w, h);
    assert!((0.0..=1.0).contains(&s));
    assert!(s < 0.5, "região escura e lisa deve ter score baixo, got {s}");
  }
}
