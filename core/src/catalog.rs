use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use rusqlite::{Connection, OptionalExtension};

use crate::types::{PhotoList, PhotoMeta, ScanResult};

static DB_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn init(data_dir: &str) -> Result<PathBuf, String> {
  let dir = PathBuf::from(data_dir);
  std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
  let db_path = dir.join("catalog.db");
  DB_PATH
    .set(db_path.clone())
    .map_err(|_| "DB_PATH ja inicializado".to_string())?;
  let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
  ensure_schema(&conn)?;
  Ok(db_path)
}

fn db_path() -> Result<&'static Path, String> {
  DB_PATH
    .get()
    .map(|p| p.as_path())
    .ok_or_else(|| "core nao inicializado: chame setup() primeiro".to_string())
}

pub fn open() -> Result<Connection, String> {
  Connection::open(db_path()?).map_err(|e| e.to_string())
}

fn ensure_schema(conn: &Connection) -> Result<(), String> {
  conn.execute_batch(
    "
    CREATE TABLE IF NOT EXISTS photos (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      path TEXT UNIQUE NOT NULL,
      file_name TEXT NOT NULL,
      ext TEXT NOT NULL,
      file_size INTEGER NOT NULL,
      sha256 TEXT NOT NULL,
      width INTEGER,
      height INTEGER,
      camera TEXT,
      taken_at TEXT,
      rating INTEGER DEFAULT 0,
      has_xmp INTEGER DEFAULT 0,
      preview_available INTEGER DEFAULT 0,
      indexed_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_photos_ext ON photos(ext);
    CREATE INDEX IF NOT EXISTS idx_photos_sha ON photos(sha256);
    CREATE INDEX IF NOT EXISTS idx_photos_taken ON photos(taken_at);
    ",
  )
  .map_err(|e| e.to_string())?;
  // Migração leve: garante colunas adicionadas em versões posteriores.
  let has_cull: bool = conn
    .query_row(
      "SELECT COUNT(*) FROM pragma_table_info('photos') WHERE name='cull_score'",
      [],
      |r| Ok(r.get::<_, i64>(0)? != 0),
    )
    .map_err(|e| e.to_string())?;
  if !has_cull {
    conn
      .execute("ALTER TABLE photos ADD COLUMN cull_score REAL", [])
      .map_err(|e| e.to_string())?;
  }
  let has_edit: bool = conn
    .query_row(
      "SELECT COUNT(*) FROM pragma_table_info('photos') WHERE name='edit_json'",
      [],
      |r| Ok(r.get::<_, i64>(0)? != 0),
    )
    .map_err(|e| e.to_string())?;
  if !has_edit {
    conn
      .execute("ALTER TABLE photos ADD COLUMN edit_json TEXT", [])
      .map_err(|e| e.to_string())?;
  }
  Ok(())
}

/// Persiste a receita de edição (JSON) de uma foto. Não-destrutiva.
pub fn set_photo_edit(id: i64, edit_json: &str) -> Result<(), String> {
  let conn = open()?;
  conn
    .execute(
      "UPDATE photos SET edit_json=?2 WHERE id=?1",
      rusqlite::params![id, edit_json],
    )
    .map_err(|e| e.to_string())?;
  Ok(())
}

/// Lê a receita de edição (JSON) de uma foto. Retorna "" se não houver.
pub fn get_photo_edit(id: i64) -> Result<String, String> {
  let conn = open()?;
  conn
    .query_row("SELECT edit_json FROM photos WHERE id=?1", [id], |r| {
      r.get::<_, Option<String>>(0)
    })
    .map(|o| o.unwrap_or_default())
    .map_err(|e| e.to_string())
}

pub fn upsert_photo(conn: &Connection, meta: &crate::imageproc::FileMeta) -> Result<bool, String> {
  let exists: bool = conn
    .query_row(
      "SELECT 1 FROM photos WHERE path = ?1",
      [&meta.path.to_string_lossy()],
      |_| Ok(true),
    )
    .optional()
    .map_err(|e| e.to_string())?
    .is_some();

  conn.execute(
    "INSERT INTO photos
       (path, file_name, ext, file_size, sha256, width, height, camera, taken_at, has_xmp, preview_available, indexed_at)
     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
     ON CONFLICT(path) DO UPDATE SET
       file_size=excluded.file_size,
       sha256=excluded.sha256,
       width=excluded.width,
       height=excluded.height,
       camera=excluded.camera,
       taken_at=excluded.taken_at,
       preview_available=excluded.preview_available,
       indexed_at=excluded.indexed_at",
    rusqlite::params![
      meta.path.to_string_lossy(),
      meta.path.file_name().map(|s| s.to_string_lossy()).unwrap_or_default(),
      meta.path.extension().map(|s| s.to_string_lossy()).unwrap_or_default().to_lowercase(),
      meta.file_size,
      meta.sha256,
      meta.width,
      meta.height,
      meta.camera,
      meta.taken_at,
      meta.has_xmp as i64,
      meta.preview_available as i64,
      meta.indexed_at,
    ],
  )
  .map_err(|e| e.to_string())?;
  Ok(!exists)
}

fn row_to_photo(row: &rusqlite::Row) -> rusqlite::Result<PhotoMeta> {
  Ok(PhotoMeta {
    id: row.get(0)?,
    path: row.get(1)?,
    file_name: row.get(2)?,
    ext: row.get(3)?,
    file_size: row.get(4)?,
    width: row.get(5).unwrap_or(0),
    height: row.get(6).unwrap_or(0),
    camera: row.get(7).unwrap_or_default(),
    taken_at: row.get(8)?,
    rating: row.get(9).unwrap_or(0),
    has_xmp: row.get::<_, i64>(10).unwrap_or(0) != 0,
    preview_available: row.get::<_, i64>(11).unwrap_or(0) != 0,
    cull_score: row.get(12).unwrap_or(None),
    hash: String::new(),
  })
}

/// Lista fotos com paginação, busca e filtro de rating.
/// filter: "all" | "picks" (>=4) | "rejects" (<=1, >0) | "unrated" (==0)
pub fn list_photos(
  search: &str,
  filter: &str,
  offset: i64,
  limit: i64,
) -> Result<PhotoList, String> {
  let conn = open()?;

  // Construir cláusula WHERE dinamicamente.
  let mut conds: Vec<String> = Vec::new();
  let mut params: Vec<rusqlite::types::Value> = Vec::new();
  if !search.trim().is_empty() {
    let like = format!("%{}%", search.trim());
    conds.push("(file_name LIKE ? || camera LIKE ?)".to_string());
    params.push(rusqlite::types::Value::Text(like.clone()));
    params.push(rusqlite::types::Value::Text(like));
  }
  match filter {
    "picks" => {
      conds.push("rating >= 4".to_string());
    }
    "rejects" => {
      conds.push("rating >= 1 AND rating <= 1".to_string());
    }
    "unrated" => {
      conds.push("rating = 0".to_string());
    }
    _ => {}
  }
  let where_clause = if conds.is_empty() {
    String::new()
  } else {
    format!("WHERE {}", conds.join(" AND "))
  };

  let total_sql = format!("SELECT COUNT(*) FROM photos {}", where_clause);
  let mut total_stmt = conn.prepare(&total_sql).map_err(|e| e.to_string())?;
  let total: i64 = {
    let mut rows = total_stmt
      .query(rusqlite::params_from_iter(params.iter()))
      .map_err(|e| e.to_string())?;
    match rows.next() {
      Ok(Some(row)) => row.get(0).map_err(|e| e.to_string())?,
      _ => 0,
    }
  };

  // SELECT com paginação. Parâmetros de busca/filtro vêm antes de LIMIT/OFFSET.
  let list_sql = format!(
    "SELECT id, path, file_name, ext, file_size, width, height, camera, taken_at, rating, has_xmp, preview_available, cull_score
     FROM photos {} ORDER BY rating DESC, cull_score DESC, id DESC LIMIT ? OFFSET ?",
    where_clause
  );
  let mut stmt = conn.prepare(&list_sql).map_err(|e| e.to_string())?;
  let mut all_params = params.clone();
  all_params.push(rusqlite::types::Value::Integer(limit));
  all_params.push(rusqlite::types::Value::Integer(offset));

  let mut photos: Vec<PhotoMeta> = Vec::new();
  let rows = stmt
    .query_map(rusqlite::params_from_iter(all_params.iter()), row_to_photo)
    .map_err(|e| e.to_string())?;
  for row in rows {
    photos.push(row.map_err(|e| e.to_string())?);
  }

  Ok(PhotoList { photos, total })
}

pub fn get_photo(id: i64) -> Result<Option<PhotoMeta>, String> {
  let conn = open()?;
  conn
    .query_row(
      "SELECT id, path, file_name, ext, file_size, width, height, camera, taken_at, rating, has_xmp, preview_available, cull_score
       FROM photos WHERE id=?1",
      [id],
      row_to_photo,
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub fn count_photos() -> Result<i64, String> {
  let conn = open()?;
  conn
    .query_row("SELECT COUNT(*) FROM photos", [], |r| r.get(0))
    .map_err(|e| e.to_string())
}

pub fn upsert_scan_photo(conn: &Connection, meta: &crate::imageproc::FileMeta) -> Result<bool, String> {
  upsert_photo(conn, meta)
}

pub struct PhotoPath {
  pub id: i64,
  pub path: String,
}

/// Todos os caminhos de fotos do catálogo (para culling em lote).
pub fn all_photo_paths() -> Result<Vec<PhotoPath>, String> {
  let conn = open()?;
  let mut stmt = conn
    .prepare("SELECT id, path FROM photos")
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map([], |r| {
      Ok(PhotoPath {
        id: r.get(0)?,
        path: r.get(1)?,
      })
    })
    .map_err(|e| e.to_string())?;
  let mut out = Vec::new();
  for r in rows {
    out.push(r.map_err(|e| e.to_string())?);
  }
  Ok(out)
}

/// Persiste o rating (0-5) e o score bruto de uma foto.
pub fn set_photo_rating(id: i64, rating: i64, cull_score: f64) -> Result<(), String> {
  let conn = open()?;
  conn.execute(
    "UPDATE photos SET rating=?2, cull_score=?3 WHERE id=?1",
    rusqlite::params![id, rating, cull_score],
  )
  .map_err(|e| e.to_string())?;
  Ok(())
}

pub fn scan_folder(dir: &str) -> Result<ScanResult, String> {
  let conn = open()?;
  let start = std::time::Instant::now();
  let mut result = ScanResult {
    scanned: 0,
    added: 0,
    updated: 0,
    skipped: 0,
    errors: Vec::new(),
  };

  for entry in walkdir::WalkDir::new(dir)
    .follow_links(false)
    .into_iter()
    .filter_entry(|e| !e.file_type().is_dir() || e.file_name() != ".gallery")
  {
    let entry = match entry {
      Ok(e) => e,
      Err(_) => {
        result.skipped += 1;
        continue;
      }
    };
    if !entry.file_type().is_file() {
      continue;
    }
    let path = entry.path();
    let is_photo = crate::imageproc::is_photo_path(path);
    if !is_photo {
      continue;
    }
    result.scanned += 1;

    match crate::imageproc::inspect_file(path) {
      Ok(meta) => {
        let is_new = upsert_scan_photo(&conn, &meta).map_err(|e| e.to_string())?;
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
  }

  log_debug(&format!(
    "scan_folder({}) done in {:?}: +{} up {} skip {} err {}",
    dir,
    start.elapsed(),
    result.added,
    result.updated,
    result.skipped,
    result.errors.len()
  ));
  Ok(result)
}

pub fn log_debug(msg: &str) {
  eprintln!("[openshoot-core] {msg}");
}
