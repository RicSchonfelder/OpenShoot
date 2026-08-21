use napi_derive::napi;

#[napi(object)]
#[derive(serde::Serialize)]
pub struct PhotoMeta {
  pub id: i64,
  pub path: String,
  pub file_name: String,
  pub ext: String,
  pub file_size: i64,
  pub width: i64,
  pub height: i64,
  pub camera: String,
  pub taken_at: Option<String>,
  pub rating: i64,
  pub has_xmp: bool,
  pub preview_available: bool,
  pub cull_score: Option<f64>,
  pub hash: String,
  pub has_face: bool,
  pub review: bool,
  pub ai_pick: bool,
}

#[napi(object)]
#[derive(serde::Serialize)]
pub struct ScanResult {
  pub scanned: i64,
  pub added: i64,
  pub updated: i64,
  pub skipped: i64,
  pub errors: Vec<String>,
}

#[napi(object)]
#[derive(serde::Serialize)]
pub struct PhotoList {
  pub photos: Vec<PhotoMeta>,
  pub total: i64,
}

#[napi(object)]
#[derive(serde::Serialize)]
pub struct DuplicateGroup {
  pub hash: String,
  pub photo_ids: Vec<i64>,
  pub photo_names: Vec<String>,
  pub photo_paths: Vec<String>,
}

#[napi(object)]
#[derive(serde::Serialize)]
pub struct FilterCounts {
  pub all: i64,
  pub picks: i64,
  pub rejects: i64,
  pub unrated: i64,
  pub review: i64,
  pub destaques: i64,
  pub selecionado: i64,
  pub duplicates: i64,
  pub faces: i64,
  pub edited: i64,
}

#[napi(object)]
#[derive(serde::Serialize)]
pub struct Preset {
  pub name: String,
  pub recipe: String,
}
