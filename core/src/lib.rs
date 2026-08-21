#[macro_use]
extern crate napi_derive;

mod catalog;
mod captions;
mod cr3;
mod culling;
mod edit;
mod geometric;
mod imageproc;
mod ml;
mod retouch;
mod types;
mod xmp;

use napi::bindgen_prelude::*;
use rayon::prelude::*;
use std::path::PathBuf;

use types::{DuplicateGroup, FilterCounts, PhotoList, PhotoMeta, Preset, ScanResult};

/// Inicializa o core: diretório de dados + catálogo SQLite.
/// data_dir: caminho absoluto. Retorna o caminho do banco criado.
#[napi]
pub fn setup(data_dir: String) -> Result<String> {
  catalog::init(&data_dir)
    .map(|p| p.to_string_lossy().to_string())
    .map_err(|e| Error::from_reason(e))
}

/// Soma de teste da ponte (Fase 0, mantido).
#[napi]
pub fn add(a: i32, b: i32) -> i32 {
  a + b
}

/// Versão do core.
#[napi(js_name = "coreVersion")]
pub fn core_version() -> String {
  env!("CARGO_PKG_VERSION").to_string()
}

/// Hello de teste (Fase 0).
#[napi]
pub fn hello(name: String) -> String {
  format!(
    "Hello from Rust (openshoot-core v{})! Ponte Electron <-> Rust OK. Bem-vindo, {name}!",
    env!("CARGO_PKG_VERSION")
  )
}

/// Varre uma pasta recursivamente, adicionando fotos ao catálogo.
#[napi]
pub fn scan_folder(dir: String) -> Result<ScanResult> {
  catalog::scan_folder(&dir).map_err(|e| Error::from_reason(e))
}

/// Lista fotos do catálogo com paginação, busca e filtro de rating.
/// filter: "all" | "picks" (>=4) | "rejects" (<=1, >0) | "unrated" (==0)
#[napi]
pub fn list_photos(
  search: Option<String>,
  filter: Option<String>,
  offset: i64,
  limit: i64,
) -> Result<PhotoList> {
  catalog::list_photos(
    &search.unwrap_or_default(),
    &filter.unwrap_or_else(|| "all".to_string()),
    offset,
    limit,
  )
  .map_err(|e| Error::from_reason(e))
}

/// Retorna metadados de uma foto pelo id.
#[napi]
pub fn get_photo(id: i64) -> Result<Option<PhotoMeta>> {
  catalog::get_photo(id).map_err(|e| Error::from_reason(e))
}

/// Agrupa fotos duplicadas por sha256 (2+ por grupo).
#[napi]
pub fn find_duplicates() -> Result<Vec<DuplicateGroup>> {
  catalog::find_duplicates().map_err(|e| Error::from_reason(e))
}

/// Contagens por bucket (painel de filtros com números vivos).
#[napi]
pub fn filter_counts() -> Result<FilterCounts> {
  catalog::filter_counts().map_err(|e| Error::from_reason(e))
}

// ---- Presets de edição ----

/// Salva um preset nomeado (receita JSON de edição).
#[napi]
pub fn save_preset(name: String, recipe: String) -> Result<()> {
  catalog::save_preset(&name, &recipe).map_err(|e| Error::from_reason(e))
}

/// Lista presets salvos.
#[napi]
pub fn list_presets() -> Result<Vec<Preset>> {
  catalog::list_presets().map_err(|e| Error::from_reason(e))
}

/// Remove um preset pelo nome.
#[napi]
pub fn delete_preset(name: String) -> Result<bool> {
  catalog::delete_preset(&name).map_err(|e| Error::from_reason(e))
}

/// Total de fotos no catálogo.
#[napi]
pub fn photo_count() -> Result<i64> {
  catalog::count_photos().map_err(|e| Error::from_reason(e))
}

/// Diretório de cache de thumbnails (liberável — pode apagar para liberar espaço).
fn thumb_cache_dir() -> PathBuf {
  dirs::home_dir()
    .unwrap_or_else(|| PathBuf::from("."))
    .join("Library/Caches/OpenShoot/thumbs")
}

