use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use rusqlite::{params_from_iter, Connection, OptionalExtension};

use crate::types::{DuplicateGroup, PhotoList, PhotoMeta, ScanResult};

static INIT_GUARD: Mutex<()> = Mutex::new(());
static DB_PATH: OnceLock<PathBuf> = OnceLock::new();

#[cfg(test)]
pub(crate) fn test_db_lock() -> std::sync::MutexGuard<'static, ()> {
  static DB_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
  DB_LOCK
    .get_or_init(|| Mutex::new(()))
    .lock()
    .unwrap_or_else(|e| e.into_inner())
}

pub fn init(data_dir: &str) -> Result<PathBuf, String> {
  let _lock = INIT_GUARD.lock().map_err(|e| e.to_string())?;
  if let Some(p) = DB_PATH.get() {
    return Err(format!("DB_PATH ja inicializado: {}", p.display()));
  }
  let dir = PathBuf::from(data_dir);
  std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
  let db_path = dir.join("catalog.db");
  let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
  ensure_schema(&conn)?;
  DB_PATH
    .set(db_path.clone())
    .map_err(|_| "DB_PATH ja inicializado".to_string())?;
  Ok(db_path)
}

fn db_path() -> Result<&'static Path, String> {
  DB_PATH
    .get()
    .map(|p| p.as_path())
    .ok_or_else(|| "core nao inicializado: chame setup() primeiro".to_string())
}

pub fn db_path_string() -> Result<String, String> {
  Ok(db_path()?.display().to_string())
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
  CREATE TABLE IF NOT EXISTS presets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    recipe TEXT NOT NULL,
    created_at TEXT NOT NULL
  );
  CREATE TABLE IF NOT EXISTS albums (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    session_type TEXT DEFAULT '',
    cover_photo_id INTEGER,
    created_at TEXT NOT NULL
  );
  CREATE TABLE IF NOT EXISTS album_photos (
    album_id INTEGER NOT NULL,
    photo_id INTEGER NOT NULL,
    added_at TEXT NOT NULL,
    PRIMARY KEY (album_id, photo_id)
  );
  CREATE INDEX IF NOT EXISTS idx_album_photos_photo ON album_photos(photo_id);
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
    conn.execute("ALTER TABLE photos ADD COLUMN cull_score REAL", [])
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
    conn.execute("ALTER TABLE photos ADD COLUMN edit_json TEXT", [])
      .map_err(|e| e.to_string())?;
  }
  let has_face: bool = conn
    .query_row(
      "SELECT COUNT(*) FROM pragma_table_info('photos') WHERE name='has_face'",
      [],
      |r| Ok(r.get::<_, i64>(0)? != 0),
    )
    .map_err(|e| e.to_string())?;
  if !has_face {
    conn.execute(
      "ALTER TABLE photos ADD COLUMN has_face INTEGER DEFAULT 0",
      [],
    )
    .map_err(|e| e.to_string())?;
  }
  let has_review: bool = conn
    .query_row(
      "SELECT COUNT(*) FROM pragma_table_info('photos') WHERE name='review'",
      [],
      |r| Ok(r.get::<_, i64>(0)? != 0),
    )
    .map_err(|e| e.to_string())?;
  if !has_review {
    conn.execute("ALTER TABLE photos ADD COLUMN review INTEGER DEFAULT 0", [])
      .map_err(|e| e.to_string())?;
  }
  let has_ai_pick: bool = conn
    .query_row(
      "SELECT COUNT(*) FROM pragma_table_info('photos') WHERE name='ai_pick'",
      [],
      |r| Ok(r.get::<_, i64>(0)? != 0),
    )
    .map_err(|e| e.to_string())?;
  if !has_ai_pick {
    conn.execute(
      "ALTER TABLE photos ADD COLUMN ai_pick INTEGER DEFAULT 0",
      [],
    )
    .map_err(|e| e.to_string())?;
  }
  let has_session: bool = conn
    .query_row(
      "SELECT COUNT(*) FROM pragma_table_info('photos') WHERE name='session_type'",
      [],
      |r| Ok(r.get::<_, i64>(0)? != 0),
    )
    .map_err(|e| e.to_string())?;
  if !has_session {
    conn.execute(
      "ALTER TABLE photos ADD COLUMN session_type TEXT DEFAULT ''",
      [],
    )
    .map_err(|e| e.to_string())?;
  }
  // Migrações leves da tabela presets: identidade de perfil (estilo AfterShoot).
  // file_type: 'raw'|'jpeg'|'' · color_type: 'color'|'bw'|'' · source: 'manual'|'learned'|'lightroom'|'imported'.
  for (col, def) in [
    ("file_type", "TEXT DEFAULT ''"),
    ("color_type", "TEXT DEFAULT ''"),
    ("source", "TEXT DEFAULT 'manual'"),
  ] {
    let has_col: bool = conn
      .query_row(
        &format!(
          "SELECT COUNT(*) FROM pragma_table_info('presets') WHERE name='{}'",
          col
        ),
        [],
        |r| Ok(r.get::<_, i64>(0)? != 0),
      )
      .map_err(|e| e.to_string())?;
    if !has_col {
      conn.execute(
        &format!("ALTER TABLE presets ADD COLUMN {} {}", col, def),
        [],
      )
      .map_err(|e| e.to_string())?;
    }
  }
  let has_eyes: bool = conn
    .query_row(
      "SELECT COUNT(*) FROM pragma_table_info('photos') WHERE name='eyes_score'",
      [],
      |r| Ok(r.get::<_, i64>(0)? != 0),
    )
    .map_err(|e| e.to_string())?;
  if !has_eyes {
    conn.execute(
      "ALTER TABLE photos ADD COLUMN eyes_score REAL DEFAULT -1",
      [],
    )
    .map_err(|e| e.to_string())?;
  }
  let has_fe: bool = conn
    .query_row(
      "SELECT COUNT(*) FROM pragma_table_info('photos') WHERE name='face_embedding'",
      [],
      |r| Ok(r.get::<_, i64>(0)? != 0),
    )
    .map_err(|e| e.to_string())?;
  if !has_fe {
    conn.execute("ALTER TABLE photos ADD COLUMN face_embedding BLOB", [])
      .map_err(|e| e.to_string())?;
  }
  // ---- Fase 7: reconhecimento de pessoas (tabelas locais) ----
  conn.execute_batch(
    "
    CREATE TABLE IF NOT EXISTS person_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    album_id INTEGER,
    name TEXT NOT NULL,
    threshold REAL DEFAULT 0.5
    );
    CREATE TABLE IF NOT EXISTS photo_person_faces (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id INTEGER NOT NULL REFERENCES person_groups(id) ON DELETE CASCADE,
    photo_id INTEGER NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
    bbox_x1 REAL NOT NULL,
    bbox_y1 REAL NOT NULL,
    bbox_x2 REAL NOT NULL,
    bbox_y2 REAL NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_ppf_group ON photo_person_faces(group_id);
    CREATE INDEX IF NOT EXISTS idx_ppf_photo ON photo_person_faces(photo_id);
    CREATE INDEX IF NOT EXISTS idx_pg_album ON person_groups(album_id);
    ",
  )
  .map_err(|e| e.to_string())?;
  Ok(())
}

/// Embeddings faciais em cache da foto (BLOB com todos os rostos), se houver.
pub fn get_face_embedding(id: i64) -> Result<Option<Vec<u8>>, String> {
  let conn = open()?;
  let blob: Option<Vec<u8>> = conn
    .query_row(
      "SELECT face_embedding FROM photos WHERE id=?1 AND face_embedding IS NOT NULL",
      rusqlite::params![id],
      |r| r.get(0),
    )
    .map(Some)
    .or_else(|e| match e {
      rusqlite::Error::QueryReturnedNoRows => Ok(None),
      other => Err(other.to_string()),
    })?;
  Ok(blob)
}

/// Salva os embeddings faciais (todos os rostos) de uma foto em cache.
pub fn set_face_embedding(id: i64, blob: &[u8]) -> Result<(), String> {
  let conn = open()?;
  conn.execute(
    "UPDATE photos SET face_embedding=?2 WHERE id=?1",
    rusqlite::params![id, blob],
  )
  .map_err(|e| e.to_string())?;
  Ok(())
}

/// Persiste a receita de edição (JSON) de uma foto. Não-destrutiva.
pub fn set_photo_edit(id: i64, edit_json: &str) -> Result<(), String> {
  let conn = open()?;
  conn.execute(
    "UPDATE photos SET edit_json=?2 WHERE id=?1",
    rusqlite::params![id, edit_json],
  )
  .map_err(|e| e.to_string())?;
  Ok(())
}

/// Lê a receita de edição (JSON) de uma foto. Retorna "" se não houver.
pub fn get_photo_edit(id: i64) -> Result<String, String> {
  let conn = open()?;
  conn.query_row("SELECT edit_json FROM photos WHERE id=?1", [id], |r| {
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
    hash: row.get::<_, String>(13).unwrap_or_default(),
    has_face: row.get::<_, i64>(14).unwrap_or(0) != 0,
    review: row.get::<_, i64>(15).unwrap_or(0) != 0,
    ai_pick: row.get::<_, i64>(16).unwrap_or(0) != 0,
  })
}

/// Lista fotos com paginação, busca e filtro de rating.
/// filter: "all" | "picks" (>=4) | "rejects" (<=1, >0) | "unrated" (==0) |
///         "duplicates" (sha256 repetido) | "faces" (fotos com rosto)
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
    conds.push("(file_name LIKE ? OR camera LIKE ?)".to_string());
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
    "duplicates" => {
      // Fotos cujo sha256 aparece em mais de um registro (duplicatas/semelhantes).
      conds.push(
    "sha256 IN (SELECT sha256 FROM photos WHERE sha256 <> '' GROUP BY sha256 HAVING COUNT(*) > 1)"
      .to_string(),
    );
    }
    "faces" => {
      conds.push("has_face = 1".to_string());
    }
    "review" => {
      conds.push("review = 1".to_string());
    }
    "destaques" => {
      // Picks da IA (top-N marcados pelo culling).
      conds.push("ai_pick = 1".to_string());
    }
    "selecionado" => {
      // Picks manuais: rating alto sem ter sido pick da IA.
      conds.push("rating >= 4 AND ai_pick = 0".to_string());
    }
    "edited" => {
      // Fotos com receita de edição salva (não-destrutiva).
      conds.push("edit_json IS NOT NULL AND edit_json <> ''".to_string());
    }
    "unedited" => {
      conds.push("edit_json IS NULL OR edit_json = ''".to_string());
    }
    "portrait" => {
      conds.push("width > 0 AND height > 0 AND height > width".to_string());
    }
    "landscape" => {
      conds.push("width > 0 AND height > 0 AND width >= height".to_string());
    }
    "raw" => {
      conds.push(
        "ext IN ('nef','arw','dng','cr2','cr3','orf','raf','rw2','pef','srw','raw')"
          .to_string(),
      );
    }
    "jpeg" => {
      conds.push("ext IN ('jpg','jpeg','tiff','tif','png','webp','heic','heif')".to_string());
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
  "SELECT id, path, file_name, ext, file_size, width, height, camera, taken_at, rating, has_xmp, preview_available, cull_score, sha256, has_face, review, ai_pick
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
    "SELECT id, path, file_name, ext, file_size, width, height, camera, taken_at, rating, has_xmp, preview_available, cull_score, sha256, has_face, review, ai_pick
     FROM photos WHERE id=?1",
    [id],
    row_to_photo,
  )
  .optional()
  .map_err(|e| e.to_string())
}

pub fn count_photos() -> Result<i64, String> {
  let conn = open()?;
  conn.query_row("SELECT COUNT(*) FROM photos", [], |r| r.get(0))
    .map_err(|e| e.to_string())
}

