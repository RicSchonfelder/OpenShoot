#[macro_use]
extern crate napi_derive;

mod catalog;
mod imageproc;
mod types;

use napi::bindgen_prelude::*;
use std::path::{Path, PathBuf};

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
  tokio::task::spawn_blocking(move || match imageproc::thumbnail_base64(&path, dim) {
    Ok(b) => Some(b),
    Err(_) => None,
  })
  .await
  .map(|o| o)
  .map_err(|e| Error::from_reason(e.to_string()))
}

/// Gera thumbnail a partir de um caminho absoluto (independente do catálogo).
#[napi]
pub async fn thumb_for_path(path: String, max_dim: u32) -> Result<Option<String>> {
  let dim = if max_dim == 0 { 256 } else { max_dim };
  tokio::task::spawn_blocking(move || {
    imageproc::thumbnail_base64(Path::new(&path), dim).ok()
  })
  .await
  .map(|o| o)
  .map_err(|e| Error::from_reason(e.to_string()))
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
