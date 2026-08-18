#[macro_use]
extern crate napi_derive;

mod catalog;
mod culling;
mod imageproc;
mod types;
mod xmp;

use napi::bindgen_prelude::*;
use rayon::prelude::*;
use std::path::PathBuf;

use types::{PhotoList, PhotoMeta, ScanResult};

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

/// Lista fotos do catálogo com paginação + busca opcional.
#[napi]
pub fn list_photos(search: Option<String>, offset: i64, limit: i64) -> Result<PhotoList> {
  catalog::list_photos(&search.unwrap_or_default(), offset, limit)
    .map_err(|e| Error::from_reason(e))
}

/// Retorna metadados de uma foto pelo id.
#[napi]
pub fn get_photo(id: i64) -> Result<Option<PhotoMeta>> {
  catalog::get_photo(id).map_err(|e| Error::from_reason(e))
}

/// Total de fotos no catálogo.
#[napi]
pub fn photo_count() -> Result<i64> {
  catalog::count_photos().map_err(|e| Error::from_reason(e))
}

/// Gera um thumbnail JPEG (base64 data-uri) para uma foto do catálogo por id.
/// Async para não travar o event-loop; uses tokio + rayon-style thread pool via
/// napi-rs async. max_dim default 256.
#[napi]
pub async fn thumb_for_photo(id: i64, max_dim: u32) -> Result<Option<String>> {
  let photo = match catalog::get_photo(id) {
    Ok(Some(p)) => p,
    Ok(None) => return Ok(None),
    Err(e) => return Err(Error::from_reason(e)),
  };
  let path = PathBuf::from(&photo.path);
  // Run blocking decode off the async thread.
  let dim = if max_dim == 0 { 256 } else { max_dim };
  tokio::task::spawn_blocking(move || imageproc::thumbnail_base64(&path, dim).ok())
    .await
    .map_err(|e| Error::from_reason(e.to_string()))
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
}

#[napi]
pub async fn cull_photos() -> Result<CullSummary> {
  let paths = match catalog::all_photo_paths() {
    Ok(p) => p,
    Err(e) => return Err(Error::from_reason(e)),
  };
  let results: Vec<(i64, std::result::Result<f64, String>)> = paths
    .into_par_iter()
    .map(|p: catalog::PhotoPath| -> (i64, std::result::Result<f64, String>) {
      let score = culling::heuristic_score(&PathBuf::from(&p.path), 320);
      (p.id, score)
    })
    .collect();

  let mut processed = 0;
  let mut errors = 0;
  let mut sum = 0.0;
  let mut scores: Vec<(i64, f64)> = Vec::new();
  for (id, r) in results {
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
  for (i, (id, s)) in scores.iter().enumerate() {
    let q = if n <= 1 { 1.0 } else { i as f64 / (n - 1) as f64 };
    let rating = if q < 0.2 { 1 } else if q < 0.4 { 2 } else if q < 0.6 { 3 } else if q < 0.8 { 4 } else { 5 };
    if let Err(e) = catalog::set_photo_rating(*id, rating, *s) {
      crate::catalog::log_debug(&format!("falha ao salvar rating {}: {e}", id));
      errors += 1;
    }
  }
  let picks = scores.iter().filter(|(_, s)| *s >= 70.0).count() as i64;

  Ok(CullSummary {
    processed: processed as i64,
    errors,
    avg_score: if processed > 0 { sum / processed as f64 } else { 0.0 },
    picks: picks.max(0),
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
}