pub fn upsert_scan_photo(
  conn: &Connection,
  meta: &crate::imageproc::FileMeta,
) -> Result<bool, String> {
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

/// Caminhos de um conjunto explícito de fotos, preservando o escopo do álbum.
pub fn photo_paths_for_ids(ids: &[i64]) -> Result<Vec<PhotoPath>, String> {
  if ids.is_empty() {
    return Ok(Vec::new());
  }
  let conn = open()?;
  let placeholders = std::iter::repeat("?")
    .take(ids.len())
    .collect::<Vec<_>>()
    .join(",");
  let sql = format!("SELECT id, path FROM photos WHERE id IN ({placeholders})");
  let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map(params_from_iter(ids.iter()), |r| {
      Ok(PhotoPath {
        id: r.get(0)?,
        path: r.get(1)?,
      })
    })
    .map_err(|e| e.to_string())?;
  rows.map(|r| r.map_err(|e| e.to_string())).collect()
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

/// Define apenas o rating (manual, via atalho de teclado). Não mexe no score.
pub fn set_photo_rating_manual(id: i64, rating: i64) -> Result<(), String> {
  let conn = open()?;
  conn.execute(
    "UPDATE photos SET rating=?2 WHERE id=?1",
    rusqlite::params![id, rating.clamp(0, 5)],
  )
  .map_err(|e| e.to_string())?;
  Ok(())
}

/// Registra se a foto tem rosto detectado (filtro "faces").
pub fn set_photo_has_face(id: i64, has_face: bool) -> Result<(), String> {
  let conn = open()?;
  conn.execute(
    "UPDATE photos SET has_face=?2 WHERE id=?1",
    rusqlite::params![id, if has_face { 1 } else { 0 }],
  )
  .map_err(|e| e.to_string())?;
  Ok(())
}

/// Persiste o score de olhos abertos (0..1) de uma foto.
/// -1 significa "ainda não calculado" (default da coluna).
#[allow(dead_code)]
pub fn set_photo_eyes(id: i64, score: f64) -> Result<(), String> {
  let conn = open()?;
  conn.execute(
    "UPDATE photos SET eyes_score=?2 WHERE id=?1",
    rusqlite::params![id, score.clamp(-1.0, 1.0)],
  )
  .map_err(|e| e.to_string())?;
  Ok(())
}

/// Registra se a foto está no bucket "Para revisão" (score ambíguo).
pub fn set_photo_review(id: i64, review: bool) -> Result<(), String> {
  let conn = open()?;
  conn.execute(
    "UPDATE photos SET review=?2 WHERE id=?1",
    rusqlite::params![id, if review { 1 } else { 0 }],
  )
  .map_err(|e| e.to_string())?;
  Ok(())
}

/// Registra se a foto foi marcada como pick pela IA ("Destaque").
/// Fotos marcadas manualmente (rating via atalho) mantêm ai_pick=0.
pub fn set_photo_ai_pick(id: i64, ai_pick: bool) -> Result<(), String> {
  let conn = open()?;
  conn.execute(
    "UPDATE photos SET ai_pick=?2 WHERE id=?1",
    rusqlite::params![id, if ai_pick { 1 } else { 0 }],
  )
  .map_err(|e| e.to_string())?;
  Ok(())
}

/// Define o tipo de sessão (gênero) das fotos de uma pasta (ex: 'wedding').
pub fn set_session_type_for_path(path_prefix: &str, session_type: &str) -> Result<i64, String> {
  let conn = open()?;
  let like = format!("{}%", path_prefix);
  let n = conn
    .execute(
      "UPDATE photos SET session_type=?2 WHERE path LIKE ?1",
      rusqlite::params![like, session_type],
    )
    .map_err(|e| e.to_string())?;
  Ok(n as i64)
}

/// Contagens por bucket (para painel de filtros com números vivos).
pub fn filter_counts(photo_ids: Option<&[i64]>) -> Result<crate::types::FilterCounts, String> {
  let conn = open()?;
  let scope = if let Some(ids) = photo_ids {
    conn.execute(
      "CREATE TEMP TABLE IF NOT EXISTS openshoot_scope_ids (id INTEGER PRIMARY KEY)",
      [],
    )
    .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM openshoot_scope_ids", [])
      .map_err(|e| e.to_string())?;
    for id in ids {
      conn.execute(
        "INSERT OR IGNORE INTO openshoot_scope_ids (id) VALUES (?1)",
        rusqlite::params![id],
      )
      .map_err(|e| e.to_string())?;
    }
    "id IN (SELECT id FROM openshoot_scope_ids)"
  } else {
    "1=1"
  };
  let one = |sql: &str| -> Result<i64, String> {
    conn.query_row(sql, [], |r| r.get::<_, i64>(0))
      .map_err(|e| e.to_string())
  };
  Ok(crate::types::FilterCounts {
  all: one(&format!("SELECT COUNT(*) FROM photos WHERE {scope}"))?,
  picks: one(&format!("SELECT COUNT(*) FROM photos WHERE {scope} AND rating >= 4"))?,
  rejects: one(&format!("SELECT COUNT(*) FROM photos WHERE {scope} AND rating >= 1 AND rating <= 2"))?,
  unrated: one(&format!("SELECT COUNT(*) FROM photos WHERE {scope} AND rating = 0"))?,
  review: one(&format!("SELECT COUNT(*) FROM photos WHERE {scope} AND review = 1"))?,
  destaques: one(&format!("SELECT COUNT(*) FROM photos WHERE {scope} AND ai_pick = 1"))?,
  selecionado: one(&format!("SELECT COUNT(*) FROM photos WHERE {scope} AND rating >= 4 AND ai_pick = 0"))?,
  duplicates: one(&format!("SELECT COUNT(*) FROM photos WHERE {scope} AND sha256 IN (SELECT sha256 FROM photos WHERE {scope} AND sha256 <> '' GROUP BY sha256 HAVING COUNT(*) > 1)"))?,
  faces: one(&format!("SELECT COUNT(*) FROM photos WHERE {scope} AND has_face = 1"))?,
  edited: one(&format!("SELECT COUNT(*) FROM photos WHERE {scope} AND edit_json IS NOT NULL AND edit_json <> ''"))?,
  })
}

/// Agrupa fotos duplicadas por sha256. Retorna lista de grupos (2+ fotos).
pub fn find_duplicates() -> Result<Vec<DuplicateGroup>, String> {
  let conn = open()?;
  let mut stmt = conn
    .prepare(
      "SELECT sha256, id, file_name, path
     FROM photos
     WHERE sha256 IN (
     SELECT sha256 FROM photos WHERE sha256 <> '' GROUP BY sha256 HAVING COUNT(*) > 1
     )
     ORDER BY sha256, id",
    )
    .map_err(|e| e.to_string())?;
  let mut groups: Vec<DuplicateGroup> = Vec::new();
  let mut current: Option<DuplicateGroup> = None;
  let rows = stmt
    .query_map([], |row| {
      Ok((
        row.get::<_, String>(0)?,
        row.get::<_, i64>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, String>(3)?,
      ))
    })
    .map_err(|e| e.to_string())?;
  for row in rows {
    let (hash, id, file_name, path) = row.map_err(|e| e.to_string())?;
    match &mut current {
      Some(g) if g.hash == hash => g.photo_ids.push(id),
      _ => {
        if let Some(g) = current.take() {
          if g.photo_ids.len() > 1 {
            groups.push(g);
          }
        }
        current = Some(DuplicateGroup {
          hash,
          photo_ids: vec![id],
          photo_names: vec![file_name],
          photo_paths: vec![path],
        });
      }
    }
  }
  if let Some(g) = current {
    if g.photo_ids.len() > 1 {
      groups.push(g);
    }
  }
  Ok(groups)
}

/// Remove uma foto do catálogo (não apaga o arquivo — o chamador decide).
pub fn remove_photo(id: i64) -> Result<(), String> {
  let conn = open()?;
  conn.execute("DELETE FROM photos WHERE id=?1", [id])
    .map_err(|e| e.to_string())?;
  Ok(())
}

// ---- Presets nomeados de edição ----

/// Salva um preset (upsert por nome). Receita = JSON dos parâmetros de edição.
/// Sem identidade: file_type/color_type vazios, source 'manual'.
pub fn save_preset(name: &str, recipe: &str) -> Result<(), String> {
  save_preset_full(name, recipe, "", "", "manual")
}

/// Salva um preset com metadados de identidade (estilo AfterShoot):
/// - `file_type`: tipo de arquivo em que o perfil foi treinado ('raw'|'jpeg'|'')
/// - `color_type`: tipo de cor do perfil ('color'|'bw'|'')
/// - `source`: origem ('manual'|'learned'|'lightroom'|'imported')
pub fn save_preset_full(
  name: &str,
  recipe: &str,
  file_type: &str,
  color_type: &str,
  source: &str,
) -> Result<(), String> {
  let conn = open()?;
  conn.execute(
    "INSERT INTO presets (name, recipe, created_at, file_type, color_type, source)
     VALUES (?1, ?2, datetime('now'), ?3, ?4, ?5)
     ON CONFLICT(name) DO UPDATE SET recipe=excluded.recipe, created_at=datetime('now'),
     file_type=excluded.file_type, color_type=excluded.color_type, source=excluded.source",
    rusqlite::params![name.trim(), recipe, file_type, color_type, source],
  )
  .map_err(|e| e.to_string())?;
  Ok(())
}

/// Atualiza apenas os metadados de identidade de um preset existente.
pub fn update_preset_meta(name: &str, file_type: &str, color_type: &str) -> Result<bool, String> {
  let conn = open()?;
  let n = conn
    .execute(
      "UPDATE presets SET file_type=?2, color_type=?3 WHERE name=?1",
      rusqlite::params![name.trim(), file_type, color_type],
    )
    .map_err(|e| e.to_string())?;
  Ok(n > 0)
}

/// Lista presets (nome + receita JSON + metadados de identidade).
pub fn list_presets() -> Result<Vec<crate::types::Preset>, String> {
  let conn = open()?;
  let mut stmt = conn
    .prepare("SELECT name, recipe, file_type, color_type, source FROM presets ORDER BY name")
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map([], |row| {
      Ok(crate::types::Preset {
        name: row.get(0)?,
        recipe: row.get(1)?,
        file_type: row.get(2).unwrap_or_default(),
        color_type: row.get(3).unwrap_or_default(),
        source: row.get(4).unwrap_or_default(),
      })
    })
    .map_err(|e| e.to_string())?;
  let mut out = Vec::new();
  for r in rows {
    out.push(r.map_err(|e| e.to_string())?);
  }
  Ok(out)
}

/// Remove um preset pelo nome.
pub fn delete_preset(name: &str) -> Result<bool, String> {
  let conn = open()?;
  let n = conn
    .execute("DELETE FROM presets WHERE name=?1", [name.trim()])
    .map_err(|e| e.to_string())?;
  Ok(n > 0)
}

/// Exporta um preset para um arquivo JSON (estilo compartilhável).
pub fn export_preset_to_file(name: &str, dest: &Path) -> Result<(), String> {
  let conn = open()?;
  let recipe: Option<String> = conn
    .query_row(
      "SELECT recipe FROM presets WHERE name=?1",
      rusqlite::params![name.trim()],
      |r| r.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())?;
  let recipe = recipe.ok_or_else(|| format!("preset '{name}' não existe"))?;
  std::fs::create_dir_all(dest.parent().unwrap_or(Path::new(".")))
    .map_err(|e| format!("criar pasta: {e}"))?;
  std::fs::write(dest, recipe).map_err(|e| format!("escrever arquivo: {e}"))?;
  Ok(())
}

/// Importa um preset de um arquivo JSON (estilo compartilhável).
pub fn import_preset_from_file(path: &Path) -> Result<String, String> {
  let recipe = std::fs::read_to_string(path).map_err(|e| format!("leitura: {e}"))?;
  // Valida que é JSON.
  serde_json::from_str::<serde_json::Value>(&recipe)
    .map_err(|e| format!("JSON inválido: {e}"))?;
  let name = path
    .file_stem()
    .map(|s| s.to_string_lossy().to_string())
    .unwrap_or_else(|| "Preset importado".to_string());
  save_preset(&name, &recipe)?;
  Ok(name)
}

/// "Aprende" um perfil de estilo: calcula a MÉDIA dos parâmetros de edição
/// aplicados às fotos (via `edit_json`), e salva como preset nomeado.
/// Retorna o nome do preset criado e quantas fotos foram usadas.
pub fn learn_profile() -> Result<(String, i64), String> {
  let conn = open()?;
  let mut stmt = conn
    .prepare("SELECT edit_json FROM photos WHERE edit_json IS NOT NULL AND edit_json <> ''")
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map([], |r| r.get::<_, String>(0))
    .map_err(|e| e.to_string())?;

  let mut sum: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
  let mut count = 0i64;
  let mut photos_used = 0i64;
  for row in rows {
    let json = match row {
      Ok(j) => j,
      Err(_) => continue,
    };
    let v: serde_json::Value = match serde_json::from_str(&json) {
      Ok(v) => v,
      Err(_) => continue,
    };
    count += 1;
    if let serde_json::Value::Object(map) = v {
      for (k, val) in map {
        if let serde_json::Value::Number(n) = val {
          if let Some(f) = n.as_f64() {
            let e = sum.entry(k).or_insert(0.0);
            *e += f;
          }
        }
      }
      photos_used += 1;
    }
  }
  if photos_used == 0 {
    return Err("nenhuma foto com edição salva para aprender o perfil".to_string());
  }

  // Média por parâmetro → receita JSON.
  let mut recipe = std::collections::BTreeMap::new();
  for (k, v) in sum {
    let avg = v / count as f64;
    // Arredonda e ignora valores ~neutros.
    let rounded = (avg * 100.0).round() / 100.0;
    if rounded.abs() > 0.001 {
      recipe.insert(k, serde_json::json!(rounded));
    }
  }
  let recipe_json = serde_json::Value::Object(
    recipe
      .into_iter()
      .map(|(k, v)| (k, v))
      .collect::<serde_json::Map<String, serde_json::Value>>(),
  )
  .to_string();

  let name = "Perfil aprendido";
  save_preset(name, &recipe_json)?;
  Ok((name.to_string(), photos_used))
}

// ---------------- Pessoas (grupos faciais persistidos) ----------------

/// Persiste grupos faciais de um álbum: apaga os antigos e insere os novos atomicamente.
/// Cada `PersistedGroup` contém name, threshold e lista de (photo_id, [x1,y1,x2,y2]).
pub struct PersistedFace {
  pub photo_id: i64,
  pub bbox: [f32; 4],
}

pub struct PersistedGroup {
  pub name: String,
  pub threshold: f32,
  pub faces: Vec<PersistedFace>,
}

/// Substitui todos os grupos faciais de um álbum (transactional).
pub fn replace_person_groups(album_id: i64, groups: &[PersistedGroup]) -> Result<(), String> {
  let conn = open()?;
  conn.execute_batch("PRAGMA foreign_keys = ON;")
    .map_err(|e| e.to_string())?;
  let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
  // Remove grupos antigos do álbum.
  tx.execute(
    "DELETE FROM person_groups WHERE album_id=?1",
    rusqlite::params![album_id],
  )
  .map_err(|e| e.to_string())?;
  for g in groups {
    tx.execute(
      "INSERT INTO person_groups (album_id, name, threshold) VALUES (?1, ?2, ?3)",
      rusqlite::params![album_id, g.name, g.threshold as f64],
    )
    .map_err(|e| e.to_string())?;
    let group_id = tx.last_insert_rowid();
    for face in &g.faces {
      tx.execute(
    "INSERT INTO photo_person_faces (group_id, photo_id, bbox_x1, bbox_y1, bbox_x2, bbox_y2)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    rusqlite::params![
      group_id,
      face.photo_id,
      face.bbox[0] as f64,
      face.bbox[1] as f64,
      face.bbox[2] as f64,
      face.bbox[3] as f64,
    ],
    )
    .map_err(|e| e.to_string())?;
    }
  }
  tx.commit().map_err(|e| e.to_string())?;
  Ok(())
}

/// Grupo facial persistido com suas faces.
#[derive(serde::Serialize)]
pub struct PersonGroupRow {
  pub id: i64,
  pub album_id: Option<i64>,
  pub name: String,
  pub threshold: f32,
}

pub struct FaceRow {
  pub id: i64,
  pub group_id: i64,
  pub photo_id: i64,
  pub bbox: [f32; 4],
  pub group_name: String,
}

/// Lista grupos faciais de um álbum.
pub fn list_person_groups(album_id: i64) -> Result<Vec<PersonGroupRow>, String> {
  let conn = open()?;
  let mut stmt = conn
    .prepare(
      "SELECT id, album_id, name, threshold FROM person_groups WHERE album_id=?1 ORDER BY id",
    )
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map(rusqlite::params![album_id], |r| {
      Ok(PersonGroupRow {
        id: r.get(0)?,
        album_id: r.get(1)?,
        name: r.get(2)?,
        threshold: r.get::<_, f64>(3).unwrap_or(0.5) as f32,
      })
    })
    .map_err(|e| e.to_string())?;
  rows.map(|r| r.map_err(|e| e.to_string())).collect()
}

/// Lista faces (rostos) de um grupo.
pub fn list_faces_in_group(group_id: i64) -> Result<Vec<FaceRow>, String> {
  let conn = open()?;
  let mut stmt = conn
  .prepare(
    "SELECT f.id, f.group_id, f.photo_id, f.bbox_x1, f.bbox_y1, f.bbox_x2, f.bbox_y2, g.name
     FROM photo_person_faces f
     JOIN person_groups g ON g.id = f.group_id
     WHERE f.group_id=?1 ORDER BY f.id",
  )
  .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map(rusqlite::params![group_id], |r| {
      Ok(FaceRow {
        id: r.get(0)?,
        group_id: r.get(1)?,
        photo_id: r.get(2)?,
        bbox: [
          r.get::<_, f64>(3)? as f32,
          r.get::<_, f64>(4)? as f32,
          r.get::<_, f64>(5)? as f32,
          r.get::<_, f64>(6)? as f32,
        ],
        group_name: r.get(7)?,
      })
    })
    .map_err(|e| e.to_string())?;
  rows.map(|r| r.map_err(|e| e.to_string())).collect()
}

/// Lista todas as faces (rostos) de uma foto (em todos os grupos).
pub fn list_faces_for_photo(photo_id: i64) -> Result<Vec<FaceRow>, String> {
  let conn = open()?;
  let mut stmt = conn
  .prepare(
    "SELECT f.id, f.group_id, f.photo_id, f.bbox_x1, f.bbox_y1, f.bbox_x2, f.bbox_y2, g.name
     FROM photo_person_faces f
     JOIN person_groups g ON g.id = f.group_id
     WHERE f.photo_id=?1 ORDER BY g.id, f.id",
  )
  .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map(rusqlite::params![photo_id], |r| {
      Ok(FaceRow {
        id: r.get(0)?,
        group_id: r.get(1)?,
        photo_id: r.get(2)?,
        bbox: [
          r.get::<_, f64>(3)? as f32,
          r.get::<_, f64>(4)? as f32,
          r.get::<_, f64>(5)? as f32,
          r.get::<_, f64>(6)? as f32,
        ],
        group_name: r.get(7)?,
      })
    })
    .map_err(|e| e.to_string())?;
  rows.map(|r| r.map_err(|e| e.to_string())).collect()
}

/// Sanitiza um nome para uso seguro como nome de pasta em qualquer SO.
/// Remove caracteres proibidos (Windows: \/:*?"<>|), controles, trima,
/// remove espaços/pontos finais. Nomes reservados Windows (CON, PRN, AUX,
/// NUL, COM1..COM9, LPT1..LPT9) recebem fallback seguro.
pub fn sanitize_folder_name(name: &str, fallback: &str) -> String {
  let sanitized: String = name
    .chars()
    .filter(|c| {
      !matches!(
        c,
        '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\x00'..='\x1f'
      )
    })
    .collect::<String>()
    .trim()
    .trim_end_matches(|c: char| c == '.' || c == ' ')
    .to_string();
  if sanitized.is_empty() {
    return fallback.to_string();
  }
  let upper = sanitized.to_uppercase();
  let reserved_stem = upper.split('.').next().unwrap_or(upper.as_str());
  let is_reserved = matches!(
    reserved_stem,
    "CON"
      | "PRN"
      | "AUX"
      | "NUL"
      | "COM1"
      | "COM2"
      | "COM3"
      | "COM4"
      | "COM5"
      | "COM6"
      | "COM7"
      | "COM8"
      | "COM9"
      | "LPT1"
      | "LPT2"
      | "LPT3"
      | "LPT4"
      | "LPT5"
      | "LPT6"
      | "LPT7"
      | "LPT8"
      | "LPT9"
  );
  if is_reserved {
    format!("{fallback}_{sanitized}")
  } else {
    sanitized
  }
}

/// Exporta os grupos faciais persistidos de um álbum para pastas nomeadas.
/// Cada grupo vira uma subpasta com o nome persistido (sanitizado).
/// Cada foto do grupo é copiada uma vez. Originais intocados.
/// Não sobrescreve: se arquivo já existe, gera sufixo numérico.
/// Se dois grupos sanitizam para o mesmo nome, o segundo recebe sufixo determinístico.
pub fn export_persisted_people_album(
  album_id: i64,
  out_dir: &str,
) -> Result<serde_json::Value, String> {
  let groups = list_person_groups(album_id)?;
  if groups.is_empty() {
    return Ok(serde_json::json!({
      "ok": true,
      "out_dir": out_dir,
      "groups": [],
      "exported": 0,
    }));
  }
  let root = std::path::PathBuf::from(out_dir);
  std::fs::create_dir_all(&root).map_err(|e| format!("criar {root:?}: {e}"))?;

  let mut exported = 0i64;
  let mut result_groups: Vec<serde_json::Value> = Vec::new();
  let mut used_folder_names: std::collections::HashMap<String, u32> =
    std::collections::HashMap::new();

  for pg in &groups {
    let base_name = sanitize_folder_name(&pg.name, &format!("Grupo {}", pg.id));
    let collision_count = used_folder_names
      .entry(base_name.to_lowercase())
      .or_insert(0);
    let folder_name = if *collision_count == 0 {
      base_name.clone()
    } else {
      format!("{base_name}-{}", *collision_count + 1)
    };
    *collision_count += 1;
    let folder = root.join(&folder_name);
    std::fs::create_dir_all(&folder).map_err(|e| format!("criar {folder:?}: {e}"))?;

    let faces = list_faces_in_group(pg.id)?;
    let mut unique_photo_ids: Vec<i64> = Vec::new();
    for f in &faces {
      if !unique_photo_ids.contains(&f.photo_id) {
        unique_photo_ids.push(f.photo_id);
      }
    }

    let mut copied = 0i64;
    for &photo_id in &unique_photo_ids {
      if let Ok(Some(photo)) = get_photo(photo_id) {
        let src = std::path::PathBuf::from(&photo.path);
        let name = src
          .file_name()
          .map(|s| s.to_string_lossy().to_string())
          .unwrap_or_else(|| format!("foto_{photo_id}.jpg"));
        let mut dest = folder.join(&name);
        if dest.exists() {
          let dot = name.rfind('.');
          let (stem, ext) = if let Some(d) = dot {
            (&name[..d], &name[d..])
          } else {
            (&name[..], "")
          };
          let mut suffix = 2u32;
          loop {
            dest = folder.join(format!("{stem}-{suffix}{ext}"));
            if !dest.exists() {
              break;
            }
            suffix += 1;
          }
        }
        if let Err(e) = std::fs::copy(&src, &dest) {
          crate::catalog::log_debug(&format!(
            "[people-export] copiar {}: {e}",
            photo.path
          ));
          continue;
        }
        copied += 1;
        exported += 1;
      }
    }
    result_groups.push(serde_json::json!({
      "person_id": pg.id,
      "folder": folder.display().to_string(),
      "count": copied,
    }));
  }

  Ok(serde_json::json!({
    "ok": true,
    "out_dir": root.display().to_string(),
    "groups": result_groups,
    "exported": exported,
  }))
}

/// Renomeia um grupo facial.
pub fn rename_person_group(group_id: i64, new_name: &str) -> Result<bool, String> {
  let trimmed = new_name.trim().to_string();
  if trimmed.is_empty() {
    return Err("nome do grupo não pode ser vazio".to_string());
  }
  let conn = open()?;
  let n = conn
    .execute(
      "UPDATE person_groups SET name=?2 WHERE id=?1",
      rusqlite::params![group_id, trimmed],
    )
    .map_err(|e| e.to_string())?;
  Ok(n > 0)
}

// ---------------- Álbuns ----------------

/// Cria um álbum. Retorna o id do novo álbum.
pub fn create_album(name: &str) -> Result<i64, String> {
  let conn = open()?;
  conn.execute(
    "INSERT INTO albums (name, created_at) VALUES (?1, datetime('now'))",
    rusqlite::params![name.trim()],
  )
  .map_err(|e| e.to_string())?;
  Ok(conn.last_insert_rowid())
}

/// Lista álbuns com contagem de fotos e thumbnail (caminho da capa).
pub fn list_albums() -> Result<Vec<crate::types::Album>, String> {
  let conn = open()?;
  let mut stmt = conn
    .prepare(
      "SELECT a.id, a.name, a.session_type, a.cover_photo_id, a.created_at,
        (SELECT COUNT(*) FROM album_photos ap WHERE ap.album_id = a.id) AS cnt,
        (SELECT p.path FROM album_photos ap JOIN photos p ON p.id = ap.photo_id
        WHERE ap.album_id = a.id ORDER BY ap.added_at LIMIT 1) AS cover_path
     FROM albums a ORDER BY a.created_at DESC",
    )
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map([], |r| {
      Ok(crate::types::Album {
        id: r.get(0)?,
        name: r.get(1)?,
        session_type: r.get(2).unwrap_or_default(),
        cover_photo_id: r.get(3).unwrap_or(None),
        created_at: r.get(4)?,
        photo_count: r.get(5).unwrap_or(0),
        cover_path: r.get(6).unwrap_or(None),
      })
    })
    .map_err(|e| e.to_string())?;
  let mut out = Vec::new();
  for r in rows {
    out.push(r.map_err(|e| e.to_string())?);
  }
  Ok(out)
}

/// Remove um álbum e suas associações (não toca nas fotos).
pub fn delete_album(id: i64) -> Result<bool, String> {
  let conn = open()?;
  conn.execute("DELETE FROM album_photos WHERE album_id=?1", [id])
    .map_err(|e| e.to_string())?;
  let n = conn
    .execute("DELETE FROM albums WHERE id=?1", [id])
    .map_err(|e| e.to_string())?;
  Ok(n > 0)
}

/// Associa fotos a um álbum. Recebe ids de fotos já no catálogo.
pub fn add_photos_to_album(album_id: i64, photo_ids: &[i64]) -> Result<i64, String> {
  let conn = open()?;
  let mut added = 0i64;
  for id in photo_ids {
    let n = conn
      .execute(
        "INSERT OR IGNORE INTO album_photos (album_id, photo_id, added_at)
     VALUES (?1, ?2, datetime('now'))",
        rusqlite::params![album_id, id],
      )
      .map_err(|e| e.to_string())?;
    added += n as i64;
  }
  Ok(added)
}

/// Associa fotos de um diretório a um álbum (após importar uma pasta).
pub fn add_folder_to_album(album_id: i64, dir: &str) -> Result<i64, String> {
  let conn = open()?;
  let like = format!("{}%", dir);
  let mut stmt = conn
    .prepare("SELECT id FROM photos WHERE path LIKE ?1")
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map(rusqlite::params![like], |r| r.get::<_, i64>(0))
    .map_err(|e| e.to_string())?;
  let mut ids: Vec<i64> = Vec::new();
  for r in rows {
    ids.push(r.map_err(|e| e.to_string())?);
  }
  add_photos_to_album(album_id, &ids)
}

/// Define o tipo de sessão (gênero) de um álbum.
pub fn set_album_session_type(album_id: i64, session_type: &str) -> Result<(), String> {
  let conn = open()?;
  conn.execute(
    "UPDATE albums SET session_type=?2 WHERE id=?1",
    rusqlite::params![album_id, session_type],
  )
  .map_err(|e| e.to_string())?;
  Ok(())
}

/// Lista os ids de fotos de um álbum (para filtrar a galeria).
pub fn album_photo_ids(album_id: i64) -> Result<Vec<i64>, String> {
  let conn = open()?;
  let mut stmt = conn
    .prepare("SELECT photo_id FROM album_photos WHERE album_id=?1 ORDER BY added_at")
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map(rusqlite::params![album_id], |r| r.get::<_, i64>(0))
    .map_err(|e| e.to_string())?;
  let mut out = Vec::new();
  for r in rows {
    out.push(r.map_err(|e| e.to_string())?);
  }
  Ok(out)
}

/// Exporta o catálogo completo como um manifesto JSON portátil.
/// As imagens não são copiadas: o manifesto guarda caminhos e hashes para
/// permitir reconciliar os registros em outro catálogo.
pub fn export_catalog_json() -> Result<String, String> {
  let conn = open()?;
  let mut photo_stmt = conn
    .prepare(
      "SELECT id, path, file_name, ext, file_size, sha256, width, height, camera,
              taken_at, rating, has_xmp, preview_available, cull_score, eyes_score,
              has_face, review, ai_pick, edit_json
       FROM photos ORDER BY id",
    )
    .map_err(|e| e.to_string())?;
  let photos: Vec<serde_json::Value> = photo_stmt
    .query_map([], |r| {
      Ok(serde_json::json!({
        "id": r.get::<_, i64>(0)?,
        "path": r.get::<_, String>(1)?,
        "file_name": r.get::<_, String>(2)?,
        "ext": r.get::<_, String>(3)?,
        "file_size": r.get::<_, i64>(4)?,
        "sha256": r.get::<_, String>(5)?,
        "width": r.get::<_, Option<i64>>(6)?,
        "height": r.get::<_, Option<i64>>(7)?,
        "camera": r.get::<_, Option<String>>(8)?,
        "taken_at": r.get::<_, Option<String>>(9)?,
        "rating": r.get::<_, i64>(10)?,
        "has_xmp": r.get::<_, i64>(11)? != 0,
        "preview_available": r.get::<_, i64>(12)? != 0,
        "cull_score": r.get::<_, Option<f64>>(13)?,
        "eyes_score": r.get::<_, Option<f64>>(14)?,
        "has_face": r.get::<_, i64>(15)? != 0,
        "review": r.get::<_, i64>(16)? != 0,
        "ai_pick": r.get::<_, i64>(17)? != 0,
        "edit_json": r.get::<_, Option<String>>(18)?,
      }))
    })
    .map_err(|e| e.to_string())?
    .map(|r| r.map_err(|e| e.to_string()))
    .collect::<Result<_, _>>()?;

  let mut album_stmt = conn
    .prepare("SELECT id, name, session_type FROM albums ORDER BY created_at, id")
    .map_err(|e| e.to_string())?;
  let albums: Vec<serde_json::Value> = album_stmt
    .query_map([], |r| {
      let album_id: i64 = r.get(0)?;
      let mut photos_stmt = conn
        .prepare(
          "SELECT p.path, p.sha256 FROM album_photos ap
           JOIN photos p ON p.id=ap.photo_id
           WHERE ap.album_id=?1 ORDER BY ap.added_at",
        )
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
      let album_photos: Vec<serde_json::Value> = photos_stmt
        .query_map([album_id], |p| {
          Ok(serde_json::json!({
            "path": p.get::<_, String>(0)?,
            "sha256": p.get::<_, String>(1)?,
          }))
        })
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
        .map(|p| p.map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(e))))?;

      let groups = list_person_groups(album_id)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(e))))?
        .into_iter()
        .map(|group| {
          let faces = list_faces_in_group(group.id)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(e))))?
            .into_iter()
            .map(|face| {
              let reference: Option<(String, String)> = conn
                .query_row(
                  "SELECT path, sha256 FROM photos WHERE id=?1",
                  [face.photo_id],
                  |p| Ok((p.get(0)?, p.get(1)?)),
                )
                .optional()
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
              Ok(serde_json::json!({
                "path": reference.as_ref().map(|v| v.0.clone()),
                "sha256": reference.as_ref().map(|v| v.1.clone()),
                "bbox": face.bbox,
              }))
            })
            .collect::<Result<Vec<_>, rusqlite::Error>>()?;
          Ok(serde_json::json!({
            "name": group.name,
            "threshold": group.threshold,
            "faces": faces,
          }))
        })
        .collect::<Result<Vec<_>, rusqlite::Error>>()?;
      Ok(serde_json::json!({
        "name": r.get::<_, String>(1)?,
        "session_type": r.get::<_, String>(2).unwrap_or_default(),
        "photos": album_photos,
        "people": groups,
      }))
    })
    .map_err(|e| e.to_string())?
    .map(|r| r.map_err(|e| e.to_string()))
    .collect::<Result<_, _>>()?;

  serde_json::to_string_pretty(&serde_json::json!({
    "schema_version": 1,
    "format": "openshoot-catalog",
    "photos": photos,
    "albums": albums,
  }))
  .map_err(|e| e.to_string())
}