/// Gera um thumbnail JPEG (base64 data-uri) para uma foto do catálogo por id.
/// Com cache em disco (~/Library/Caches/OpenShoot/thumbs) para não re-decodificar.
#[napi]
pub async fn thumb_for_photo(id: i64, max_dim: u32) -> Result<Option<String>> {
  let photo = match catalog::get_photo(id) {
    Ok(Some(p)) => p,
    Ok(None) => return Ok(None),
    Err(e) => return Err(Error::from_reason(e)),
  };
  let path = PathBuf::from(&photo.path);
  let dim = if max_dim == 0 { 256 } else { max_dim };

  // Cache em disco: <cache_dir>/<id>.jpg (base64 data-uri).
  let cache_dir = thumb_cache_dir();
  let cache_file = cache_dir.join(format!("{id}.jpg"));
  if let Ok(text) = std::fs::read_to_string(&cache_file) {
    if text.starts_with("data:image/jpeg;base64,") {
      return Ok(Some(text));
    }
  }

  // Gera e salva no cache.
  let gen = tokio::task::spawn_blocking(move || imageproc::thumbnail_base64(&path, dim).ok())
    .await
    .map_err(|e| Error::from_reason(e.to_string()))?;

  if let Some(base64) = &gen {
    if let Err(e) = std::fs::create_dir_all(&cache_dir) {
      crate::catalog::log_debug(&format!("[cache] erro criar dir: {e}"));
    } else if let Err(e) = std::fs::write(&cache_file, base64) {
      crate::catalog::log_debug(&format!("[cache] erro gravar: {e}"));
    }
  }
  Ok(gen)
}

/// Gera thumbnail a partir de um caminho absoluto (independente do catálogo).
#[napi]
pub async fn thumb_for_path(path: String, max_dim: u32) -> Result<Option<String>> {
  let dim = if max_dim == 0 { 256 } else { max_dim };
  let pathb = PathBuf::from(&path);
  tokio::task::spawn_blocking(move || imageproc::thumbnail_base64(&pathb, dim).ok())
    .await
    .map_err(|e| Error::from_reason(e.to_string()))
}

