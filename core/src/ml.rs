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
}

static ENGINE: OnceLock<Result<MlEngine, String>> = OnceLock::new();

fn init_engine() -> &'static Result<MlEngine, String> {
  ENGINE.get_or_init(|| build_engine().map_err(|e| e.to_string()))
}

fn build_engine() -> Result<MlEngine, Box<dyn std::error::Error>> {
  let dir = models_dir();
  let scrfd_path = dir.join("scrfd_2.5g_bnkps.onnx");
  let nima_path = dir.join("nima_mobilenet_aesthetic.onnx");

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

  Ok(MlEngine { scrfd, nima })
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
    .collect();
  crate::catalog::log_debug(&format!(
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

  let input = Tensor::from_array(tensor).map_err(|e| e.to_string())?;
  let outputs = guard
    .run(ort::inputs![input])
    .map_err(|e| format!("erro SCRFD: {e}"))?;

  // Extrai arrays de saída por nome ou posição.
  let mut score_data: Option<Vec<f32>> = None;
  let mut bbox_data: Option<Vec<f32>> = None;
  for (name, val) in outputs.iter() {
    let n = name.to_string();
    if n.contains("score") || n.contains("scores") {
      if let Ok((_, arr)) = val.try_extract_tensor::<f32>() {
        score_data = Some(arr.to_vec());
      }
    } else if n.contains("bbox") || n.contains("boxes") || n.contains("loc") {
      if let Ok((_, arr)) = val.try_extract_tensor::<f32>() {
        bbox_data = Some(arr.to_vec());
      }
    }
  }
  if score_data.is_none() || bbox_data.is_none() {
    // Fallback por posição: [0] = score, [1] = bbox
    let vals: Vec<_> = outputs.iter().map(|(_, v)| v).collect();
    if score_data.is_none() && !vals.is_empty() {
      if let Ok((_, arr)) = vals[0].try_extract_tensor::<f32>() {
        score_data = Some(arr.to_vec());
      }
    }
    if bbox_data.is_none() && vals.len() > 1 {
      if let Ok((_, arr)) = vals[1].try_extract_tensor::<f32>() {
        bbox_data = Some(arr.to_vec());
      }
    }
  }

  let scores = score_data.ok_or_else(|| "output score nao encontrado".to_string())?;
  let boxes = bbox_data.ok_or_else(|| "output bbox nao encontrado".to_string())?;

  // SCRFD: score shape [1, N, 2] (bg/fg), bbox [1, N, 4].
  // Interpretação simplificada: emparelhar pela dimensão N.
  let stride = if boxes.len() % 4 == 0 { 4 } else { boxes.len() };
  let num = boxes.len() / stride;
  let mut faces = Vec::new();
  for i in 0..num {
    // score pode ser [N] ou [N,2]; usa o maior valor do par.
    let s = scores
      .get(i)
      .copied()
      .or_else(|| scores.get(i * 2).copied())
      .unwrap_or(0.0);
    let s2 = scores.get(i * 2 + 1).copied().unwrap_or(0.0);
    let score = s.max(s2);
    if score >= threshold {
      faces.push([boxes[i * 4], boxes[i * 4 + 1], boxes[i * 4 + 2], boxes[i * 4 + 3]]);
    }
  }
  Ok(faces)
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