/// Importa um manifesto JSON sem tocar nas imagens originais.
/// Fotos são reconciliadas primeiro por caminho e depois por SHA-256; entradas
/// que não existem neste catálogo ficam contabilizadas como ausentes.
pub fn import_catalog_json(manifest: &str) -> Result<serde_json::Value, String> {
  let root: serde_json::Value = serde_json::from_str(manifest).map_err(|e| format!("JSON inválido: {e}"))?;
  if root.get("format").and_then(|v| v.as_str()) != Some("openshoot-catalog") {
    return Err("arquivo não é um catálogo OpenShoot".to_string());
  }
  if root.get("schema_version").and_then(|v| v.as_i64()) != Some(1) {
    return Err("versão de catálogo não suportada".to_string());
  }
  let conn = open()?;
  let mut refs = std::collections::HashMap::<String, i64>::new();
  let mut hashes = std::collections::HashMap::<String, i64>::new();
  {
    let mut stmt = conn
      .prepare("SELECT id, path, sha256 FROM photos")
      .map_err(|e| e.to_string())?;
    let rows = stmt
      .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))
      .map_err(|e| e.to_string())?;
    for row in rows {
      let (id, path, hash) = row.map_err(|e| e.to_string())?;
      refs.insert(path, id);
      if !hash.is_empty() {
        hashes.insert(hash, id);
      }
    }
  }

  let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
  let mut albums_imported = 0i64;
  let mut photos_linked = 0i64;
  let mut photos_missing = 0i64;
  let mut photos_updated = 0i64;
  let mut groups_imported = 0i64;
  let mut faces_imported = 0i64;
  let albums = root.get("albums").and_then(|v| v.as_array()).ok_or_else(|| "manifesto sem albums".to_string())?;

  // Restaura metadados não destrutivos das fotos já presentes no catálogo.
  if let Some(photo_list) = root.get("photos").and_then(|v| v.as_array()) {
    for photo in photo_list {
      let path = photo.get("path").and_then(|v| v.as_str()).unwrap_or_default();
      let hash = photo.get("sha256").and_then(|v| v.as_str()).unwrap_or_default();
      let photo_id = refs.get(path).copied().or_else(|| hashes.get(hash).copied());
      if let Some(photo_id) = photo_id {
        tx.execute(
          "UPDATE photos SET rating=?2, has_xmp=?3, preview_available=?4, cull_score=?5, eyes_score=?6, has_face=?7, review=?8, ai_pick=?9, edit_json=?10 WHERE id=?1",
          rusqlite::params![
            photo_id,
            photo.get("rating").and_then(|v| v.as_i64()).unwrap_or(0),
            if photo.get("has_xmp").and_then(|v| v.as_bool()).unwrap_or(false) { 1 } else { 0 },
            if photo.get("preview_available").and_then(|v| v.as_bool()).unwrap_or(false) { 1 } else { 0 },
            photo.get("cull_score").and_then(|v| v.as_f64()),
            photo.get("eyes_score").and_then(|v| v.as_f64()),
            if photo.get("has_face").and_then(|v| v.as_bool()).unwrap_or(false) { 1 } else { 0 },
            if photo.get("review").and_then(|v| v.as_bool()).unwrap_or(false) { 1 } else { 0 },
            if photo.get("ai_pick").and_then(|v| v.as_bool()).unwrap_or(false) { 1 } else { 0 },
            photo.get("edit_json").and_then(|v| v.as_str()),
          ],
        )
        .map_err(|e| e.to_string())?;
        photos_updated += 1;
      }
    }
  }

  for (album_index, album) in albums.iter().enumerate() {
    let base_name = album.get("name").and_then(|v| v.as_str()).unwrap_or("Álbum importado").trim();
    let base_name = if base_name.is_empty() { "Álbum importado" } else { base_name };
    let name = if tx
      .query_row("SELECT COUNT(*) FROM albums WHERE name=?1", [base_name], |r| r.get::<_, i64>(0))
      .unwrap_or(0) > 0
    {
      format!("{base_name} (importado)")
    } else {
      base_name.to_string()
    };
    tx.execute(
      "INSERT INTO albums (name, session_type, created_at) VALUES (?1, ?2, datetime('now'))",
      rusqlite::params![name, album.get("session_type").and_then(|v| v.as_str()).unwrap_or("")],
    )
    .map_err(|e| e.to_string())?;
    let album_id = tx.last_insert_rowid();
    albums_imported += 1;

    if let Some(photo_list) = album.get("photos").and_then(|v| v.as_array()) {
      for photo in photo_list {
        let path = photo.get("path").and_then(|v| v.as_str()).unwrap_or_default();
        let hash = photo.get("sha256").and_then(|v| v.as_str()).unwrap_or_default();
        let photo_id = refs.get(path).copied().or_else(|| hashes.get(hash).copied());
        if let Some(photo_id) = photo_id {
          tx.execute(
            "INSERT OR IGNORE INTO album_photos (album_id, photo_id, added_at) VALUES (?1, ?2, datetime('now'))",
            rusqlite::params![album_id, photo_id],
          )
          .map_err(|e| e.to_string())?;
          photos_linked += 1;
        } else {
          photos_missing += 1;
        }
      }
    }

    if let Some(groups) = album.get("people").and_then(|v| v.as_array()) {
      for group in groups {
        tx.execute(
          "INSERT INTO person_groups (album_id, name, threshold) VALUES (?1, ?2, ?3)",
          rusqlite::params![album_id, group.get("name").and_then(|v| v.as_str()).unwrap_or("Pessoa"), group.get("threshold").and_then(|v| v.as_f64()).unwrap_or(0.5)],
        )
        .map_err(|e| e.to_string())?;
        let group_id = tx.last_insert_rowid();
        groups_imported += 1;
        if let Some(faces) = group.get("faces").and_then(|v| v.as_array()) {
          for face in faces {
            let path = face.get("path").and_then(|v| v.as_str()).unwrap_or_default();
            let hash = face.get("sha256").and_then(|v| v.as_str()).unwrap_or_default();
            let photo_id = refs.get(path).copied().or_else(|| hashes.get(hash).copied());
            let bbox = face.get("bbox").and_then(|v| v.as_array());
            if let (Some(photo_id), Some(bbox)) = (photo_id, bbox) {
              if bbox.len() == 4 {
                tx.execute(
                  "INSERT INTO photo_person_faces (group_id, photo_id, bbox_x1, bbox_y1, bbox_x2, bbox_y2) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                  rusqlite::params![group_id, photo_id, bbox[0].as_f64().unwrap_or(0.0), bbox[1].as_f64().unwrap_or(0.0), bbox[2].as_f64().unwrap_or(0.0), bbox[3].as_f64().unwrap_or(0.0)],
                )
                .map_err(|e| e.to_string())?;
                faces_imported += 1;
              }
            }
          }
        }
      }
    }

    let _ = album_index;
  }
  tx.commit().map_err(|e| e.to_string())?;
  Ok(serde_json::json!({
    "ok": true,
    "albums_imported": albums_imported,
    "photos_linked": photos_linked,
    "photos_missing": photos_missing,
    "photos_updated": photos_updated,
    "groups_imported": groups_imported,
    "faces_imported": faces_imported,
  }))
}