/// Executa o culling heurístico em todas as fotos do catálogo.
/// Persiste rating (0-5) e cull_score. Retorna resumo.
#[napi(object)]
pub struct CullSummary {
  pub processed: i64,
  pub errors: i64,
  pub avg_score: f64,
  pub picks: i64,
  pub review: i64,
}
#[napi]
pub async fn cull_photos(target_picks: Option<i64>) -> Result<CullSummary> {
  let paths = match catalog::all_photo_paths() {
    Ok(p) => p,
    Err(e) => return Err(Error::from_reason(e)),
  };
  let ml_ok = ml::models_available();
  if ml_ok {
    crate::catalog::log_debug("[cull] usando ML (NIMA + SCRFD)");
  } else {
    crate::catalog::log_debug("[cull] ML indisponivel, usando heuristica apenas");
  }

  let results: Vec<(i64, bool, std::result::Result<f64, String>)> = paths
    .into_par_iter()
    .map(|p: catalog::PhotoPath| -> (i64, bool, std::result::Result<f64, String>) {
      let path = PathBuf::from(&p.path);
      // Detecção de rosto (SCRFD) para preencher has_face (usado no filtro "faces").
      let has_face = if ml_ok {
        ml::load_rgb(&path, 640)
          .and_then(|(rgb, w, h)| ml::detect_faces(&rgb, w, h, 0.5))
          .map(|faces| !faces.is_empty())
          .unwrap_or(false)
      } else {
        false
      };
      let score = if ml_ok {
        // IA: heurística + ML combinados
        let heur = culling::heuristic_score(&path, 320);
        match ml::load_rgb(&path, 640).and_then(|(rgb, w, h)| ml::ml_quality_score(&rgb, w, h)) {
          Ok(mls) => {
            let h = heur.unwrap_or(50.0);
            Ok(h * 0.5 + mls * 0.5)
          }
          Err(e) => {
            crate::catalog::log_debug(&format!("[cull] fallback heuristica p/ {}: {}", p.path, e));
            heur
          }
        }
      } else {
        culling::heuristic_score(&path, 320)
      };
      (p.id, has_face, score)
    })
    .collect();

  let mut processed = 0;
  let mut errors = 0;
  let mut sum = 0.0;
  let mut scores: Vec<(i64, f64)> = Vec::new();
  for (id, has_face, r) in results {
    // Persiste has_face independentemente do score.
    if let Err(e) = catalog::set_photo_has_face(id, has_face) {
      crate::catalog::log_debug(&format!("falha ao salvar has_face {}: {e}", id));
    }
    match r {
      Ok(s) => {
        processed += 1;
        sum += s;
        scores.push((id, s));
      }
      Err(_) => errors += 1,
    }
  }

  // Normaliza scores 0..100 -> rating 1..5 pela distribuição (quantis simples).
  scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
  let n = scores.len();

  // Quantis → rating 1-5 (distribuição).
  for (i, (id, s)) in scores.iter().enumerate() {
    let q = if n <= 1 { 1.0 } else { i as f64 / (n - 1) as f64 };
    let rating = if q < 0.2 { 1 } else if q < 0.4 { 2 } else if q < 0.6 { 3 } else if q < 0.8 { 4 } else { 5 };
    if let Err(e) = catalog::set_photo_rating(*id, rating, *s) {
      crate::catalog::log_debug(&format!("falha ao salvar rating {}: {e}", id));
      errors += 1;
    }
  }

  // Meta de picks: se o usuário pedir N fotos, as top-N viram ★5 (pick).
  // Caso contrário, usa o limiar de score >= 70.
  let picks: i64;
  let target = target_picks.unwrap_or(0);
  if target > 0 && n > 0 {
    let count = target.min(n as i64) as usize;
    for (i, (id, _)) in scores.iter().enumerate() {
      // scores está em ordem crescente → top-N são os últimos.
      let is_pick = i >= n - count;
      let rating = if is_pick { 5 } else { 0 };
      if let Err(e) = catalog::set_photo_rating(*id, rating, scores[i].1) {
        crate::catalog::log_debug(&format!("falha ao salvar rating {}: {e}", id));
        errors += 1;
      }
      // Destaque IA: marca ai_pick só nas fotos escolhidas pela IA.
      if let Err(e) = catalog::set_photo_ai_pick(*id, is_pick) {
        crate::catalog::log_debug(&format!("falha ao salvar ai_pick {}: {e}", id));
      }
    }
    picks = count as i64;
  } else {
    for (id, s) in &scores {
      let is_pick = *s >= 70.0;
      if let Err(e) = catalog::set_photo_ai_pick(*id, is_pick) {
        crate::catalog::log_debug(&format!("falha ao salvar ai_pick {}: {e}", id));
      }
    }
    picks = scores.iter().filter(|(_, s)| *s >= 70.0).count() as i64;
  }

  // Bucket "Para revisão": fotos com score ambíguo (entre 55 e 70) — nem óbvias
  // picks, nem rejeições claras. Marcadas na coluna `review` para filtro na UI.
  let mut review = 0i64;
  for (id, s) in &scores {
    let ambiguous = *s >= 55.0 && *s < 70.0;
    if let Err(e) = catalog::set_photo_review(*id, ambiguous) {
      crate::catalog::log_debug(&format!("falha ao salvar review {}: {e}", id));
    }
    if ambiguous {
      review += 1;
    }
  }

  Ok(CullSummary {
    processed: processed as i64,
    errors,
    avg_score: if processed > 0 { sum / processed as f64 } else { 0.0 },
    picks: picks.max(0),
    review,
  })
}

/// Escreve o sidecar XMP da foto (rating + label) ao lado do arquivo original.
#[napi]
pub fn write_xmp_for_photo(id: i64) -> Result<String> {
  let photo = catalog::get_photo(id).map_err(|e| Error::from_reason(e))?;
  let photo = photo.ok_or_else(|| Error::from_reason(format!("foto {id} nao encontrada")))?;
  let path = PathBuf::from(&photo.path);
  let (label, keywords) = if photo.rating >= 4 {
    ("Green", vec!["OpenShoot:keep".to_string()])
  } else if photo.rating == 3 {
    ("Yellow", vec!["OpenShoot:maybe".to_string()])
  } else {
    ("Red", vec!["OpenShoot:cull".to_string()])
  };
  xmp::write_xmp(&path, photo.rating, label, &keywords)
    .map(|p| p.to_string_lossy().to_string())
    .map_err(|e| Error::from_reason(e))
}

/// [debug] Detecta faces numa imagem por caminho. Retorna contagem + bboxes.
#[napi]
pub fn detect_faces_in_path(path: String) -> Result<serde_json::Value> {
  let pb = PathBuf::from(&path);
  let (rgb, w, h) = ml::load_rgb(&pb, 640).map_err(|e| Error::from_reason(e))?;
  let faces = ml::detect_faces(&rgb, w, h, 0.5).map_err(|e| Error::from_reason(e))?;
  serde_json::json!({
    "count": faces.len(),
    "faces": faces,
    "width": w,
    "height": h,
  })
  .try_into()
  .map_err(|e| Error::from_reason(format!("json: {e}")))
}

