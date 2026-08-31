#[macro_use]
extern crate napi_derive;

mod catalog;
mod captions;
mod cr3;
mod culling;
mod edit;
mod gallery;
mod geometric;
mod grouping;
mod imageproc;
mod lrimport;
mod ml;
mod retouch;
mod types;
mod upscale;
mod xmp;

use napi::bindgen_prelude::*;
use rayon::prelude::*;
use std::path::PathBuf;

use base64::Engine;

use types::{Album, DuplicateGroup, FilterCounts, PhotoList, PhotoMeta, Preset, ScanResult};

/// Inicializa o core: diretório de dados + catálogo SQLite.
/// data_dir: caminho absoluto. Retorna o caminho do banco criado.
#[napi]
pub fn setup(data_dir: String) -> Result<String> {
  let path = catalog::init(&data_dir)
    .map(|p| p.to_string_lossy().to_string())
    .map_err(|e| Error::from_reason(e))?;
  ensure_extra_columns()?;
  Ok(path)
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
pub fn scan_folder(
  dir: String,
  include_subdirs: Option<bool>,
  types: Option<String>,
) -> Result<ScanResult> {
  catalog::scan_folder(&dir, include_subdirs.unwrap_or(true), &types.unwrap_or_else(|| "all".into()))
    .map_err(|e| Error::from_reason(e))
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
pub fn filter_counts(photo_ids: Option<Vec<i64>>) -> Result<FilterCounts> {
  catalog::filter_counts(photo_ids.as_deref()).map_err(|e| Error::from_reason(e))
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

/// Aprende um perfil de estilo a partir das fotos com edição salva (média).
#[napi]
pub fn learn_profile() -> Result<serde_json::Value> {
  match catalog::learn_profile() {
    Ok((name, photos)) => {
      Ok(serde_json::json!({ "name": name, "photos": photos, "ok": true }))
    }
    Err(e) => Ok(serde_json::json!({ "error": e, "ok": false })),
  }
}

/// Define o tipo de sessão (gênero) de todas as fotos sob um caminho.
/// Retorna JSON { ok, updated }.
#[napi]
pub fn set_session_type(path_prefix: String, session_type: String) -> Result<serde_json::Value> {
  match catalog::set_session_type_for_path(&path_prefix, &session_type) {
    Ok(updated) => Ok(serde_json::json!({ "ok": true, "updated": updated })),
    Err(e) => Ok(serde_json::json!({ "ok": false, "error": e })),
  }
}

/// Importa um preset do Lightroom (.xmp com crs: ou .lrtemplate) e salva como
/// preset OpenShoot. Retorna JSON { ok, name, recipe }.
#[napi]
pub fn import_lightroom_preset(
  path: String,
  name: Option<String>,
) -> Result<serde_json::Value> {
  let p = PathBuf::from(&path);
  match lrimport::import_lightroom_preset(&p) {
    Ok(recipe) => {
      let preset_name = name.unwrap_or_else(|| {
        p.file_stem()
          .map(|s| s.to_string_lossy().to_string())
          .unwrap_or_else(|| "Preset Lightroom".to_string())
      });
      let result = catalog::save_preset(&preset_name, &recipe);
      match result {
        Ok(()) => Ok(serde_json::json!({ "ok": true, "name": preset_name, "recipe": recipe })),
        Err(e) => Ok(serde_json::json!({ "ok": false, "error": e })),
      }
    }
    Err(e) => Ok(serde_json::json!({ "ok": false, "error": e })),
  }
}

/// Exporta um preset para um arquivo JSON (estilo compartilhável).
#[napi]
pub fn export_preset_to_file(name: String, dest: String) -> Result<serde_json::Value> {
  match catalog::export_preset_to_file(&name, &PathBuf::from(&dest)) {
    Ok(()) => Ok(serde_json::json!({ "ok": true, "name": name })),
    Err(e) => Ok(serde_json::json!({ "ok": false, "error": e })),
  }
}

/// Importa um preset de um arquivo JSON (estilo compartilhável).
#[napi]
 pub fn import_preset_from_file(path: String) -> Result<serde_json::Value> {
  match catalog::import_preset_from_file(&PathBuf::from(&path)) {
    Ok(name) => Ok(serde_json::json!({ "ok": true, "name": name })),
    Err(e) => Ok(serde_json::json!({ "ok": false, "error": e })),
  }
}

// ---------------- Álbuns ----------------

/// Cria um álbum. Retorna o id.
#[napi]
pub fn create_album(name: String) -> Result<i64> {
  catalog::create_album(&name).map_err(|e| Error::from_reason(e))
}

/// Lista álbuns (com contagem e capa).
#[napi]
pub fn list_albums() -> Result<Vec<Album>> {
  catalog::list_albums().map_err(|e| Error::from_reason(e))
}

/// Remove um álbum (não toca nas fotos).
#[napi]
pub fn delete_album(id: i64) -> Result<bool> {
  catalog::delete_album(id).map_err(|e| Error::from_reason(e))
}

/// Associa fotos (por id) a um álbum.
#[napi]
pub fn add_photos_to_album(album_id: i64, photo_ids: Vec<i64>) -> Result<i64> {
  catalog::add_photos_to_album(album_id, &photo_ids).map_err(|e| Error::from_reason(e))
}

/// Associa todas as fotos de um diretório a um álbum.
#[napi]
pub fn add_folder_to_album(album_id: i64, dir: String) -> Result<i64> {
  catalog::add_folder_to_album(album_id, &dir).map_err(|e| Error::from_reason(e))
}

/// Define o tipo de sessão de um álbum.
#[napi]
pub fn set_album_session_type(album_id: i64, session_type: String) -> Result<()> {
  catalog::set_album_session_type(album_id, &session_type).map_err(|e| Error::from_reason(e))
}

/// Retorna os ids de fotos de um álbum.
#[napi]
pub fn album_photo_ids(album_id: i64) -> Result<Vec<i64>> {
  catalog::album_photo_ids(album_id).map_err(|e| Error::from_reason(e))
}

/// Máscara de sujeito: desfoca o fundo mantendo o sujeito (face + pele) nítido.
/// Detecta o rosto via SCRFD. Retorna o preview (base64).
#[napi]
pub async fn subject_mask_photo(
  id: i64,
  background_blur: f64,
  max_dim: u32,
) -> Result<Option<String>> {
  let photo = match catalog::get_photo(id) {
    Ok(Some(p)) => p,
    Ok(None) => return Ok(None),
    Err(e) => return Err(Error::from_reason(e)),
  };
  let path = PathBuf::from(&photo.path);
  let dim = if max_dim == 0 { 512 } else { max_dim };
  let blur = background_blur;
  tokio::task::spawn_blocking(move || {
    let img = image::ImageReader::open(&path)
      .ok()
      .and_then(|r| r.decode().ok())
      .map(|d| d.to_rgb8())
      .or_else(|| {
        crate::imageproc::read_embedded_jpeg(&path).and_then(|jpeg| {
          image::ImageReader::new(std::io::Cursor::new(jpeg))
            .with_guessed_format()
            .ok()
            .and_then(|r| r.decode().ok())
            .map(|d| d.to_rgb8())
        })
      })?;
    let (w, h) = img.dimensions();
    let rgb = img.as_raw();
    let bbox = if crate::ml::models_available() {
      crate::ml::detect_faces(rgb, w, h, 0.5)
        .ok()
        .and_then(|f| f.into_iter().next())
        .unwrap_or([0.2, 0.2, 0.8, 0.8])
    } else {
      [0.2, 0.2, 0.8, 0.8]
    };
    let out = retouch::subject_mask_base64(rgb, w, h, bbox, blur as f32);
    let img_out = image::RgbImage::from_raw(w, h, out)?;
    let dynimg = image::DynamicImage::ImageRgb8(img_out);
    let thumb = dynimg.thumbnail(dim, dim);
    let mut buf = std::io::Cursor::new(Vec::new());
    thumb.write_to(&mut buf, image::ImageFormat::Jpeg).ok()?;
    Some(format!(
      "data:image/jpeg;base64,{}",
      base64::engine::general_purpose::STANDARD.encode(buf.into_inner())
    ))
  })
  .await
  .map_err(|e| Error::from_reason(e.to_string()))
}

// ---------------- Reconhecimento facial / agrupamento ----------------

/// Agrupa fotos por pessoa (similaridade facial via MobileFaceNet).
/// Retorna JSON { ok, groups: [{person_id, count, sample_path, photo_ids, photo_paths}] }.
#[napi]
pub async fn group_by_similarity_async(
  threshold: Option<f64>,
  photo_ids: Option<Vec<i64>>,
) -> Result<serde_json::Value> {
  let paths_result = match photo_ids.as_deref() {
    Some(ids) => catalog::photo_paths_for_ids(ids),
    None => catalog::all_photo_paths(),
  };
  let paths = match paths_result {
    Ok(p) => p,
    Err(e) => return Err(Error::from_reason(e)),
  };
  let t = threshold.unwrap_or(0.5) as f32;
  tokio::task::spawn_blocking(move || {
    match grouping::group_by_similarity(&paths, t) {
      Ok(groups) => serde_json::to_value(groups).unwrap_or(serde_json::json!([])),
      Err(e) => serde_json::json!({ "error": e }),
    }
  })
  .await
  .map_err(|e| Error::from_reason(e.to_string()))
}

/// Exporta as fotos agrupadas por pessoa para pastas (Pessoa 1, Pessoa 2...).
/// Retorna JSON { ok, out_dir, groups, exported }.
#[napi]
pub async fn export_people_to_folders(
  out_dir: String,
  threshold: Option<f64>,
  photo_ids: Option<Vec<i64>>,
) -> Result<serde_json::Value> {
  let paths_result = match photo_ids.as_deref() {
    Some(ids) => catalog::photo_paths_for_ids(ids),
    None => catalog::all_photo_paths(),
  };
  let paths = match paths_result {
    Ok(p) => p,
    Err(e) => return Err(Error::from_reason(e)),
  };
  let t = threshold.unwrap_or(0.5) as f32;
  tokio::task::spawn_blocking(move || {
    grouping::export_grouped(&paths, &PathBuf::from(&out_dir), t)
      .unwrap_or_else(|e| serde_json::json!({ "ok": false, "error": e }))
  })
  .await
  .map_err(|e| Error::from_reason(e.to_string()))
}

/// Total de fotos no catálogo.
#[napi]
pub fn photo_count() -> Result<i64> {
  catalog::count_photos().map_err(|e| Error::from_reason(e))
}

/// Diretório de cache de thumbnails (liberável — pode apagar para liberar espaço).
/// Multi-plataforma: ~/Library/Caches (mac), %LOCALAPPDATA% (win), XDG (linux).
fn thumb_cache_dir() -> PathBuf {
  dirs::cache_dir()
    .unwrap_or_else(|| std::env::temp_dir())
    .join("OpenShoot/thumbs")
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
pub async fn cull_photos(target_picks: Option<i64>, photo_ids: Option<Vec<i64>>) -> Result<CullSummary> {
  let paths_result = match photo_ids.as_deref() {
    Some(ids) => catalog::photo_paths_for_ids(ids),
    None => catalog::all_photo_paths(),
  };
  let paths = match paths_result {
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
      // Decode ÚNICO em 640px, reusado por faces + NIMA + heurística (gap G3:
      // antes decodificava 3× por foto — 2,5s/foto no benchmark de 459 fotos).
      let decoded = if ml_ok { ml::load_rgb(&path, 640).ok() } else { None };

      // Detecção de rosto (SCRFD) para preencher has_face (usado no filtro "faces").
      let has_face = match &decoded {
        Some((rgb, w, h)) => ml::detect_faces(rgb, *w, *h, 0.5)
          .map(|faces| !faces.is_empty())
          .unwrap_or(false),
        None => false,
      };
      let score = if ml_ok {
        // IA: heurística + ML combinados
        let heur = match &decoded {
          Some((rgb, w, h)) => culling::heuristic_score_rgb(rgb, *w, *h, 320),
          None => culling::heuristic_score(&path, 320),
        };
        let mls = match &decoded {
          Some((rgb, w, h)) => ml::ml_quality_score(rgb, *w, *h),
          None => Err("decode falhou".to_string()),
        };
        match mls {
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
  let mut rated: Vec<(i64, i64)> = Vec::with_capacity(n);
  for (i, (id, s)) in scores.iter().enumerate() {
    let q = if n <= 1 { 1.0 } else { i as f64 / (n - 1) as f64 };
    let rating = if q < 0.2 { 1 } else if q < 0.4 { 2 } else if q < 0.6 { 3 } else if q < 0.8 { 4 } else { 5 };
    if let Err(e) = catalog::set_photo_rating(*id, rating, *s) {
      crate::catalog::log_debug(&format!("falha ao salvar rating {}: {e}", id));
    }
    rated.push((*id, rating));
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
    // Gap G4: picks alinhados ao rating por quantis (★4+), igual ao filtro
    // "picks" da UI — antes usava limiar fixo de score e divergia do grid.
    for (id, rating) in &rated {
      let is_pick = *rating >= 4;
      if let Err(e) = catalog::set_photo_ai_pick(*id, is_pick) {
        crate::catalog::log_debug(&format!("falha ao salvar ai_pick {}: {e}", id));
      }
    }
    picks = rated.iter().filter(|(_, r)| *r >= 4).count() as i64;
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

/// Detecta faces de uma foto do catálogo (por id). Retorna { faces: [[x0,y0,x1,y1]...] }.
#[napi]
pub fn detect_faces_in_photo(id: i64) -> Result<serde_json::Value> {
  let photo = match catalog::get_photo(id) {
    Ok(Some(p)) => p,
    Ok(None) => return Err(Error::from_reason(format!("foto {id} nao encontrada"))),
    Err(e) => return Err(Error::from_reason(e)),
  };
  let pb = PathBuf::from(&photo.path);
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

#[napi]
pub fn get_exif_detail(id: i64) -> Result<serde_json::Value> {
  let photo = match catalog::get_photo(id) {
    Ok(Some(p)) => p,
    Ok(None) => return Err(Error::from_reason(format!("foto {id} nao encontrada"))),
    Err(e) => return Err(Error::from_reason(e)),
  };
  let pb = PathBuf::from(&photo.path);
  let exif = imageproc::read_exif_detail(&pb);
  serde_json::json!({
    "iso": exif.iso,
    "aperture": exif.aperture,
    "focal_length": exif.focal_length,
    "shutter_speed": exif.shutter_speed,
    "lens": exif.lens,
    "flash": exif.flash,
    "white_balance": exif.white_balance,
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

/// Exporta fotos (com edição aplicada) para uma pasta de destino.
/// `ids`: fotos a exportar; `dest_dir`: pasta destino; `format`: "jpeg"|"png";
/// `quality`: 1..100; `color_profile`: "srgb"|"display-p3";
/// `naming`: "{original}"|"{n}_{original}"|"{date}_{original}".
/// Retorna { ok, exported, errors, files }.
#[napi]
pub fn export_photos(
  ids: Vec<i64>,
  dest_dir: String,
  format: String,
  quality: i64,
  color_profile: String,
  naming: String,
) -> Result<serde_json::Value> {
  let dest = PathBuf::from(&dest_dir);
  if let Err(e) = std::fs::create_dir_all(&dest) {
    return Ok(serde_json::json!({ "ok": false, "error": format!("criar pasta: {e}") }));
  }
  let fmt = if format.eq_ignore_ascii_case("png") { "png" } else { "jpeg" };
  let ext = if fmt == "png" { "png" } else { "jpg" };
  let q = quality.clamp(1, 100) as u8;

  // ---- Fase serial: metadados + receita + nome-base ----
  // SQLite não é thread-safe para conexão compartilhada; get_photo/get_photo_edit
  // rodam aqui, na thread principal. O trabalho CPU-bound (decode+edit+save)
  // fica para o par_iter abaixo.
  struct ExportJob {
    src: PathBuf,
    params: edit::EditParams,
    base_name: String,
  }
  let mut jobs: Vec<ExportJob> = Vec::new();
  let mut skipped = 0i64;
  {
    let mut seq = 0usize; // contador global para naming "{n}_{original}"
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
    for id in ids {
      let photo = match catalog::get_photo(id) {
        Ok(Some(p)) => p,
        _ => {
          skipped += 1;
          continue;
        }
      };
      // Carrega a receita de edição salva (se houver).
      let params = match catalog::get_photo_edit(id) {
        Ok(json) if !json.is_empty() => {
          serde_json::from_str::<edit::EditParams>(&json).unwrap_or_default()
        }
        _ => edit::EditParams::default(),
      };
      let src = PathBuf::from(&photo.path);
      // Nomeação: gera o nome base antes do sufixo de conflito.
      seq += 1;
      let stem = src
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("foto_{id}"));
      let mut base_name = edit::build_export_name(&naming, seq, photo.taken_at.as_deref(), &stem);
      // Garante nomes únicos dentro do próprio lote (mesma ordem do modo serial).
      let mut k = 1;
      while !used.insert(format!("{base_name}.{ext}")) {
        base_name = format!("{base_name}_{k}");
        k += 1;
      }
      jobs.push(ExportJob { src, params, base_name });
    }
  }

  // ---- Fase paralela (rayon): decode + edit + save por foto (CPU-bound) ----
  // Contador atômico global para sufixos de conflito com arquivos já existentes.
  let next_suffix = std::sync::atomic::AtomicI64::new(1);
  let results: Vec<Option<String>> = jobs
    .par_iter()
    .map(|job| {
      let mut dest_path = dest.join(format!("{}.{ext}", job.base_name));
      while dest_path.exists() {
        let n = next_suffix.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        dest_path = dest.join(format!("{}_{}.{ext}", job.base_name, n));
      }
      match edit::export_photo_to_file(&job.src, &job.params, &dest_path, fmt, q, &color_profile) {
        Ok(()) => Some(dest_path.display().to_string()),
        Err(e) => {
          crate::catalog::log_debug(&format!("[export] {}: {e}", job.src.display()));
          None
        }
      }
    })
    .collect();

  let exported = results.iter().flatten().count() as i64;
  let errors = skipped + (results.len() as i64 - exported);
  let files: Vec<String> = results.into_iter().flatten().collect();
  Ok(serde_json::json!({
    "ok": true,
    "exported": exported,
    "errors": errors,
    "files": files,
    "dest_dir": dest.display().to_string(),
  }))
}

/// Aplica retoque (pele + regiões faciais) em lote às fotos e grava em pasta.
/// `skin_intensity` 0..1; `face_regions`: { "acne":0.5, "olhos":0.3, ... }.
/// Não-destrutivo: grava cópias retocadas no destino.
#[napi]
pub fn apply_retouch_all(
  ids: Vec<i64>,
  dest_dir: String,
  skin_intensity: f64,
  face_regions: serde_json::Value,
  format: String,
  quality: i64,
) -> Result<serde_json::Value> {
  let dest = PathBuf::from(&dest_dir);
  if let Err(e) = std::fs::create_dir_all(&dest) {
    return Ok(serde_json::json!({ "ok": false, "error": format!("criar pasta: {e}") }));
  }
  let fmt = if format.eq_ignore_ascii_case("png") { "png" } else { "jpeg" };
  let ext = if fmt == "png" { "png" } else { "jpg" };
  let q = quality.clamp(1, 100) as u8;
  let skin = skin_intensity.clamp(0.0, 1.0) as f32;

  // Lê as regiões faciais do JSON (mapa região → intensidade).
  let mut regions: Vec<(String, f32)> = Vec::new();
  if let serde_json::Value::Object(map) = &face_regions {
    for (k, v) in map {
      if let Some(f) = v.as_f64() {
        if f > 0.0 {
          regions.push((k.clone(), f as f32));
        }
      }
    }
  }

  let mut jobs: Vec<(PathBuf, String)> = Vec::new(); // (src, stem único no lote)
  let mut skipped = 0i64;
  {
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
    for id in ids {
      let photo = match catalog::get_photo(id) {
        Ok(Some(p)) => p,
        _ => {
          skipped += 1;
          continue;
        }
      };
      let src = PathBuf::from(&photo.path);
      let stem = src
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("foto_{id}"));
      // Garante nomes únicos dentro do próprio lote (mesma ordem do modo serial).
      let mut name = stem.clone();
      let mut k = 1;
      while !used.insert(format!("{name}.{ext}")) {
        name = format!("{stem}_{k}");
        k += 1;
      }
      jobs.push((src, name));
    }
  }

  // ---- Fase paralela (rayon): retouch não toca SQLite no loop ----
  // Contador atômico global para sufixos de conflito com arquivos já existentes.
  let next_suffix = std::sync::atomic::AtomicI64::new(1);
  let results: Vec<Option<String>> = jobs
    .par_iter()
    .map(|(src, name)| {
      let mut dest_path = dest.join(format!("{name}.{ext}"));
      while dest_path.exists() {
        let n = next_suffix.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        dest_path = dest.join(format!("{name}_{n}.{ext}"));
      }
      let mut ok = true;
      // Pele primeiro.
      if skin > 0.0 {
        if let Err(e) = retouch::retouch_skin_to_file(src, skin, &dest_path, fmt, q) {
          crate::catalog::log_debug(&format!("[retouch] pele {}: {e}", src.display()));
          ok = false;
        }
      }
      // Depois regiões faciais (empilhadas na mesma saída).
      if ok {
        for (region, intensity) in &regions {
          // tmp único por thread para não colidir entre workers do rayon.
          let tmp =
            dest.join(format!("__tmp_{}_{:?}.{ext}", name, std::thread::current().id()));
          if let Err(e) = retouch::retouch_face_to_file(src, region, *intensity, &tmp, fmt, q) {
            crate::catalog::log_debug(&format!("[retouch] {region} {}: {e}", src.display()));
            continue;
          }
          // Re-carrega a tmp como base para o próximo ajuste e move para dest.
          let _ = std::fs::copy(&tmp, &dest_path);
          let _ = std::fs::remove_file(&tmp);
        }
      }
      if ok {
        Some(dest_path.display().to_string())
      } else {
        None
      }
    })
    .collect();

  let exported = results.iter().flatten().count() as i64;
  let errors = skipped + (results.len() as i64 - exported);
  let files: Vec<String> = results.into_iter().flatten().collect();
  Ok(serde_json::json!({
    "ok": true,
    "exported": exported,
    "errors": errors,
    "files": files,
    "dest_dir": dest.display().to_string(),
  }))
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
pub async fn auto_level_photo(id: i64, max_dim: u32) -> Result<serde_json::Value> {  let photo = match catalog::get_photo(id) {
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

/// Retoque de região facial (acne/olhos/dentes/cabelo) dentro da bbox do rosto.
/// Detecta o rosto via SCRFD e aplica o ajuste direcionado.
#[napi]
pub async fn retouch_face_photo(
  id: i64,
  region: String,
  intensity: f64,
  max_dim: u32,
) -> Result<Option<String>> {
  let photo = match catalog::get_photo(id) {
    Ok(Some(p)) => p,
    Ok(None) => return Ok(None),
    Err(e) => return Err(Error::from_reason(e)),
  };
  let path = PathBuf::from(&photo.path);
  let dim = if max_dim == 0 { 512 } else { max_dim };
  let region_name = region;
  tokio::task::spawn_blocking(move || {
    let img = crate::imageproc::read_embedded_jpeg(&path)
      .or_else(|| None);
    let rgb_result = if let Some(jpeg) = img {
      image::ImageReader::new(std::io::Cursor::new(jpeg))
        .with_guessed_format()
        .ok()
        .and_then(|r| r.decode().ok())
        .map(|d| d.to_rgb8())
    } else {
      image::ImageReader::open(&path)
        .ok()
        .and_then(|r| r.decode().ok())
        .map(|d| d.to_rgb8())
    };
    let img = match rgb_result {
      Some(i) => i,
      None => return None,
    };
    let (w, h) = img.dimensions();
    let rgb = img.as_raw();
    // Detecta o primeiro rosto (se modelos disponíveis).
    let bbox = if crate::ml::models_available() {
      crate::ml::detect_faces(rgb, w, h, 0.5)
        .ok()
        .and_then(|f| f.into_iter().next())
        .unwrap_or([0.0, 0.0, 1.0, 1.0])
    } else {
      [0.0, 0.0, 1.0, 1.0]
    };
    let out = retouch::retouch_face_region(rgb, w, h, bbox, &region_name, intensity as f32);
    let img_out = image::RgbImage::from_raw(w, h, out)?;
    let dynimg = image::DynamicImage::ImageRgb8(img_out);
    let thumb = dynimg.thumbnail(dim, dim);
    let mut buf = std::io::Cursor::new(Vec::new());
    thumb.write_to(&mut buf, image::ImageFormat::Jpeg).ok()?;
    Some(format!(
      "data:image/jpeg;base64,{}",
      base64::engine::general_purpose::STANDARD.encode(buf.into_inner())
    ))
  })
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
/// Move um arquivo para a LIXEIRA nativa do sistema (macOS Finder, Windows
/// Recycle Bin, Linux freedesktop). Multi-plataforma via crate `trash`.
fn move_to_trash(src: &std::path::Path) -> std::result::Result<(), String> {
  trash::delete(src).map_err(|e| format!("falha ao mover p/ lixeira: {e}"))
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
  include_subdirs: Option<bool>,
  types: Option<String>,
  callback: napi::threadsafe_function::ThreadsafeFunction<
    ScanProgress,
    napi::threadsafe_function::ErrorStrategy::Fatal,
  >,
) -> Result<String> {
  let include_subdirs = include_subdirs.unwrap_or(true);
  let types = types.unwrap_or_else(|| "all".into());
  // Coleta os caminhos primeiro (rápido) para ter o total.
  let mut walker = walkdir::WalkDir::new(&dir).follow_links(false);
  if !include_subdirs {
    walker = walker.max_depth(1);
  }
  let paths: Vec<std::path::PathBuf> = walker
    .into_iter()
    .filter_map(|e| e.ok())
    .filter(|e| e.file_type().is_file())
    .map(|e| e.path().to_path_buf())
    .filter(|p| crate::imageproc::is_photo_path(p))
    .filter(|p| catalog::matches_photo_type(p, &types))
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

// ---------------- Identidade de perfis (presets, estilo AfterShoot) ----------------

/// Salva um preset com metadados de identidade: file_type ('raw'|'jpeg'|''),
/// color_type ('color'|'bw'|''), source ('manual'|'learned'|'lightroom'|'imported').
#[napi]
pub fn save_preset_full(
  name: String,
  recipe: String,
  file_type: String,
  color_type: String,
  source: String,
) -> Result<()> {
  catalog::save_preset_full(&name, &recipe, &file_type, &color_type, &source)
    .map_err(|e| Error::from_reason(e))
}

/// Atualiza os metadados de identidade de um preset existente.
#[napi]
pub fn update_preset_meta(name: String, file_type: String, color_type: String) -> Result<bool> {
  catalog::update_preset_meta(&name, &file_type, &color_type).map_err(|e| Error::from_reason(e))
}

// --- Galeria web (agent-05) ---

/// Cria uma galeria web estática (equivalente ao "Criar galeria" do AfterShoot).
/// Copia as fotos para `dest_dir/photos/`, gera thumbnails 400px em
/// `dest_dir/thumbs/` e escreve um `index.html` self-contained (dark theme,
/// grid responsivo, lightbox CSS). Retorna { ok, path, count }.
#[napi]
pub fn create_web_gallery(ids: Vec<i64>, dest_dir: String, title: String) -> Result<serde_json::Value> {
  let title = title.trim().to_string();
  if title.is_empty() {
    return Ok(serde_json::json!({ "ok": false, "error": "título vazio" }));
  }
  let dest = PathBuf::from(&dest_dir);
  if let Err(e) = std::fs::create_dir_all(&dest) {
    return Ok(serde_json::json!({ "ok": false, "error": format!("criar pasta: {e}") }));
  }
  let photos_dir = dest.join("photos");
  let thumbs_dir = dest.join("thumbs");
  if let Err(e) = std::fs::create_dir_all(&photos_dir) {
    return Ok(serde_json::json!({ "ok": false, "error": format!("criar photos/: {e}") }));
  }
  if let Err(e) = std::fs::create_dir_all(&thumbs_dir) {
    return Ok(serde_json::json!({ "ok": false, "error": format!("criar thumbs/: {e}") }));
  }

  let mut items: Vec<(String, String)> = Vec::new();
  for id in ids {
    let photo = match catalog::get_photo(id) {
      Ok(Some(p)) => p,
      _ => {
        crate::catalog::log_debug(&format!("[gallery] foto {id} não encontrada"));
        continue;
      }
    };
    let src = PathBuf::from(&photo.path);
    // Nome único dentro de photos/ (evita sobrescrever conflitos).
    let stem = src
      .file_stem()
      .map(|s| s.to_string_lossy().to_string())
      .unwrap_or_else(|| format!("foto_{id}"));
    let ext = src
      .extension()
      .map(|e| e.to_string_lossy().to_lowercase())
      .unwrap_or_else(|| "jpg".to_string());
    let mut dest_name = format!("{stem}.{ext}");
    let mut counter = 1;
    while photos_dir.join(&dest_name).exists() {
      dest_name = format!("{stem}_{counter}.{ext}");
      counter += 1;
    }
    let thumb_name = {
      let stem_thumb = dest_name
        .rsplit_once('.')
        .map(|(s, _)| s.to_string())
        .unwrap_or_else(|| dest_name.clone());
      format!("{stem_thumb}.jpg")
    };
    if std::fs::copy(&src, photos_dir.join(&dest_name)).is_err() {
      crate::catalog::log_debug(&format!("[gallery] falha ao copiar {}", photo.path));
      continue;
    }
    // Thumbnail ~400px (JPEG). Para RAW, usa o preview JPEG embutido.
    match image::ImageReader::open(&src)
      .ok()
      .and_then(|r| r.with_guessed_format().ok())
      .and_then(|r| r.decode().ok())
      .or_else(|| {
        crate::imageproc::read_embedded_jpeg(&src).and_then(|jpeg| {
          image::ImageReader::new(std::io::Cursor::new(jpeg))
            .with_guessed_format()
            .ok()
            .and_then(|r| r.decode().ok())
        })
      })
      .map(|d| d.thumbnail(400, 400))
    {
      Some(thumb) => {
        if let Err(e) = thumb.save(thumbs_dir.join(&thumb_name)) {
          crate::catalog::log_debug(&format!("[gallery] falha ao gravar thumb {}: {e}", dest_name));
          continue;
        }
      }
      None => {
        crate::catalog::log_debug(&format!("[gallery] sem thumbnail para {}", photo.path));
        continue;
      }
    }
    items.push((format!("photos/{dest_name}"), photo.file_name));
  }

  if items.is_empty() {
    return Ok(serde_json::json!({ "ok": false, "error": "nenhuma foto pôde ser exportada" }));
  }

  let html = match gallery::generate_html(&items, &title) {
    Ok(h) => h,
    Err(e) => return Ok(serde_json::json!({ "ok": false, "error": e })),
  };
  let index_path = dest.join("index.html");
  if let Err(e) = std::fs::write(&index_path, html) {
    return Ok(serde_json::json!({ "ok": false, "error": format!("gravar index.html: {e}") }));
  }

  Ok(serde_json::json!({
    "ok": true,
    "path": index_path.display().to_string(),
      "count": items.len(),
    }))
  }

  // ---------------- Fase 7: upscale / enhance (ref. Upscayl) ----------------

  /// Indica se o modelo ONNX de upscale está disponível (cai no fallback
  /// bicúbico quando não). `model_name` vazio -> modelo padrão (4x-UltraSharp).
  #[napi]
  pub fn upscale_available(model_name: Option<String>) -> bool {
    let m = model_name
      .filter(|s| !s.is_empty())
      .unwrap_or_else(|| crate::upscale::upscale_default_model().to_string());
    crate::upscale::upscale_model_available(&m)
  }

  /// Gera um preview de upscale (base64) para uma foto do catálogo.
  /// `model_name` vazio -> padrão; `scale` 1..4 (modelo nativo 4x, 2x/3x via
  /// pós-redimensionamento); `max_dim` limita o preview retornado.
  /// Sem modelo: fallback bicúbico (recurso mais simples, mesma API).
  #[napi]
  pub async fn upscale_photo(
    id: i64,
    model_name: Option<String>,
    scale: Option<u32>,
    max_dim: u32,
  ) -> Result<Option<String>> {
    let photo = match catalog::get_photo(id) {
      Ok(Some(p)) => p,
      Ok(None) => return Ok(None),
      Err(e) => return Err(Error::from_reason(e)),
    };
    let path = PathBuf::from(&photo.path);
    let model = model_name
      .filter(|s| !s.is_empty())
      .unwrap_or_else(|| crate::upscale::upscale_default_model().to_string());
    let scale = scale.unwrap_or(4).clamp(1, 4);
    let dim = if max_dim == 0 { 512 } else { max_dim };
    tokio::task::spawn_blocking(move || {
      // Preview: decodifica capado (~1000px) p/ não estourar memória no 4x.
      let (rgb, w, h) = crate::ml::load_rgb(&path, 1000).ok()?;
      let out = crate::upscale::upscale_rgb(&rgb, w, h, &model, scale);
      let img = image::RgbImage::from_raw(w * scale, h * scale, out)?;
      let thumb = image::DynamicImage::ImageRgb8(img).thumbnail(dim, dim);
      let mut buf = std::io::Cursor::new(Vec::new());
      thumb.write_to(&mut buf, image::ImageFormat::Jpeg).ok()?;
      Some(format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(buf.into_inner())
      ))
    })
    .await
    .map_err(|e| Error::from_reason(e.to_string()))
  }

  /// Progresso de upscale em lote.
  #[napi(object)]
  pub struct UpscaleProgress {
    pub processed: i64,
    pub total: i64,
    pub current_file: String,
  }

  /// Upscale em lote (sequencial — VRAM de GPU é limitada, igual ao Upscayl)
  /// e grava as fotos aumentadas em `dest_dir`. Retorna JSON { ok, exported,
  /// errors, files }. `callback` recebe { processed, total, current_file }.
  #[napi]
  pub async fn export_upscaled(
    ids: Vec<i64>,
    dest_dir: String,
    model_name: Option<String>,
    scale: Option<u32>,
    format: String,
    quality: i64,
    callback: napi::threadsafe_function::ThreadsafeFunction<
      UpscaleProgress,
      napi::threadsafe_function::ErrorStrategy::Fatal,
    >,
  ) -> Result<String> {
    let model = model_name
      .filter(|s| !s.is_empty())
      .unwrap_or_else(|| crate::upscale::upscale_default_model().to_string());
    let scale = scale.unwrap_or(4).clamp(1, 4);
    let fmt = if format.eq_ignore_ascii_case("png") { "png" } else { "jpeg" };
    let q = quality.clamp(1, 100) as u8;

    // Fase serial: metadados do catálogo (SQLite não é thread-safe compartilhado).
    let mut jobs: Vec<(PathBuf, String)> = Vec::new();
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
    for id in &ids {
      let photo = match catalog::get_photo(*id) {
        Ok(Some(p)) => p,
        _ => continue,
      };
      let src = PathBuf::from(&photo.path);
      let stem = src
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("foto_{id}"));
      let mut name = stem.clone();
      let mut k = 1;
      while !used.insert(format!("{name}.{fmt}")) {
        name = format!("{stem}_{k}");
        k += 1;
      }
      jobs.push((src, name));
    }
    let total = jobs.len() as i64;
    let dest = PathBuf::from(&dest_dir);
    if let Err(e) = std::fs::create_dir_all(&dest) {
      return Ok(
        serde_json::json!({ "ok": false, "error": format!("criar pasta: {e}") }).to_string(),
      );
    }

    let res = tokio::task::spawn_blocking(move || {
      let mut processed = 0i64;
      let mut errors = 0i64;
      let mut files: Vec<String> = Vec::new();
      for (i, (src, name)) in jobs.into_iter().enumerate() {
        let (rgb, w, h) = match crate::upscale::decode_full_rgb(&src) {
          Some(v) => v,
          None => {
            errors += 1;
            continue;
          }
        };
        let out = crate::upscale::upscale_rgb(&rgb, w, h, &model, scale);
        let ow = w * scale;
        let oh = h * scale;
        let ext = if fmt == "png" { "png" } else { "jpg" };
        let dpath = dest.join(format!("{name}.{ext}"));
        if let Err(e) = crate::upscale::save_rgb(&dpath, out, ow, oh, fmt, q) {
          crate::catalog::log_debug(&format!("[upscale] {}: {e}", src.display()));
          errors += 1;
        } else {
          files.push(dpath.display().to_string());
          processed += 1;
        }
        let prog = UpscaleProgress {
          processed: (i + 1) as i64,
          total,
          current_file: src
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        };
        let _ = callback.call(
          prog,
          napi::threadsafe_function::ThreadsafeFunctionCallMode::NonBlocking,
        );
      }
      serde_json::json!({
        "ok": true,
        "exported": processed,
        "errors": errors,
        "files": files,
        "dest_dir": dest.display().to_string(),
      })
      .to_string()
    })
    .await
    .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(res)
  }

  #[cfg(test)]
  mod tests {
  use std::sync::{Mutex, OnceLock};

  /// Serializa os testes que usam o catalog.db (OnceLock compartilhado entre
  /// eles): evita corridas de contagem global e SQLITE_BUSY.
  pub fn db_lock() -> std::sync::MutexGuard<'static, ()> {
    static DB_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    DB_LOCK
      .get_or_init(|| Mutex::new(()))
      .lock()
      .unwrap_or_else(|e| e.into_inner())
  }

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
  fn export_photos_parallel_exports_all() {
    let _db = db_lock();
    let dir = std::env::temp_dir().join(format!("openshoot_export_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok();
    if let Err(e) = super::catalog::init(dir.to_str().unwrap()) {
      eprintln!("init reutilizado: {e}");
    }
    let conn = super::catalog::open().expect("open catalog");
    conn.busy_timeout(std::time::Duration::from_secs(5)).ok();
    // Limpa resíduos de execuções anteriores (escopo deste teste — outros
    // testes rodam em paralelo na mesma thread de BD via OnceLock).
    conn
      .execute("DELETE FROM photos WHERE file_name LIKE 'synth_%'", [])
      .unwrap();
    let sql = "INSERT INTO photos (path, file_name, ext, file_size, sha256, indexed_at)
               VALUES (?1, ?2, 'png', 100, ?3, datetime('now'))";
    let mut ids: Vec<i64> = Vec::new();
    for i in 0..4 {
      let img = image::RgbImage::from_fn(8, 8, |x, y| {
        image::Rgb([
          ((x * 13 + i * 29) % 255) as u8,
          ((y * 7 + i * 11) % 255) as u8,
          ((i * 61) % 255) as u8,
        ])
      });
      let path = dir.join(format!("synth_{i}.png"));
      image::DynamicImage::ImageRgb8(img)
        .save_with_format(&path, image::ImageFormat::Png)
        .expect("gravar PNG sintético");
      conn
        .execute(
          sql,
          rusqlite::params![
            path.to_string_lossy(),
            format!("synth_{i}.png"),
            format!("SYNTH{i}")
          ],
        )
        .unwrap();
      let id: i64 = conn
        .query_row(
          "SELECT id FROM photos WHERE file_name=?1",
          [format!("synth_{i}.png")],
          |r| r.get(0),
        )
        .unwrap();
      ids.push(id);
    }

    // Exporta os 4 em lote (paralelo com rayon) para uma subpasta.
    let dest = dir.join("out_parallel");
    let res = super::export_photos(
      ids,
      dest.to_string_lossy().to_string(),
      "png".to_string(),
      90,
      "srgb".to_string(),
      "{original}".to_string(),
    )
    .expect("export_photos");
    assert_eq!(res["exported"].as_i64(), Some(4), "exported == 4");
    assert_eq!(res["errors"].as_i64(), Some(0), "sem erros");
    assert_eq!(res["files"].as_array().map(|a| a.len()), Some(4));
    for i in 0..4 {
      assert!(dest.join(format!("synth_{i}.png")).exists(), "arquivo {i} exportado");
    }

    // Limpeza: apenas os artefatos deste teste — NUNCA remover o diretório
    // que pode conter o catalog.db ativo compartilhado via OnceLock.
    conn.execute("DELETE FROM photos WHERE sha256 LIKE 'SYNTH%'", []).unwrap();
    for i in 0..4 {
      let _ = std::fs::remove_file(dir.join(format!("synth_{i}.png")));
    }
    let _ = std::fs::remove_dir_all(dest);
  }

  #[test]
  fn duplicates_grouped_by_sha256() {
    let _db = db_lock();
    let dir = std::env::temp_dir().join(format!("openshoot_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok();
    if let Err(e) = super::catalog::init(dir.to_str().unwrap()) {
      eprintln!("init reutilizado: {e}");
    }
    let conn = super::catalog::open().expect("open catalog");
    conn.busy_timeout(std::time::Duration::from_secs(5)).ok();
    // Garante o schema completo (o OnceLock pode ter sido inicializado por
    // outro teste em paralelo antes das migrations terminarem).
    for col in [
      "ALTER TABLE photos ADD COLUMN cull_score REAL",
      "ALTER TABLE photos ADD COLUMN edit_json TEXT",
      "ALTER TABLE photos ADD COLUMN has_face INTEGER DEFAULT 0",
      "ALTER TABLE photos ADD COLUMN review INTEGER DEFAULT 0",
      "ALTER TABLE photos ADD COLUMN ai_pick INTEGER DEFAULT 0",
      "ALTER TABLE photos ADD COLUMN session_type TEXT DEFAULT ''",
      "ALTER TABLE photos ADD COLUMN label TEXT DEFAULT ''",
    ] {
      let _ = conn.execute(col, []);
    }
    // A mesma proteção vale para presets. Em execuções repetidas, o OnceLock
    // pode apontar para um banco temporário de uma versão anterior do schema.
    for col in [
      "ALTER TABLE presets ADD COLUMN file_type TEXT DEFAULT ''",
      "ALTER TABLE presets ADD COLUMN color_type TEXT DEFAULT ''",
      "ALTER TABLE presets ADD COLUMN source TEXT DEFAULT 'manual'",
    ] {
      let _ = conn.execute(col, []);
    }
    // Limpa resíduos de execuções anteriores (escopo deste teste — outros
    // testes rodam em paralelo no mesmo BD via OnceLock).
    conn
      .execute(
        "DELETE FROM photos WHERE file_name IN ('dup_a.jpg','dup_b.jpg','uniq.jpg')",
        [],
      )
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

    // ---- Aprender perfil (média de edições) ----
    super::catalog::set_photo_edit(id_a, r#"{"exposure":1.0,"contrast":20}"#).unwrap();
    // dup_b também com exposição (média de exposure deve ser ~1.0).
    let id_b: i64 = conn
      .query_row("SELECT id FROM photos WHERE file_name='dup_b.jpg'", [], |r| r.get(0))
      .unwrap();
    super::catalog::set_photo_edit(id_b, r#"{"exposure":1.0,"contrast":40}"#).unwrap();
    let (name, photos) = super::catalog::learn_profile().unwrap();
    assert!(name.contains("Perfil"));
    assert!(photos >= 2);
    let presets = super::catalog::list_presets().unwrap();
    let prof = presets.iter().find(|p| p.name == name).unwrap();
    let v: serde_json::Value = serde_json::from_str(&prof.recipe).unwrap();
    assert_eq!(v["exposure"], 1.0, "média de exposure deve ser 1.0");
    assert_eq!(v["contrast"], 30.0, "média de contrast deve ser 30");
    super::catalog::delete_preset(&name).unwrap();

    // ---- Tipo de sessão ----
    let updated = super::catalog::set_session_type_for_path("/tmp/", "casamento").unwrap();
    assert!(updated >= 2, "deve atualizar as fotos sob /tmp/");
    let list = super::catalog::list_photos("", "all", 0, 100).unwrap();
    // session_type não está no PhotoMeta; validamos indiretamente via contagem.
    assert!(list.total >= 2);

    // ---- Exportar/importar preset como arquivo (mercado) ----
    super::catalog::save_preset("Estilo", r#"{"exposure":0.5}"#).unwrap();
    let dest = std::env::temp_dir().join("openshoot_preset_export.json");
    super::catalog::export_preset_to_file("Estilo", &dest).unwrap();
    let name = super::catalog::import_preset_from_file(&dest).unwrap();
    assert_eq!(name, "openshoot_preset_export");
    super::catalog::delete_preset("Estilo").unwrap();
    super::catalog::delete_preset(&name).unwrap();
    let _ = std::fs::remove_file(&dest);

    // ---- Álbuns ----
    let album_id = super::catalog::create_album("Teste Album").unwrap();
    let ids = [id_a, id_b];
    let added = super::catalog::add_photos_to_album(album_id, &ids).unwrap();
    assert_eq!(added, 2, "deve associar 2 fotos");
    let albums = super::catalog::list_albums().unwrap();
    let a = albums.iter().find(|a| a.id == album_id).unwrap();
    assert_eq!(a.photo_count, 2, "contagem de fotos do álbum");
    let photo_ids = super::catalog::album_photo_ids(album_id).unwrap();
    assert_eq!(photo_ids.len(), 2);
    super::catalog::delete_album(album_id).unwrap();
    assert!(super::catalog::album_photo_ids(album_id).unwrap().is_empty());

    // ---- Tipo de foto no scan ----
    let tmp = std::env::temp_dir().join(format!("openshoot_scan_{}", std::process::id()));
    std::fs::create_dir_all(tmp.join("sub")).ok();
    std::fs::write(tmp.join("a.jpg"), "xx").ok();
    std::fs::write(tmp.join("sub/b.nef"), "yy").ok();
    // Nef é considerado foto? is_photo_path aceita .nef, mas inspect falha em
    // arquivo fake — testamos apenas o filtro de tipo via matches_photo_type.
    assert!(super::catalog::matches_photo_type(&tmp.join("a.jpg"), "jpeg"));
    assert!(!super::catalog::matches_photo_type(&tmp.join("a.jpg"), "raw"));
    assert!(super::catalog::matches_photo_type(&tmp.join("sub/b.nef"), "raw"));
    let _ = std::fs::remove_dir_all(&tmp);

    // Limpeza (escopo deste teste; NÃO remover o dir do catalog.db ativo
    // compartilhado via OnceLock — labels_tests o reutiliza).
    conn
      .execute(
        "DELETE FROM photos WHERE file_name IN ('dup_a.jpg','dup_b.jpg','uniq.jpg')",
        [],
      )
      .unwrap();
  }
}

// --- Labels de cor (agent-10) ---

/// Labels de cor válidos (padrão AfterShoot), armazenados em minúsculas.
const VALID_LABELS: [&str; 6] = ["", "red", "yellow", "green", "blue", "purple"];

/// Migração leve: garante a coluna `label` na tabela photos.
/// Executada pelo setup() logo após catalog::init (catalog.rs é intocado).
fn ensure_extra_columns() -> Result<()> {
  let conn = catalog::open().map_err(|e| Error::from_reason(e))?;
  let has_label: bool = conn
    .query_row(
      "SELECT COUNT(*) FROM pragma_table_info('photos') WHERE name='label'",
      [],
      |r| Ok(r.get::<_, i64>(0)? != 0),
    )
    .map_err(|e| Error::from_reason(e.to_string()))?;
  if !has_label {
    conn
      .execute("ALTER TABLE photos ADD COLUMN label TEXT DEFAULT ''", [])
      .map_err(|e| Error::from_reason(e.to_string()))?;
  }
  Ok(())
}

/// Define a etiqueta de cor manual de uma foto.
/// label: "" | "red" | "yellow" | "green" | "blue" | "purple".
#[napi]
pub fn set_photo_label(id: i64, label: String) -> Result<()> {
  let normalized = label.trim().to_lowercase();
  if !VALID_LABELS.contains(&normalized.as_str()) {
    return Err(Error::from_reason(format!(
      "label inválido: {label} (use '', red, yellow, green, blue ou purple)"
    )));
  }
  let conn = catalog::open().map_err(|e| Error::from_reason(e))?;
  conn
    .execute(
      "UPDATE photos SET label=?2 WHERE id=?1",
      rusqlite::params![id, normalized],
    )
    .map_err(|e| Error::from_reason(e.to_string()))?;
  Ok(())
}

/// Retorna a etiqueta de cor de uma foto ("" se não houver ou se não existir).
#[napi]
pub fn get_photo_label(id: i64) -> Result<String> {
  let conn = catalog::open().map_err(|e| Error::from_reason(e))?;
  match conn.query_row("SELECT label FROM photos WHERE id=?1", [id], |r| {
    r.get::<_, Option<String>>(0)
  }) {
    Ok(v) => Ok(v.unwrap_or_default()),
    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(String::new()),
    Err(e) => Err(Error::from_reason(e.to_string())),
  }
}

/// Retorna { "<id>": "<label>" } para as fotos informadas (lote, para a grade).
#[napi]
pub fn get_labels_bulk(ids: Vec<i64>) -> Result<serde_json::Value> {
  let mut out = serde_json::Map::new();
  if ids.is_empty() {
    return Ok(serde_json::Value::Object(out));
  }
  let conn = catalog::open().map_err(|e| Error::from_reason(e))?;
  // Fatia os ids para respeitar limites de variáveis do SQLite.
  for chunk in ids.chunks(500) {
    let placeholders: Vec<String> =
      chunk.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
    let sql = format!(
      "SELECT id, COALESCE(label,'') FROM photos WHERE id IN ({})",
      placeholders.join(",")
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| Error::from_reason(e.to_string()))?;
    let params: Vec<&dyn rusqlite::ToSql> =
      chunk.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
    let rows = stmt
      .query_map(params.as_slice(), |r| {
        let id: i64 = r.get(0)?;
        let label: String = r.get::<_, Option<String>>(1)?.unwrap_or_default();
        Ok((id, label))
      })
      .map_err(|e| Error::from_reason(e.to_string()))?;
    for row in rows {
      let (id, label) = row.map_err(|e| Error::from_reason(e.to_string()))?;
      out.insert(id.to_string(), serde_json::Value::String(label));
    }
  }
  Ok(serde_json::Value::Object(out))
}

#[cfg(test)]
mod labels_tests {
  use std::time::Duration;

  /// Uma tentativa do roundtrip. Janela crítica mínima: as linhas de teste só
  /// existem (visíveis a outras conexões) entre o INSERT e o DELETE final —
  /// qualquer erro dispara limpeza imediata. Retorna Err com o motivo em vez
  /// de panicar, para permitir retries caso o teste de catálogo (paralelo,
  /// mesma DB compartilhada via OnceLock) interferir.
  fn try_roundtrip() -> Result<(), String> {
    let dir = std::env::temp_dir().join(format!("openshoot_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok();
    if let Err(e) = super::catalog::init(dir.to_str().unwrap()) {
      // DB_PATH já inicializada pelo outro teste (mesmo arquivo) — segue.
      let _ = e;
    }
    let conn = super::catalog::open().map_err(|e| e)?;
    let _ = conn.busy_timeout(Duration::from_secs(10));
    // Aguarda o schema existir (fora da janela crítica; só leitura).
    let mut ready = false;
    for _ in 0..100 {
      let n: i64 = conn
        .query_row(
          "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='photos'",
          [],
          |r| r.get(0),
        )
        .unwrap_or(0);
      if n > 0 {
        ready = true;
        break;
      }
      std::thread::sleep(Duration::from_millis(20));
    }
    if !ready {
      return Err("tabela photos não ficou pronta".to_string());
    }
    // Garante a coluna label ANTES da janela crítica.
    super::ensure_extra_columns().map_err(|e| e.to_string())?;

    // Checagens sem tocar em linhas (fora da janela crítica).
    if super::get_photo_label(-1).map_err(|e| e.to_string())? != "" {
      return Err("foto inexistente deveria retornar ''".to_string());
    }
    if super::set_photo_label(-1, "mauve".to_string()).is_ok() {
      return Err("label inválido deveria falhar".to_string());
    }
    if !super::get_labels_bulk(vec![]).unwrap().as_object().unwrap().is_empty() {
      return Err("bulk vazio deveria ser {}".to_string());
    }

    // ---- Janela crítica: linhas visíveis por poucos microssegundos ----
    let cleanup = |conn: &rusqlite::Connection| {
      let _ = conn.execute("DELETE FROM photos WHERE file_name LIKE 'label_%'", []);
    };
    let run = || -> Result<(), String> {
      let conn = super::catalog::open().map_err(|e| e)?;
      let _ = conn.busy_timeout(Duration::from_secs(10));
      conn
        .execute("DELETE FROM photos WHERE file_name LIKE 'label_%'", [])
        .map_err(|e| e.to_string())?;
      let sql = "INSERT INTO photos (path, file_name, ext, file_size, sha256, indexed_at)
                 VALUES (?1, ?2, 'jpg', 10, ?3, datetime('now'))";
      conn
        .execute(sql, rusqlite::params!["/tmp/label_a.jpg", "label_a.jpg", "LABELA"])
        .map_err(|e| e.to_string())?;
      conn
        .execute(sql, rusqlite::params!["/tmp/label_b.jpg", "label_b.jpg", "LABELB"])
        .map_err(|e| e.to_string())?;
      let id_a: i64 = conn
        .query_row("SELECT id FROM photos WHERE file_name='label_a.jpg'", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
      let id_b: i64 = conn
        .query_row("SELECT id FROM photos WHERE file_name='label_b.jpg'", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;

      if super::get_photo_label(id_a).map_err(|e| e.to_string())? != "" {
        return Err("label default deveria ser vazio".to_string());
      }
      super::set_photo_label(id_a, "red".to_string()).map_err(|e| e.to_string())?;
      if super::get_photo_label(id_a).map_err(|e| e.to_string())? != "red" {
        return Err("get_photo_label não retornou 'red'".to_string());
      }
      let bulk = super::get_labels_bulk(vec![id_a, id_b]).map_err(|e| e.to_string())?;
      let m = bulk
        .as_object()
        .ok_or_else(|| "bulk não é objeto".to_string())?;
      if m.get(&id_a.to_string()).and_then(|v| v.as_str()) != Some("red") {
        return Err("bulk não retornou 'red' para id_a".to_string());
      }
      if m.get(&id_b.to_string()).and_then(|v| v.as_str()) != Some("") {
        return Err("bulk não retornou '' para id_b".to_string());
      }
      // Limpar ("" volta ao default).
      super::set_photo_label(id_a, "".to_string()).map_err(|e| e.to_string())?;
      if super::get_photo_label(id_a).map_err(|e| e.to_string())? != "" {
        return Err("limpar label falhou".to_string());
      }
      conn
        .execute("DELETE FROM photos WHERE file_name LIKE 'label_%'", [])
        .map_err(|e| e.to_string())?;
      Ok(())
    };
    match run() {
      Ok(()) => Ok(()),
      Err(e) => {
        cleanup(&conn);
        Err(e)
      }
    }
  }

  #[test]
  fn photo_label_set_get_roundtrip() {
    let _db = crate::tests::db_lock();
    let mut last = String::new();
    for attempt in 0..8 {
      match try_roundtrip() {
        Ok(()) => return,
        Err(e) => {
          last = e;
          // Pausa curtíssima: sem backoff longo (nada fica vazado).
          std::thread::sleep(Duration::from_millis(5 * (attempt + 1)));
        }
      }
    }
    panic!("roundtrip de labels falhou após 8 tentativas: {last}");
  }
}