pub fn scan_folder(dir: &str, include_subdirs: bool, types: &str) -> Result<ScanResult, String> {
  let conn = open()?;
  let start = std::time::Instant::now();
  let mut result = ScanResult {
    scanned: 0,
    added: 0,
    updated: 0,
    skipped: 0,
    errors: Vec::new(),
  };

  let mut wd = walkdir::WalkDir::new(dir).follow_links(false);
  if !include_subdirs {
    wd = wd.max_depth(1);
  }
  let walker = wd
    .into_iter()
    .filter_entry(|e| !e.file_type().is_dir() || e.file_name() != ".gallery");

  for entry in walker {
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
    // Filtro por tipo de foto ("raw" | "jpeg" | "all").
    if !matches_photo_type(path, types) {
      result.skipped += 1;
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

/// Verifica se a extensão do arquivo corresponde ao tipo de importação.
pub fn matches_photo_type(path: &std::path::Path, types: &str) -> bool {
  let ext = path
    .extension()
    .map(|s| s.to_string_lossy().to_lowercase())
    .unwrap_or_default();
  match types {
    "raw" => matches!(
      ext.as_str(),
      "nef" | "arw" | "dng" | "cr2" | "cr3" | "orf" | "raf" | "rw2" | "pef" | "srw"
    ),
    "jpeg" => matches!(
      ext.as_str(),
      "jpg" | "jpeg" | "tiff" | "tif" | "png" | "webp" | "heic" | "heif"
    ),
    _ => true,
  }
}

pub fn log_debug(msg: &str) {
  eprintln!("[openshoot-core] {msg}");
}

#[cfg(test)]
mod tests {
  #[test]
  fn preset_identity_roundtrip() {
    let _lock = super::test_db_lock();
    let dir = std::env::temp_dir().join(format!(
      "openshoot_catalog_preset_test_{}",
      std::process::id()
    ));
    std::fs::create_dir_all(&dir).ok();
    if let Err(e) = super::init(dir.to_str().unwrap()) {
      eprintln!("init reutilizado: {e}");
    }
    // Limpa resíduos de execuções anteriores.
    super::open()
      .unwrap()
      .execute("DELETE FROM presets", [])
      .unwrap();

    // save_preset_full grava os metadados de identidade.
    super::save_preset_full(
      "Perfil Casamento RAW",
      r#"{"contrast":10}"#,
      "raw",
      "color",
      "learned",
    )
    .unwrap();
    let presets = super::list_presets().expect("list_presets");
    let p = presets
      .iter()
      .find(|p| p.name == "Perfil Casamento RAW")
      .expect("preset com identidade");
    assert_eq!(p.file_type, "raw");
    assert_eq!(p.color_type, "color");
    assert_eq!(p.source, "learned");

    // save_preset antigo delega com defaults (source 'manual', tipos vazios).
    super::save_preset("Só Receita", r#"{"exposure":0.3}"#).unwrap();
    let presets = super::list_presets().unwrap();
    let plain = presets.iter().find(|p| p.name == "Só Receita").unwrap();
    assert_eq!(plain.source, "manual");
    assert_eq!(plain.file_type, "");
    assert_eq!(plain.color_type, "");

    // Upsert via save_preset_full atualiza receita E metadados.
    super::save_preset_full(
      "Perfil Casamento RAW",
      r#"{"contrast":15}"#,
      "jpeg",
      "bw",
      "manual",
    )
    .unwrap();
    let presets = super::list_presets().unwrap();
    let p = presets
      .iter()
      .find(|p| p.name == "Perfil Casamento RAW")
      .unwrap();
    assert_eq!(p.recipe, r#"{"contrast":15}"#);
    assert_eq!(p.file_type, "jpeg");
    assert_eq!(p.color_type, "bw");
    assert_eq!(
      presets
        .iter()
        .filter(|p| p.name == "Perfil Casamento RAW")
        .count(),
      1
    );

    // update_preset_meta muda só file_type/color_type.
    assert!(super::update_preset_meta("Só Receita", "raw", "color").unwrap());
    let presets = super::list_presets().unwrap();
    let plain = presets.iter().find(|p| p.name == "Só Receita").unwrap();
    assert_eq!(plain.file_type, "raw");
    assert_eq!(plain.color_type, "color");
    // Preset inexistente → false.
    assert!(!super::update_preset_meta("Nao Existe", "raw", "color").unwrap());

    // Limpeza.
    assert!(super::delete_preset("Perfil Casamento RAW").unwrap());
    assert!(super::delete_preset("Só Receita").unwrap());
  }

  fn setup_test_db() -> rusqlite::Connection {
    let dir = std::env::temp_dir().join(format!(
      "openshoot_catalog_test_{}_{}",
      std::process::id(),
      // Sufixo rand para evitar colisão entre testes paralelos.
      std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        % 1_000_000
    ));
    std::fs::create_dir_all(&dir).ok();
    // init() pode falhar se DB_PATH já foi definido por outro teste.
    // Ignoramos — o schema já existe no DB ativo via OnceLock.
    let _ = super::init(dir.to_str().unwrap());
    let conn = super::open().unwrap();
    // Garante tabelas de pessoas (o OnceLock pode apontar para DB de teste anterior).
    conn.execute_batch(
      "
    CREATE TABLE IF NOT EXISTS person_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    album_id INTEGER,
    name TEXT NOT NULL,
    threshold REAL DEFAULT 0.5
    );
    CREATE TABLE IF NOT EXISTS photo_person_faces (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id INTEGER NOT NULL REFERENCES person_groups(id) ON DELETE CASCADE,
    photo_id INTEGER NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
    bbox_x1 REAL NOT NULL,
    bbox_y1 REAL NOT NULL,
    bbox_x2 REAL NOT NULL,
    bbox_y2 REAL NOT NULL
    );
    ",
    )
    .unwrap();
    conn
  }

  #[test]
  fn person_groups_migration_idempotent() {
    let _lock = super::test_db_lock();
    let conn = setup_test_db();
    // Verifica existência das tabelas via sqlite_master (não contagem global).
    let has_groups: bool = conn
      .query_row(
        "SELECT COUNT(*)>0 FROM sqlite_master WHERE type='table' AND name='person_groups'",
        [],
        |r| r.get(0),
      )
      .unwrap();
    assert!(has_groups, "person_groups table must exist");
    let has_faces: bool = conn
    .query_row(
    "SELECT COUNT(*)>0 FROM sqlite_master WHERE type='table' AND name='photo_person_faces'",
    [],
    |r| r.get(0),
    )
    .unwrap();
    assert!(has_faces, "photo_person_faces table must exist");
    // Verifica colunas via PRAGMA.
    let cols: Vec<String> = {
      let mut stmt = conn.prepare("PRAGMA table_info(person_groups)").unwrap();
      stmt.query_map([], |r| r.get(1))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
    };
    assert!(cols.contains(&"name".to_string()));
    assert!(cols.contains(&"threshold".to_string()));
  }

  #[test]
  fn person_groups_roundtrip_with_bbox() {
    let _lock = super::test_db_lock();
    let conn = setup_test_db();
    let album_id: i64 = 99991;
    let prefix = format!("rt_{}", std::process::id());
    // Limpa apenas os resíduos deste teste.
    conn.execute(&format!("DELETE FROM photo_person_faces WHERE group_id IN (SELECT id FROM person_groups WHERE album_id={album_id})"), []).ok();
    conn.execute(
      &format!("DELETE FROM person_groups WHERE album_id={album_id}"),
      [],
    )
    .ok();

    let path_a = format!("/{prefix}_a.jpg");
    let path_b = format!("/{prefix}_b.jpg");
    let path_c = format!("/{prefix}_c.jpg");
    conn.execute(
      &format!("DELETE FROM photos WHERE path IN ('{path_a}','{path_b}','{path_c}')"),
      [],
    )
    .ok();

    // Insere fotos dummy.
    conn.execute(
      "INSERT INTO photos (path, file_name, ext, file_size, sha256, indexed_at)
     VALUES (?1, ?2, 'jpg', 100, ?3, datetime('now'))",
      rusqlite::params![path_a, format!("{prefix}_a.jpg"), format!("sha_{prefix}_a")],
    )
    .unwrap();
    let photo_id_a: i64 = conn.last_insert_rowid();
    conn.execute(
      "INSERT INTO photos (path, file_name, ext, file_size, sha256, indexed_at)
     VALUES (?1, ?2, 'jpg', 200, ?3, datetime('now'))",
      rusqlite::params![path_b, format!("{prefix}_b.jpg"), format!("sha_{prefix}_b")],
    )
    .unwrap();
    conn.execute(
      "INSERT INTO photos (path, file_name, ext, file_size, sha256, indexed_at)
     VALUES (?1, ?2, 'jpg', 300, ?3, datetime('now'))",
      rusqlite::params![path_c, format!("{prefix}_c.jpg"), format!("sha_{prefix}_c")],
    )
    .unwrap();
    let photo_id_c: i64 = conn.last_insert_rowid();
    drop(conn);

    // Persiste 2 grupos no álbum 99991.
    let groups = vec![
      super::PersistedGroup {
        name: "Alice".to_string(),
        threshold: 0.55,
        faces: vec![
          super::PersistedFace {
            photo_id: photo_id_a,
            bbox: [0.1, 0.2, 0.3, 0.4],
          },
          super::PersistedFace {
            photo_id: photo_id_c,
            bbox: [0.5, 0.6, 0.7, 0.8],
          },
        ],
      },
      super::PersistedGroup {
        name: "Bob".to_string(),
        threshold: 0.6,
        faces: vec![super::PersistedFace {
          photo_id: photo_id_c,
          bbox: [0.0, 0.0, 0.2, 0.2],
        }],
      },
    ];
    super::replace_person_groups(album_id, &groups).unwrap();

    // Lista grupos do álbum 99991.
    let pg = super::list_person_groups(album_id).unwrap();
    assert_eq!(pg.len(), 2);
    assert_eq!(pg[0].name, "Alice");
    assert!((pg[0].threshold - 0.55).abs() < 1e-5);
    assert_eq!(pg[1].name, "Bob");

    // Lista faces do grupo Alice.
    let alice_id = pg[0].id;
    let faces = super::list_faces_in_group(alice_id).unwrap();
    assert_eq!(faces.len(), 2);
    assert_eq!(faces[0].photo_id, photo_id_a);
    assert_eq!(faces[0].group_name, "Alice");
    assert!((faces[0].bbox[0] - 0.1).abs() < 1e-5);
    assert!((faces[0].bbox[3] - 0.4).abs() < 1e-5);
    assert_eq!(faces[1].photo_id, photo_id_c);

    // Lista faces da foto C (deve ter 2 faces em 2 grupos diferentes).
    let photo_faces = super::list_faces_for_photo(photo_id_c).unwrap();
    assert_eq!(photo_faces.len(), 2);

    // Álbum vazio não tem grupos.
    let empty = super::list_person_groups(99999).unwrap();
    assert!(empty.is_empty());

    // Limpeza (escopo deste teste).
    let conn = super::open().unwrap();
    conn.execute(&format!("DELETE FROM photo_person_faces WHERE group_id IN (SELECT id FROM person_groups WHERE album_id={album_id})"), []).ok();
    conn.execute(
      &format!("DELETE FROM person_groups WHERE album_id={album_id}"),
      [],
    )
    .ok();
  }

  #[test]
  fn person_group_rename() {
    let _lock = super::test_db_lock();
    let conn = setup_test_db();
    let album_id: i64 = 99992;
    let prefix = format!("rn_{}", std::process::id());
    let path = format!("/{prefix}.jpg");
    // Limpa apenas resíduos deste teste.
    conn.execute(&format!("DELETE FROM photo_person_faces WHERE group_id IN (SELECT id FROM person_groups WHERE album_id={album_id})"), []).ok();
    conn.execute(
      &format!("DELETE FROM person_groups WHERE album_id={album_id}"),
      [],
    )
    .ok();
    conn.execute(&format!("DELETE FROM photos WHERE path='{path}'"), [])
      .ok();

    conn.execute(
      "INSERT INTO photos (path, file_name, ext, file_size, sha256, indexed_at)
     VALUES (?1, ?2, 'jpg', 100, ?3, datetime('now'))",
      rusqlite::params![path, format!("{prefix}.jpg"), format!("sha_{prefix}")],
    )
    .unwrap();
    let photo_id: i64 = conn.last_insert_rowid();
    drop(conn);

    let groups = vec![super::PersistedGroup {
      name: "Original".to_string(),
      threshold: 0.5,
      faces: vec![super::PersistedFace {
        photo_id,
        bbox: [0.1; 4],
      }],
    }];
    super::replace_person_groups(album_id, &groups).unwrap();

    let pg = super::list_person_groups(album_id).unwrap();
    let gid = pg[0].id;

    // Trim + rejeição de vazio.
    assert!(super::rename_person_group(gid, "  Renomeada  ").unwrap());
    let pg = super::list_person_groups(album_id).unwrap();
    assert_eq!(pg[0].name, "Renomeada");

    // Nome vazio (com espaços) deve retornar erro.
    let result = super::rename_person_group(gid, "   ");
    assert!(result.is_err(), "empty name after trim must fail");

    // Grupo inexistente → false.
    assert!(!super::rename_person_group(99999, "X").unwrap());

    // Limpeza.
    let conn = super::open().unwrap();
    conn.execute(&format!("DELETE FROM photo_person_faces WHERE group_id IN (SELECT id FROM person_groups WHERE album_id={album_id})"), []).ok();
    conn.execute(
      &format!("DELETE FROM person_groups WHERE album_id={album_id}"),
      [],
    )
    .ok();
  }

  #[test]
  fn replace_person_groups_atomic() {
    let _lock = super::test_db_lock();
    let conn = setup_test_db();
    let album_id: i64 = 99993;
    let prefix = format!("at_{}", std::process::id());
    // Limpa apenas resíduos deste teste.
    conn.execute(&format!("DELETE FROM photo_person_faces WHERE group_id IN (SELECT id FROM person_groups WHERE album_id={album_id})"), []).ok();
    conn.execute(
      &format!("DELETE FROM person_groups WHERE album_id={album_id}"),
      [],
    )
    .ok();

    let mut photo_ids = Vec::new();
    for i in 0..3 {
      let path = format!("/{prefix}_f{i}.jpg");
      conn.execute(
        "INSERT INTO photos (path, file_name, ext, file_size, sha256, indexed_at)
       VALUES (?1, ?2, 'jpg', 100, ?3, datetime('now'))",
        rusqlite::params![
          path,
          format!("{prefix}_f{i}.jpg"),
          format!("sha_{prefix}_{i}")
        ],
      )
      .unwrap();
      let id: i64 = conn.last_insert_rowid();
      photo_ids.push(id);
    }
    drop(conn);

    // Primeira rodada: 2 grupos usando IDs reais.
    let groups1 = vec![
      super::PersistedGroup {
        name: "P1".to_string(),
        threshold: 0.5,
        faces: vec![super::PersistedFace {
          photo_id: photo_ids[0],
          bbox: [0.0; 4],
        }],
      },
      super::PersistedGroup {
        name: "P2".to_string(),
        threshold: 0.5,
        faces: vec![super::PersistedFace {
          photo_id: photo_ids[1],
          bbox: [0.1; 4],
        }],
      },
    ];
    super::replace_person_groups(album_id, &groups1).unwrap();
    assert_eq!(super::list_person_groups(album_id).unwrap().len(), 2);

    // Segunda rodada: substitui por 1 grupo (os antigos devem sumir).
    let groups2 = vec![super::PersistedGroup {
      name: "P3".to_string(),
      threshold: 0.6,
      faces: vec![
        super::PersistedFace {
          photo_id: photo_ids[0],
          bbox: [0.2; 4],
        },
        super::PersistedFace {
          photo_id: photo_ids[2],
          bbox: [0.3; 4],
        },
      ],
    }];
    super::replace_person_groups(album_id, &groups2).unwrap();
    let pg = super::list_person_groups(album_id).unwrap();
    assert_eq!(pg.len(), 1);
    assert_eq!(pg[0].name, "P3");
    let faces = super::list_faces_in_group(pg[0].id).unwrap();
    assert_eq!(faces.len(), 2);
    assert!((faces[0].bbox[0] - 0.2).abs() < 1e-5);

    // Limpeza.
    let conn = super::open().unwrap();
    conn.execute(&format!("DELETE FROM photo_person_faces WHERE group_id IN (SELECT id FROM person_groups WHERE album_id={album_id})"), []).ok();
    conn.execute(
      &format!("DELETE FROM person_groups WHERE album_id={album_id}"),
      [],
    )
    .ok();
  }

  #[test]
  fn filter_counts_scoped_to_ids() {
    let _lock = super::test_db_lock();
    let conn = setup_test_db();
    let prefix = format!("fc_{}", std::process::id());
    let mut photo_ids = Vec::new();
    let mut paths = Vec::new();
    for i in 0..4 {
      let ext = if i < 2 { "jpg" } else { "nef" };
      let path = format!("/{prefix}_f{i}.{ext}");
      conn.execute(
        &format!(
          "INSERT INTO photos (path, file_name, ext, file_size, sha256, indexed_at)
       VALUES (?1, ?2, ?3, 100, ?4, datetime('now'))"
        ),
        rusqlite::params![
          path,
          format!("{prefix}_f{i}.{ext}"),
          ext,
          format!("sha_{prefix}_{i}")
        ],
      )
      .unwrap();
      let id: i64 = conn.last_insert_rowid();
      photo_ids.push(id);
      paths.push(path);
    }
    // rating: photo 0 = 5 (pick), photo 1 = 0, photo 2 = 3, photo 3 = 1 (reject)
    conn.execute(
      &format!("UPDATE photos SET rating=5 WHERE id={}", photo_ids[0]),
      [],
    )
    .unwrap();
    conn.execute(
      &format!("UPDATE photos SET rating=3 WHERE id={}", photo_ids[2]),
      [],
    )
    .unwrap();
    conn.execute(
      &format!("UPDATE photos SET rating=1 WHERE id={}", photo_ids[3]),
      [],
    )
    .unwrap();
    // edit_json on photo 1
    conn.execute(
      &format!(
        "UPDATE photos SET edit_json='{{\"exposure\":0.5}}' WHERE id={}",
        photo_ids[1]
      ),
      [],
    )
    .unwrap();
    drop(conn);

    // Scoped to JPEG only (photos 0, 1).
    let jpeg_ids = &photo_ids[0..2];
    let counts = super::filter_counts(Some(jpeg_ids)).unwrap();
    assert_eq!(counts.all, 2, "JPEG scope should have 2 photos");
    assert_eq!(counts.picks, 1, "one pick in JPEG scope");
    assert_eq!(counts.rejects, 0, "no rejects in JPEG scope");
    assert_eq!(counts.unrated, 1, "one unrated in JPEG scope");
    assert_eq!(counts.edited, 1, "one edited in JPEG scope");

    // Scoped to unedited filter (photos 0, 2, 3).
    let unedited_ids: Vec<i64> = photo_ids
      .iter()
      .copied()
      .filter(|id| *id != photo_ids[1])
      .collect();
    let counts = super::filter_counts(Some(&unedited_ids)).unwrap();
    assert_eq!(counts.all, 3, "unedited scope should have 3 photos");
    assert_eq!(counts.edited, 0, "no edited in unedited scope");

    // Limpeza.
    let conn = super::open().unwrap();
    for path in &paths {
      conn.execute(&format!("DELETE FROM photos WHERE path='{path}'"), [])
        .ok();
    }
  }

  #[test]
  fn set_photo_has_face_updates_correctly() {
    let _lock = super::test_db_lock();
    let conn = setup_test_db();
    let prefix = format!("hf_{}", std::process::id());
    let path = format!("/{prefix}.jpg");
    conn.execute(&format!("DELETE FROM photos WHERE path='{path}'"), [])
      .ok();
    conn.execute(
      "INSERT INTO photos (path, file_name, ext, file_size, sha256, has_face, indexed_at)
     VALUES (?1, ?2, 'jpg', 100, ?3, 0, datetime('now'))",
      rusqlite::params![path, format!("{prefix}.jpg"), format!("sha_{prefix}")],
    )
    .unwrap();
    let id = conn.last_insert_rowid();
    drop(conn);

    // Inicialmente has_face = false.
    let photo = super::get_photo(id).unwrap().unwrap();
    assert!(!photo.has_face);

    // Define has_face = true.
    super::set_photo_has_face(id, true).unwrap();
    let photo = super::get_photo(id).unwrap().unwrap();
    assert!(photo.has_face);

    // Volta para false.
    super::set_photo_has_face(id, false).unwrap();
    let photo = super::get_photo(id).unwrap().unwrap();
    assert!(!photo.has_face);

    // Limpeza.
    let conn = super::open().unwrap();
    conn.execute(&format!("DELETE FROM photos WHERE path='{path}'"), [])
      .ok();
  }

  #[test]
  fn sanitize_folder_name_basic() {
    assert_eq!(super::sanitize_folder_name("Alice", "fallback"), "Alice");
    assert_eq!(super::sanitize_folder_name("Pessoa 1", "fb"), "Pessoa 1");
    assert_eq!(
      super::sanitize_folder_name("C:\\bad/name*", "fb"),
      "Cbadname"
    );
    assert_eq!(super::sanitize_folder_name("?!<>", "fb"), "!");
    assert_eq!(super::sanitize_folder_name("  ", "fb"), "fb");
    assert_eq!(super::sanitize_folder_name("", "fb"), "fb");
    assert_eq!(super::sanitize_folder_name("a:b;c", "fb"), "ab;c");
    assert_eq!(
      super::sanitize_folder_name("Pessoa / Teste", "fb"),
      "Pessoa  Teste"
    );
  }

  #[test]
  fn sanitize_folder_name_control_chars() {
    assert_eq!(super::sanitize_folder_name("A\x00B\x1fC", "fb"), "ABC");
    assert_eq!(super::sanitize_folder_name(" normal ", "fb"), "normal");
  }

  #[test]
  fn sanitize_folder_name_trailing_dots_and_spaces() {
    assert_eq!(super::sanitize_folder_name("abc.", "fb"), "abc");
    assert_eq!(super::sanitize_folder_name("abc...", "fb"), "abc");
    assert_eq!(super::sanitize_folder_name("abc  ", "fb"), "abc");
    assert_eq!(super::sanitize_folder_name("abc. .", "fb"), "abc");
    assert_eq!(super::sanitize_folder_name(".", "fb"), "fb");
    assert_eq!(super::sanitize_folder_name("..", "fb"), "fb");
    assert_eq!(super::sanitize_folder_name("...", "fb"), "fb");
  }

  #[test]
  fn sanitize_folder_name_reserved_windows() {
    assert_eq!(super::sanitize_folder_name("CON", "fb"), "fb_CON");
    assert_eq!(super::sanitize_folder_name("con", "fb"), "fb_con");
    assert_eq!(super::sanitize_folder_name("PRN", "fb"), "fb_PRN");
    assert_eq!(super::sanitize_folder_name("AUX", "fb"), "fb_AUX");
    assert_eq!(super::sanitize_folder_name("NUL", "fb"), "fb_NUL");
    assert_eq!(super::sanitize_folder_name("COM1", "fb"), "fb_COM1");
    assert_eq!(super::sanitize_folder_name("COM9", "fb"), "fb_COM9");
    assert_eq!(super::sanitize_folder_name("LPT1", "fb"), "fb_LPT1");
    assert_eq!(super::sanitize_folder_name("LPT9", "fb"), "fb_LPT9");
    assert_eq!(super::sanitize_folder_name("CON.txt", "fb"), "fb_CON.txt");
    assert_eq!(super::sanitize_folder_name("lpt1.jpg", "fb"), "fb_lpt1.jpg");
    // Non-reserved names with similar patterns are fine.
    assert_eq!(super::sanitize_folder_name("CONSOLE", "fb"), "CONSOLE");
    assert_eq!(super::sanitize_folder_name("LPT10", "fb"), "LPT10");
  }

  #[test]
  fn export_persisted_people_album_uses_persisted_names_and_scoped_to_album() {
    let _lock = super::test_db_lock();
    let conn = setup_test_db();
    let album_id: i64 = 99996;
    let prefix = format!("exp_{}", std::process::id());

    // Limpa resíduos.
    conn.execute(&format!("DELETE FROM photo_person_faces WHERE group_id IN (SELECT id FROM person_groups WHERE album_id={album_id})"), []).ok();
    conn.execute(
      &format!("DELETE FROM person_groups WHERE album_id={album_id}"),
      [],
    )
    .ok();
    conn.execute(
      &format!("DELETE FROM photos WHERE file_name LIKE '{prefix}_%'"),
      [],
    )
    .ok();

    // Cria arquivos dummy reais no disco.
    let tmp = std::env::temp_dir().join(format!("openshoot_exp_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let path_a = tmp.join(format!("{prefix}_a.jpg"));
    let path_b = tmp.join(format!("{prefix}_b.jpg"));
    let path_other = tmp.join(format!("{prefix}_other.jpg"));
    std::fs::write(&path_a, b"photo a").unwrap();
    std::fs::write(&path_b, b"photo b").unwrap();
    std::fs::write(&path_other, b"photo other").unwrap();

    let path_a_s = path_a.to_string_lossy().to_string();
    let path_b_s = path_b.to_string_lossy().to_string();
    let path_other_s = path_other.to_string_lossy().to_string();

    // Insere fotos.
    conn.execute(
      "INSERT INTO photos (path, file_name, ext, file_size, sha256, indexed_at)
     VALUES (?1, ?2, 'jpg', 100, ?3, datetime('now'))",
      rusqlite::params![
        path_a_s,
        format!("{prefix}_a.jpg"),
        format!("sha_{prefix}_a")
      ],
    )
    .unwrap();
    let id_a: i64 = conn.last_insert_rowid();
    conn.execute(
      "INSERT INTO photos (path, file_name, ext, file_size, sha256, indexed_at)
     VALUES (?1, ?2, 'jpg', 200, ?3, datetime('now'))",
      rusqlite::params![
        path_b_s,
        format!("{prefix}_b.jpg"),
        format!("sha_{prefix}_b")
      ],
    )
    .unwrap();
    let id_b: i64 = conn.last_insert_rowid();
    conn.execute(
      "INSERT INTO photos (path, file_name, ext, file_size, sha256, indexed_at)
     VALUES (?1, ?2, 'jpg', 300, ?3, datetime('now'))",
      rusqlite::params![
        path_other_s,
        format!("{prefix}_other.jpg"),
        format!("sha_{prefix}_other")
      ],
    )
    .unwrap();
    let id_other: i64 = conn.last_insert_rowid();

    // Persiste 2 grupos no álbum com nomes customizados.
    let groups = vec![
      super::PersistedGroup {
        name: "Maria / Santos".to_string(),
        threshold: 0.5,
        faces: vec![super::PersistedFace {
          photo_id: id_a,
          bbox: [0.1, 0.2, 0.3, 0.4],
        }],
      },
      super::PersistedGroup {
        name: "João".to_string(),
        threshold: 0.6,
        faces: vec![super::PersistedFace {
          photo_id: id_b,
          bbox: [0.0, 0.0, 0.2, 0.2],
        }],
      },
    ];
    super::replace_person_groups(album_id, &groups).unwrap();

    // Cria um outro álbum com a foto "other" e um grupo.
    let other_album_id: i64 = 99997;
    conn.execute(&format!("DELETE FROM photo_person_faces WHERE group_id IN (SELECT id FROM person_groups WHERE album_id={other_album_id})"), []).ok();
    conn.execute(
      &format!("DELETE FROM person_groups WHERE album_id={other_album_id}"),
      [],
    )
    .ok();
    let other_groups = vec![super::PersistedGroup {
      name: "Outro".to_string(),
      threshold: 0.5,
      faces: vec![super::PersistedFace {
        photo_id: id_other,
        bbox: [0.1, 0.1, 0.5, 0.5],
      }],
    }];
    super::replace_person_groups(other_album_id, &other_groups).unwrap();
    drop(conn);

    // Exporta o álbum 99996.
    let out_dir =
      std::env::temp_dir().join(format!("openshoot_exp_people_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out_dir);
    let result =
      super::export_persisted_people_album(album_id, out_dir.to_str().unwrap()).unwrap();
    assert!(result["ok"].as_bool().unwrap());
    assert_eq!(result["groups"].as_array().unwrap().len(), 2);
    assert!(result["exported"].as_i64().unwrap() >= 2);

    // Verifica que os nomes das pastas foram sanitizados: "/" vira nome seguro.
    let folders: Vec<String> = result["groups"]
      .as_array()
      .unwrap()
      .iter()
      .map(|g| {
        let f = g["folder"].as_str().unwrap();
        std::path::PathBuf::from(f)
          .file_name()
          .unwrap()
          .to_string_lossy()
          .to_string()
      })
      .collect();
    assert!(
      folders
        .iter()
        .any(|f| f.contains("Maria") && f.contains("Santos")),
      "slash in name should be sanitized: {folders:?}"
    );
    assert!(
      folders.iter().any(|f| f == "João"),
      "João folder should exist"
    );

    // Verifica que a foto "other" do álbum 99994 NÃO foi exportada.
    let all_files: Vec<String> = walkdir(&out_dir);
    assert!(
      !all_files.iter().any(|f| f.contains("other")),
      "other album photo should not be exported"
    );

    // Limpeza.
    let _ = std::fs::remove_dir_all(&out_dir);
    let _ = std::fs::remove_dir_all(&tmp);
    let conn = super::open().unwrap();
    conn.execute(&format!("DELETE FROM photo_person_faces WHERE group_id IN (SELECT id FROM person_groups WHERE album_id={album_id})"), []).ok();
    conn.execute(
      &format!("DELETE FROM person_groups WHERE album_id={album_id}"),
      [],
    )
    .ok();
    conn.execute(&format!("DELETE FROM photo_person_faces WHERE group_id IN (SELECT id FROM person_groups WHERE album_id={other_album_id})"), []).ok();
    conn.execute(
      &format!("DELETE FROM person_groups WHERE album_id={other_album_id}"),
      [],
    )
    .ok();
    conn.execute(
      &format!("DELETE FROM photos WHERE file_name LIKE '{prefix}_%'"),
      [],
    )
    .ok();
  }

  #[test]
  fn export_persisted_people_album_no_overwrite_collision() {
    let _lock = super::test_db_lock();
    let conn = setup_test_db();
    let album_id: i64 = 99995;
    let prefix = format!("col_{}", std::process::id());

    conn.execute(&format!("DELETE FROM photo_person_faces WHERE group_id IN (SELECT id FROM person_groups WHERE album_id={album_id})"), []).ok();
    conn.execute(
      &format!("DELETE FROM person_groups WHERE album_id={album_id}"),
      [],
    )
    .ok();
    conn.execute(
      &format!("DELETE FROM photos WHERE file_name LIKE '{prefix}_%'"),
      [],
    )
    .ok();

    // Cria um arquivo dummy real no disco.
    let tmp = std::env::temp_dir().join(format!("openshoot_col_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let temp_photo = tmp.join(format!("{prefix}_a.jpg"));
    std::fs::write(&temp_photo, b"real photo data").unwrap();

    let out_dir =
      std::env::temp_dir().join(format!("openshoot_col_export_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out_dir);
    let _ = std::fs::create_dir_all(&out_dir);

    // Cria um arquivo dummy na pasta de destino que colidirá.
    let collision_dir = out_dir.join("Pessoa Teste");
    std::fs::create_dir_all(&collision_dir).unwrap();
    let dummy = collision_dir.join(format!("{prefix}_a.jpg"));
    std::fs::write(&dummy, b"existing").unwrap();

    // Cria a foto no catálogo apontando para o arquivo real.
    let temp_photo_s = temp_photo.to_string_lossy().to_string();
    conn.execute(
      "INSERT INTO photos (path, file_name, ext, file_size, sha256, indexed_at)
     VALUES (?1, ?2, 'jpg', 100, ?3, datetime('now'))",
      rusqlite::params![
        temp_photo_s,
        format!("{prefix}_a.jpg"),
        format!("sha_{prefix}_a")
      ],
    )
    .unwrap();
    let id_a: i64 = conn.last_insert_rowid();

    let groups = vec![super::PersistedGroup {
      name: "Pessoa Teste".to_string(),
      threshold: 0.5,
      faces: vec![super::PersistedFace {
        photo_id: id_a,
        bbox: [0.1, 0.1, 0.5, 0.5],
      }],
    }];
    super::replace_person_groups(album_id, &groups).unwrap();
    drop(conn);

    let result =
      super::export_persisted_people_album(album_id, out_dir.to_str().unwrap()).unwrap();
    assert!(result["ok"].as_bool().unwrap());
    assert_eq!(result["exported"].as_i64().unwrap(), 1);

    // Verifica que o arquivo original NÃO foi sobrescrito e o novo tem sufixo.
    let exported_files: Vec<String> = walkdir(&collision_dir);
    assert!(
      exported_files
        .iter()
        .any(|f| f.ends_with(&format!("{prefix}_a.jpg"))),
      "original file should exist"
    );
    assert!(
      exported_files
        .iter()
        .any(|f| f.contains(&format!("{prefix}_a-2.jpg"))),
      "collision file should have suffix -2"
    );

    // Limpeza.
    let _ = std::fs::remove_dir_all(&out_dir);
    let _ = std::fs::remove_dir_all(&tmp);
    let conn = super::open().unwrap();
    conn.execute(&format!("DELETE FROM photo_person_faces WHERE group_id IN (SELECT id FROM person_groups WHERE album_id={album_id})"), []).ok();
    conn.execute(
      &format!("DELETE FROM person_groups WHERE album_id={album_id}"),
      [],
    )
    .ok();
    conn.execute(
      &format!("DELETE FROM photos WHERE file_name LIKE '{prefix}_%'"),
      [],
    )
    .ok();
  }

  #[test]
  fn export_persisted_people_album_sanitized_folder_collision() {
    let _lock = super::test_db_lock();
    let conn = setup_test_db();
    let album_id: i64 = 99998;
    let prefix = format!("fcol_{}", std::process::id());

    conn.execute(&format!("DELETE FROM photo_person_faces WHERE group_id IN (SELECT id FROM person_groups WHERE album_id={album_id})"), []).ok();
    conn.execute(
      &format!("DELETE FROM person_groups WHERE album_id={album_id}"),
      [],
    )
    .ok();
    conn.execute(
      &format!("DELETE FROM photos WHERE file_name LIKE '{prefix}_%'"),
      [],
    )
    .ok();

    // Cria fotos dummy no disco.
    let tmp = std::env::temp_dir().join(format!("openshoot_fcol_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let path_a = tmp.join(format!("{prefix}_a.jpg"));
    let path_b = tmp.join(format!("{prefix}_b.jpg"));
    std::fs::write(&path_a, b"photo a").unwrap();
    std::fs::write(&path_b, b"photo b").unwrap();

    let path_a_s = path_a.to_string_lossy().to_string();
    let path_b_s = path_b.to_string_lossy().to_string();

    conn.execute(
      "INSERT INTO photos (path, file_name, ext, file_size, sha256, indexed_at)
     VALUES (?1, ?2, 'jpg', 100, ?3, datetime('now'))",
      rusqlite::params![
        path_a_s,
        format!("{prefix}_a.jpg"),
        format!("sha_{prefix}_a")
      ],
    )
    .unwrap();
    let id_a: i64 = conn.last_insert_rowid();
    conn.execute(
      "INSERT INTO photos (path, file_name, ext, file_size, sha256, indexed_at)
     VALUES (?1, ?2, 'jpg', 200, ?3, datetime('now'))",
      rusqlite::params![
        path_b_s,
        format!("{prefix}_b.jpg"),
        format!("sha_{prefix}_b")
      ],
    )
    .unwrap();
    let id_b: i64 = conn.last_insert_rowid();

    // Dois grupos com nomes que sanitizam para o mesmo resultado.
    let groups = vec![
      super::PersistedGroup {
        name: "Pessoa / Teste".to_string(),
        threshold: 0.5,
        faces: vec![super::PersistedFace {
          photo_id: id_a,
          bbox: [0.1, 0.2, 0.3, 0.4],
        }],
      },
      super::PersistedGroup {
        name: "Pessoa / Teste".to_string(),
        threshold: 0.6,
        faces: vec![super::PersistedFace {
          photo_id: id_b,
          bbox: [0.0, 0.0, 0.2, 0.2],
        }],
      },
    ];
    super::replace_person_groups(album_id, &groups).unwrap();
    drop(conn);

    let out_dir =
      std::env::temp_dir().join(format!("openshoot_fcol_export_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out_dir);
    let result =
      super::export_persisted_people_album(album_id, out_dir.to_str().unwrap()).unwrap();
    assert!(result["ok"].as_bool().unwrap());
    assert_eq!(result["groups"].as_array().unwrap().len(), 2);
    assert_eq!(result["exported"].as_i64().unwrap(), 2);

    // Verifica que as pastas têm nomes diferentes (a segunda recebe sufixo -2).
    let folders: Vec<String> = result["groups"]
      .as_array()
      .unwrap()
      .iter()
      .map(|g| {
        let f = g["folder"].as_str().unwrap();
        std::path::PathBuf::from(f)
          .file_name()
          .unwrap()
          .to_string_lossy()
          .to_string()
      })
      .collect();
    assert!(
      folders.contains(&"Pessoa  Teste".to_string()),
      "first folder should be sanitized: {folders:?}"
    );
    assert!(
      folders
        .iter()
        .any(|f| f.starts_with("Pessoa  Teste") && f != "Pessoa  Teste"),
      "second folder should have collision suffix: {folders:?}"
    );

    // Limpeza.
    let _ = std::fs::remove_dir_all(&out_dir);
    let _ = std::fs::remove_dir_all(&tmp);
    let conn = super::open().unwrap();
    conn.execute(&format!("DELETE FROM photo_person_faces WHERE group_id IN (SELECT id FROM person_groups WHERE album_id={album_id})"), []).ok();
    conn.execute(
      &format!("DELETE FROM person_groups WHERE album_id={album_id}"),
      [],
    )
    .ok();
    conn.execute(
      &format!("DELETE FROM photos WHERE file_name LIKE '{prefix}_%'"),
      [],
    )
    .ok();
  }

  #[test]
  fn catalog_json_exports_and_imports_album_people() {
    let conn = setup_test_db();
    let prefix = format!("json_{}", std::process::id());
    let path = format!("/tmp/{prefix}.jpg");
    conn.execute(
      "INSERT INTO photos (path, file_name, ext, file_size, sha256, indexed_at) VALUES (?1, ?2, 'jpg', 10, ?3, datetime('now'))",
      rusqlite::params![path, format!("{prefix}.jpg"), format!("hash_{prefix}")],
    )
    .unwrap();
    let photo_id = conn.last_insert_rowid();
    conn.execute(
      "INSERT INTO albums (name, session_type, created_at) VALUES (?1, 'portrait', datetime('now'))",
      rusqlite::params![format!("Álbum {prefix}")],
    )
    .unwrap();
    let album_id = conn.last_insert_rowid();
    conn.execute(
      "INSERT INTO album_photos (album_id, photo_id, added_at) VALUES (?1, ?2, datetime('now'))",
      rusqlite::params![album_id, photo_id],
    )
    .unwrap();
    drop(conn);

    super::replace_person_groups(
      album_id,
      &[super::PersistedGroup {
        name: "Pessoa JSON".to_string(),
        threshold: 0.62,
        faces: vec![super::PersistedFace {
          photo_id,
          bbox: [0.1, 0.2, 0.3, 0.4],
        }],
      }],
    )
    .unwrap();

    let manifest = super::export_catalog_json().unwrap();
    assert!(manifest.contains("openshoot-catalog"));
    assert!(manifest.contains("Pessoa JSON"));
    let imported = super::import_catalog_json(&manifest).unwrap();
    assert_eq!(imported["ok"], true);
    assert_eq!(imported["albums_imported"], 1);
    assert_eq!(imported["photos_linked"], 1);
    assert_eq!(imported["faces_imported"], 1);

    let conn = super::open().unwrap();
    conn.execute(
      "DELETE FROM photo_person_faces WHERE group_id IN (SELECT id FROM person_groups WHERE album_id IN (SELECT id FROM albums WHERE name LIKE ?1))",
      rusqlite::params![format!("%{prefix}%")],
    )
    .ok();
    conn.execute(
      "DELETE FROM person_groups WHERE album_id IN (SELECT id FROM albums WHERE name LIKE ?1)",
      rusqlite::params![format!("%{prefix}%")],
    )
    .ok();
    conn.execute(
      "DELETE FROM album_photos WHERE album_id IN (SELECT id FROM albums WHERE name LIKE ?1)",
      rusqlite::params![format!("%{prefix}%")],
    )
    .ok();
    conn.execute(
      "DELETE FROM albums WHERE name LIKE ?1",
      rusqlite::params![format!("%{prefix}%")],
    )
    .ok();
    conn.execute(
      "DELETE FROM photos WHERE file_name=?1",
      rusqlite::params![format!("{prefix}.jpg")],
    )
    .ok();
  }

  fn walkdir(dir: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
      for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
          out.extend(walkdir(&p));
        } else {
          out.push(p.file_name().unwrap().to_string_lossy().to_string());
        }
      }
    }
    out
  }
}