/// Exporta sidecars XMP em massa para todas as fotos com rating > 0.
/// Retorna { exported, errors }.
#[napi(object)]
pub struct XmpExportResult {
  pub exported: i64,
  pub errors: i64,
  pub total: i64,
}

#[napi]
pub fn export_all_xmp() -> Result<XmpExportResult> {
  let photos = catalog::all_photo_paths().map_err(|e| Error::from_reason(e))?;
  let mut exported = 0i64;
  let mut errors = 0i64;
  for p in photos {
    // busca rating da foto
    let photo = match catalog::get_photo(p.id) {
      Ok(Some(x)) => x,
      _ => {
        errors += 1;
        continue;
      }
    };
    if photo.rating <= 0 {
      continue;
    }
    let path = PathBuf::from(&photo.path);
    let (label, keywords) = if photo.rating >= 4 {
      ("Green", vec!["OpenShoot:keep".to_string()])
    } else if photo.rating == 3 {
      ("Yellow", vec!["OpenShoot:maybe".to_string()])
    } else {
      ("Red", vec!["OpenShoot:cull".to_string()])
    };
    match xmp::write_xmp(&path, photo.rating, label, &keywords) {
      Ok(_) => exported += 1,
      Err(e) => {
        errors += 1;
        crate::catalog::log_debug(&format!("[xmp] erro {}: {e}", p.path));
      }
    }
  }
  Ok(XmpExportResult {
    exported,
    errors,
    total: exported + errors,
  })
}

// ---------------- Fase 3: edição em lote ----------------

fn parse_edit_params(json: &str) -> std::result::Result<edit::EditParams, String> {
  serde_json::from_str(json).map_err(|e| format!("receita inválida: {e}"))
}

/// Salva a receita de edição (JSON) de uma foto. Não-destrutiva.
#[napi]
pub fn set_photo_edit(id: i64, params_json: String) -> Result<()> {
  // valida a receita antes de persistir
  parse_edit_params(&params_json).map_err(|e| Error::from_reason(e))?;
  catalog::set_photo_edit(id, &params_json).map_err(|e| Error::from_reason(e))
}

/// Lê a receita de edição (JSON) de uma foto. "" se não houver.
#[napi]
pub fn get_photo_edit(id: i64) -> Result<String> {
  catalog::get_photo_edit(id).map_err(|e| Error::from_reason(e))
}

/// Gera um thumbnail EDITADO (base64) de uma foto, aplicando a receita.
/// Usado para preview em tempo real na UI.
#[napi]
pub async fn preview_edit(id: i64, params_json: String, max_dim: u32) -> Result<Option<String>> {
  let params = match parse_edit_params(&params_json) {
    Ok(p) => p,
    Err(e) => return Err(Error::from_reason(e)),
  };
  let photo = match catalog::get_photo(id) {
    Ok(Some(p)) => p,
    Ok(None) => return Ok(None),
    Err(e) => return Err(Error::from_reason(e)),
  };
  let path = PathBuf::from(&photo.path);
  let dim = if max_dim == 0 { 256 } else { max_dim };
  tokio::task::spawn_blocking(move || edit::edit_thumbnail_base64(&path, &params, dim).ok())
    .await
    .map_err(|e| Error::from_reason(e.to_string()))
}

/// Aplica a receita de edição a TODAS as fotos do catálogo (não-destrutivo).
/// Retorna JSON { applied, errors, total }.
#[napi]
pub fn apply_edit_all(params_json: String) -> Result<String> {
  let params = match parse_edit_params(&params_json) {
    Ok(p) => p,
    Err(e) => return Err(Error::from_reason(e)),
  };
  let photos = match catalog::all_photo_paths() {
    Ok(p) => p,
    Err(e) => return Err(Error::from_reason(e)),
  };
  let mut applied = 0i64;
  let mut errors = 0i64;
  for p in photos {
    let json = serde_json::to_string(&params).unwrap_or_default();
    match catalog::set_photo_edit(p.id, &json) {
      Ok(_) => applied += 1,
      Err(e) => {
        errors += 1;
        crate::catalog::log_debug(&format!("[edit] erro {}: {e}", p.id));
      }
    }
  }
  Ok(serde_json::json!({ "applied": applied, "errors": errors, "total": applied + errors }).to_string())
}

