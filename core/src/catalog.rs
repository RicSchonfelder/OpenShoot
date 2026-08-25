use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use rusqlite::{Connection, OptionalExtension};

use crate::types::{DuplicateGroup, PhotoList, PhotoMeta, ScanResult};

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
  let has_face: bool = conn
    .query_row(
      "SELECT COUNT(*) FROM pragma_table_info('photos') WHERE name='has_face'",
      [],
      |r| Ok(r.get::<_, i64>(0)? != 0),
    )
    .map_err(|e| e.to_string())?;
  if !has_face {
    conn
      .execute("ALTER TABLE photos ADD COLUMN has_face INTEGER DEFAULT 0", [])
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
    conn
      .execute("ALTER TABLE photos ADD COLUMN review INTEGER DEFAULT 0", [])
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
    conn
      .execute("ALTER TABLE photos ADD COLUMN ai_pick INTEGER DEFAULT 0", [])
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
    conn
      .execute("ALTER TABLE photos ADD COLUMN session_type TEXT DEFAULT ''", [])
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
      conn
        .execute(
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
    conn
      .execute("ALTER TABLE photos ADD COLUMN eyes_score REAL DEFAULT -1", [])
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
    conn
      .execute("ALTER TABLE photos ADD COLUMN face_embedding BLOB", [])
      .map_err(|e| e.to_string())?;
  }
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
  conn
    .execute(
      "UPDATE photos SET face_embedding=?2 WHERE id=?1",
      rusqlite::params![id, blob],
    )
    .map_err(|e| e.to_string())?;
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
        "ext IN ('nef','arw','dng','cr2','cr3','orf','raf','rw2','pef','srw','raw')".to_string(),
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

/// Define apenas o rating (manual, via atalho de teclado). Não mexe no score.
pub fn set_photo_rating_manual(id: i64, rating: i64) -> Result<(), String> {
  let conn = open()?;
  conn
    .execute(
      "UPDATE photos SET rating=?2 WHERE id=?1",
      rusqlite::params![id, rating.clamp(0, 5)],
    )
    .map_err(|e| e.to_string())?;
  Ok(())
}

/// Registra se a foto tem rosto detectado (filtro "faces").
pub fn set_photo_has_face(id: i64, has_face: bool) -> Result<(), String> {
  let conn = open()?;
  conn
    .execute(
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
  conn
    .execute(
      "UPDATE photos SET eyes_score=?2 WHERE id=?1",
      rusqlite::params![id, score.clamp(-1.0, 1.0)],
    )
    .map_err(|e| e.to_string())?;
  Ok(())
}

/// Registra se a foto está no bucket "Para revisão" (score ambíguo).
pub fn set_photo_review(id: i64, review: bool) -> Result<(), String> {
  let conn = open()?;
  conn
    .execute(
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
  conn
    .execute(
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
pub fn filter_counts() -> Result<crate::types::FilterCounts, String> {  let conn = open()?;
  let one = |sql: &str| -> Result<i64, String> {
    conn
      .query_row(sql, [], |r| r.get::<_, i64>(0))
      .map_err(|e| e.to_string())
  };
  Ok(crate::types::FilterCounts {
    all: one("SELECT COUNT(*) FROM photos")?,
    picks: one("SELECT COUNT(*) FROM photos WHERE rating >= 4")?,
    rejects: one("SELECT COUNT(*) FROM photos WHERE rating >= 1 AND rating <= 2")?,
    unrated: one("SELECT COUNT(*) FROM photos WHERE rating = 0")?,
    review: one("SELECT COUNT(*) FROM photos WHERE review = 1")?,
    destaques: one("SELECT COUNT(*) FROM photos WHERE ai_pick = 1")?,
    selecionado: one("SELECT COUNT(*) FROM photos WHERE rating >= 4 AND ai_pick = 0")?,
    duplicates: one(
      "SELECT COUNT(*) FROM photos WHERE sha256 IN (SELECT sha256 FROM photos WHERE sha256 <> '' GROUP BY sha256 HAVING COUNT(*) > 1)",
    )?,
    faces: one("SELECT COUNT(*) FROM photos WHERE has_face = 1")?,
    edited: one("SELECT COUNT(*) FROM photos WHERE edit_json IS NOT NULL AND edit_json <> ''")?,
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
  conn
    .execute("DELETE FROM photos WHERE id=?1", [id])
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
  conn
    .execute(
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

// ---------------- Álbuns ----------------

/// Cria um álbum. Retorna o id do novo álbum.
pub fn create_album(name: &str) -> Result<i64, String> {
  let conn = open()?;
  conn
    .execute(
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
  conn
    .execute("DELETE FROM album_photos WHERE album_id=?1", [id])
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
  conn
    .execute(
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

pub fn scan_folder(
  dir: &str,
  include_subdirs: bool,
  types: &str,
) -> Result<ScanResult, String> {
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
  let walker = wd.into_iter().filter_entry(|e| {
    !e.file_type().is_dir() || e.file_name() != ".gallery"
  });

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
    "jpeg" => matches!(ext.as_str(), "jpg" | "jpeg" | "tiff" | "tif" | "png" | "webp" | "heic" | "heif"),
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
    assert_eq!(presets.iter().filter(|p| p.name == "Perfil Casamento RAW").count(), 1);

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
}
