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
  pub hash: String,
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