/// Aplica a receita a uma foto específica e retorna o preview editado.
#[napi]
pub async fn apply_edit_one(id: i64, params_json: String, max_dim: u32) -> Result<Option<String>> {
  let params = match parse_edit_params(&params_json) {
    Ok(p) => p,
    Err(e) => return Err(Error::from_reason(e)),
  };
  let json = serde_json::to_string(&params).unwrap_or_default();
  if let Err(e) = catalog::set_photo_edit(id, &json) {
    return Err(Error::from_reason(e));
  }
  preview_edit(id, json, max_dim).await
}

// ---------------- Fase 4: retoque ----------------

/// Gera um thumbnail com suavização de pele aplicada (base64).
/// intensity 0..1 (força do retoque). Não-destrutivo.
#[napi]
pub async fn retouch_skin_photo(id: i64, intensity: f64, max_dim: u32) -> Result<Option<String>> {
  let photo = match catalog::get_photo(id) {
    Ok(Some(p)) => p,
    Ok(None) => return Ok(None),
    Err(e) => return Err(Error::from_reason(e)),
  };
  let path = PathBuf::from(&photo.path);
  let dim = if max_dim == 0 { 256 } else { max_dim };
  let inten = intensity as f32;
  tokio::task::spawn_blocking(move || {
    retouch::retouch_skin_thumbnail_base64(&path, inten, dim).ok()
  })
  .await
  .map_err(|e| Error::from_reason(e.to_string()))
}

/// Aplica suavização de pele a uma foto e salva a intensidade (não-destrutiva).
#[napi]
pub async fn apply_retouch(id: i64, intensity: f64, max_dim: u32) -> Result<Option<String>> {
  let preview = retouch_skin_photo(id, intensity, max_dim).await?;
  Ok(preview)
}

/// Remove uma distração (inpainting) de uma foto por bbox normalizada.
/// mask_rect: [x1, y1, x2, y2] em 0..1. Retorna thumbnail (base64).
#[napi]
pub async fn inpaint_photo(
  id: i64,
  mask_rect: Vec<f64>,
  max_dim: u32,
) -> Result<Option<String>> {
  if mask_rect.len() != 4 {
    return Err(Error::from_reason("mask_rect deve ter 4 valores".to_string()));
  }
  let photo = match catalog::get_photo(id) {
    Ok(Some(p)) => p,
    Ok(None) => return Ok(None),
    Err(e) => return Err(Error::from_reason(e)),
  };
  let path = PathBuf::from(&photo.path);
  let rect = [mask_rect[0] as f32, mask_rect[1] as f32, mask_rect[2] as f32, mask_rect[3] as f32];
  let dim = if max_dim == 0 { 256 } else { max_dim };
  tokio::task::spawn_blocking(move || retouch::inpaint_thumbnail_base64(&path, rect, dim).ok())
    .await
    .map_err(|e| Error::from_reason(e.to_string()))
}

// ---------------- Fase 4b: ajuste de horizonte + recorte IA ----------------

/// Ajuste de horizonte automático (Hough). Retorna { preview, angle }.
#[napi]
pub async fn auto_level_photo(id: i64, max_dim: u32) -> Result<serde_json::Value> {
  let photo = match catalog::get_photo(id) {
    Ok(Some(p)) => p,
    Ok(None) => return Err(Error::from_reason(format!("foto {id} nao encontrada"))),
    Err(e) => return Err(Error::from_reason(e)),
  };
  let path = PathBuf::from(&photo.path);
  let dim = if max_dim == 0 { 512 } else { max_dim };
  tokio::task::spawn_blocking(move || {
    match geometric::auto_level_base64(&path, dim) {
      Ok((preview, angle)) => serde_json::json!({ "preview": preview, "angle": angle }),
      Err(e) => serde_json::json!({ "error": e }),
    }
  })
  .await
  .map_err(|e| Error::from_reason(e.to_string()))
}

/// Recorte automático com IA (centraliza no sujeito/faces). Retorna preview.
#[napi]
pub async fn ai_crop_photo(id: i64, max_dim: u32) -> Result<Option<String>> {
  let photo = match catalog::get_photo(id) {
    Ok(Some(p)) => p,
    Ok(None) => return Ok(None),
    Err(e) => return Err(Error::from_reason(e)),
  };
  let path = PathBuf::from(&photo.path);
  let dim = if max_dim == 0 { 512 } else { max_dim };
  tokio::task::spawn_blocking(move || geometric::ai_crop_base64(&path, dim).ok())
    .await
    .map_err(|e| Error::from_reason(e.to_string()))
}

// ---------------- Fase 6: captions locais ----------------

/// Gera captions/keywords locais (offline) para uma foto, usando EXIF + faces.
/// Retorna JSON { keywords, title, description }.
#[napi]
pub async fn generate_caption(id: i64) -> Result<String> {
  let photo = match catalog::get_photo(id) {
    Ok(Some(p)) => p,
    Ok(None) => return Err(Error::from_reason(format!("foto {id} nao encontrada"))),
    Err(e) => return Err(Error::from_reason(e)),
  };
  // Conta faces localmente (se disponível).
  let mut face_count = 0usize;
  if ml::models_available() {
    let path = PathBuf::from(&photo.path);
    if let Ok((rgb, w, h)) = ml::load_rgb(&path, 640) {
      if let Ok(faces) = ml::detect_faces(&rgb, w, h, 0.5) {
        face_count = faces.len();
      }
    }
  }
  let cap = captions::generate(&photo, face_count);
  Ok(serde_json::json!({
    "keywords": cap.keywords,
    "title": cap.title,
    "description": cap.description,
  })
  .to_string())
}

// ---------------- UX: atalhos de teclado / rating manual / deletar ----------------

/// Define o rating manual de uma foto (via atalho de teclado P/X/1-5).
#[napi]
pub fn set_rating(id: i64, rating: i64) -> Result<()> {
  catalog::set_photo_rating_manual(id, rating).map_err(Error::from_reason)
}

/// Move uma foto para a LIXEIRA do sistema (não-destrutivo) e remove do catálogo.
/// Retorna true se moveu, false se a foto não existia.
/// Move um arquivo para a Lixeira do macOS sem disparar permissão de Automação
/// do Finder. Usa a pasta ~/.Trash diretamente (100% nativo, sem AppleScript).
/// Retorna o destino ou erro.
fn move_to_trash(src: &std::path::Path) -> std::result::Result<(), String> {
  let trash_dir = dirs::home_dir()
    .ok_or_else(|| "sem home dir".to_string())?
    .join(".Trash");
  std::fs::create_dir_all(&trash_dir).map_err(|e| e.to_string())?;
  let name = src
    .file_name()
    .ok_or_else(|| "arquivo sem nome".to_string())?
    .to_string_lossy()
    .to_string();
  // Nome único na Lixeira (evita sobrescrever).
  let mut dest = trash_dir.join(&name);
  if dest.exists() {
    let stem = src.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let ext = src.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
    let mut i = 1;
    loop {
      let candidate = if ext.is_empty() {
        trash_dir.join(format!("{stem} ({i})"))
      } else {
        trash_dir.join(format!("{stem} ({i}).{ext}"))
      };
      if !candidate.exists() {
        dest = candidate;
        break;
      }
      i += 1;
    }
  }
  std::fs::rename(src, &dest).map_err(|e| format!("falha ao mover p/ lixeira: {e}"))?;
  Ok(())
}

#[napi]
pub fn delete_photo(id: i64) -> Result<bool> {
  let photo = match catalog::get_photo(id) {
    Ok(Some(p)) => p,
    Ok(None) => return Ok(false),
    Err(e) => return Err(Error::from_reason(e)),
  };
  let path = PathBuf::from(&photo.path);
  // Move para a lixeira do sistema (sem permissão Finder).
  move_to_trash(&path).map_err(Error::from_reason)?;
  // Remove do cache de thumbnails em disco.
  let _ = std::fs::remove_file(thumb_cache_dir().join(format!("{id}.jpg")));
  // Se havia sidecar XMP, move junto.
  let xmp_path = path.with_extension("xmp");
  if xmp_path.exists() {
    let _ = move_to_trash(&xmp_path);
  }
  catalog::remove_photo(id).map_err(Error::from_reason)?;
  Ok(true)
}

/// Remove todos os thumbnails em cache do disco (libera espaço).
/// Retorna quantos arquivos foram removidos.
#[napi]
pub fn clear_thumb_cache() -> Result<i64> {
  let dir = thumb_cache_dir();
  if !dir.exists() {
    return Ok(0);
  }
  let mut removed = 0i64;
  if let Ok(entries) = std::fs::read_dir(&dir) {
    for e in entries.flatten() {
      let p = e.path();
      if p.is_file() {
        if std::fs::remove_file(&p).is_ok() {
          removed += 1;
        }
      }
    }
  }
  Ok(removed)
}

/// Remove uma foto APENAS do catálogo do OpenShoot — o arquivo original
/// permanece intocado no disco. Retorna true se removida, false se não existia.
#[napi]
pub fn remove_photo_from_catalog(id: i64) -> Result<bool> {
  // Verifica se existe antes de remover.
  match catalog::get_photo(id) {
    Ok(Some(_)) => {}
    Ok(None) => return Ok(false),
    Err(e) => return Err(Error::from_reason(e)),
  }
  // Remove do catálogo e do cache de thumbnails (sem tocar no arquivo).
  let _ = std::fs::remove_file(thumb_cache_dir().join(format!("{id}.jpg")));
  catalog::remove_photo(id).map_err(Error::from_reason)?;
  Ok(true)
}

/// Progresso de varredura emitido ao callback.
#[napi(object)]
pub struct ScanProgress {
  pub processed: i64,
  pub total: i64,
  pub current_file: String,
}

/// Varre uma pasta recursivamente emitindo progresso ao callback (async).
/// callback recebe { processed, total, current_file }.
#[napi]
pub async fn scan_folder_progress(
  dir: String,
  callback: napi::threadsafe_function::ThreadsafeFunction<
    ScanProgress,
    napi::threadsafe_function::ErrorStrategy::Fatal,
  >,
) -> Result<String> {
  // Coleta os caminhos primeiro (rápido) para ter o total.
  let paths: Vec<std::path::PathBuf> = walkdir::WalkDir::new(&dir)
    .follow_links(false)
    .into_iter()
    .filter_map(|e| e.ok())
    .filter(|e| e.file_type().is_file())
    .map(|e| e.path().to_path_buf())
    .filter(|p| crate::imageproc::is_photo_path(p))
    .collect();
  let total = paths.len() as i64;

  let conn = catalog::open().map_err(Error::from_reason)?;
  let mut result = crate::types::ScanResult {
    scanned: 0,
    added: 0,
    updated: 0,
    skipped: 0,
    errors: Vec::new(),
  };

  for (i, path) in paths.into_iter().enumerate() {
    result.scanned += 1;
    let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    match crate::imageproc::inspect_file(&path) {
      Ok(meta) => {
        let is_new = catalog::upsert_scan_photo(&conn, &meta).unwrap_or(false);
        if is_new {
          result.added += 1;
        } else {
          result.updated += 1;
        }
      }
      Err(e) => {
        result.errors.push(format!("{}: {}", path.display(), e));
      }
    }
    // Emite progresso (a cada arquivo).
    let prog = ScanProgress {
      processed: (i + 1) as i64,
      total,
      current_file: file_name,
    };
    let _ = callback.call(
      prog,
      napi::threadsafe_function::ThreadsafeFunctionCallMode::NonBlocking,
    );
  }

  let summary = serde_json::json!({
    "scanned": result.scanned,
    "added": result.added,
    "updated": result.updated,
    "skipped": result.skipped,
    "errors": result.errors.len(),
  })
  .to_string();
  Ok(summary)
}

#[cfg(test)]
mod tests {
  #[test]
  fn hello_contains_name() {
    let msg = super::hello("Test".to_string());
    assert!(msg.contains("Test"));
    assert!(msg.contains("OK"));
  }

  #[test]
  fn add_works() {
    assert_eq!(super::add(2, 3), 5);
  }

  #[test]
  fn version_non_empty() {
    assert!(!super::core_version().is_empty());
  }

  #[test]
  fn is_photo_extension() {
    assert!(super::imageproc::is_photo_path(std::path::Path::new("a.jpg")));
    assert!(super::imageproc::is_photo_path(std::path::Path::new("a.CR3")));
    assert!(!super::imageproc::is_photo_path(std::path::Path::new("a.txt")));
  }

  #[test]
  fn duplicates_grouped_by_sha256() {
    let dir = std::env::temp_dir().join(format!("openshoot_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok();
    if let Err(e) = super::catalog::init(dir.to_str().unwrap()) {
      eprintln!("init reutilizado: {e}");
    }
    let conn = super::catalog::open().expect("open catalog");
    // Limpa resíduos de execuções anteriores.
    conn
      .execute("DELETE FROM photos", [])
      .unwrap();
    // Duas fotos com o MESMO sha256 e uma única.
    let sql = "INSERT INTO photos (path, file_name, ext, file_size, sha256, indexed_at)
               VALUES (?1, ?2, 'jpg', 100, ?3, datetime('now'))";
    conn
      .execute(sql, rusqlite::params!["/tmp/dup_a.jpg", "dup_a.jpg", "AAAA"])
      .unwrap();
    conn
      .execute(sql, rusqlite::params!["/tmp/dup_b.jpg", "dup_b.jpg", "AAAA"])
      .unwrap();
    conn
      .execute(sql, rusqlite::params!["/tmp/uniq.jpg", "uniq.jpg", "BBBB"])
      .unwrap();
    let groups = super::catalog::find_duplicates().expect("find_duplicates");
    assert_eq!(groups.len(), 1, "deve haver 1 grupo de duplicatas");
    assert_eq!(groups[0].photo_ids.len(), 2, "grupo com 2 fotos");
    assert_eq!(groups[0].hash, "AAAA");
    // Filtro "duplicates" deve retornar apenas as 2 duplicatas.
    let list = super::catalog::list_photos("", "duplicates", 0, 100).expect("list duplicates");
    assert_eq!(list.total, 2);

    // ---- Bucket "Para revisão" ----
    let id_a: i64 = conn
      .query_row("SELECT id FROM photos WHERE file_name='dup_a.jpg'", [], |r| r.get(0))
      .unwrap();
    super::catalog::set_photo_review(id_a, true).unwrap();
    let rev = super::catalog::list_photos("", "review", 0, 100).expect("list review");
    assert_eq!(rev.total, 1);
    assert_eq!(rev.photos[0].id, id_a);
    // Filtro de orientação retrato.
    conn
      .execute("UPDATE photos SET width=100, height=200 WHERE id=?1", [id_a])
      .unwrap();
    let portrait = super::catalog::list_photos("", "portrait", 0, 100).expect("list portrait");
    assert_eq!(portrait.total, 1);
    // Filtro RAW vs JPEG (dup_a.jpg é 'jpg').
    let jpeg = super::catalog::list_photos("", "jpeg", 0, 100).expect("list jpeg");
    assert_eq!(jpeg.total, 3);
    let raw = super::catalog::list_photos("", "raw", 0, 100).expect("list raw");
    assert_eq!(raw.total, 0);

    // ---- Destaques IA vs Selecionado manual ----
    super::catalog::set_photo_ai_pick(id_a, true).unwrap();
    let destaques = super::catalog::list_photos("", "destaques", 0, 100).expect("list destaques");
    assert_eq!(destaques.total, 1);
    assert_eq!(destaques.photos[0].id, id_a);
    // "Selecionado" = rating>=4 sem ser ai_pick → zera (rating ainda 0).
    let selecionado = super::catalog::list_photos("", "selecionado", 0, 100).expect("list selecionado");
    assert_eq!(selecionado.total, 0);

    // ---- Filtro "Editar status" ----
    super::catalog::set_photo_edit(id_a, r#"{"exposure":0.5}"#).unwrap();
    let edited = super::catalog::list_photos("", "edited", 0, 100).expect("list edited");
    assert_eq!(edited.total, 1);
    let unedited = super::catalog::list_photos("", "unedited", 0, 100).expect("list unedited");
    assert_eq!(unedited.total, 2);

    // ---- Presets nomeados ----
    super::catalog::save_preset("Meu Estilo", r#"{"contrast":20}"#).unwrap();
    let presets = super::catalog::list_presets().expect("list presets");
    assert!(presets.iter().any(|p| p.name == "Meu Estilo"));
    // Upsert (mesmo nome sobrescreve).
    super::catalog::save_preset("Meu Estilo", r#"{"contrast":30}"#).unwrap();
    let presets2 = super::catalog::list_presets().expect("list presets 2");
    let mine = presets2.iter().find(|p| p.name == "Meu Estilo").unwrap();
    assert!(mine.recipe.contains("30"));
    // Delete.
    assert!(super::catalog::delete_preset("Meu Estilo").unwrap());

    // Limpeza.
    conn.execute("DELETE FROM photos", []).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
  }
}
