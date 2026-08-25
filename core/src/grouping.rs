use std::path::{Path, PathBuf};

use rayon::prelude::*;
use serde::Serialize;

use crate::catalog::PhotoPath;

/// Resultado do agrupamento por similaridade facial.
#[derive(Serialize)]
pub struct PersonGroup {
  pub person_id: usize,
  /// Nº de fotos no grupo.
  pub count: usize,
  /// Foto representativa (caminho).
  pub sample_path: String,
  pub photo_ids: Vec<i64>,
  pub photo_paths: Vec<String>,
}

/// Uma face detectada com embedding e metadados.
struct FaceItem {
  photo_id: i64,
  photo_path: String,
  embedding: Vec<f32>,
}

/// Agrupa fotos por pessoa (similaridade facial via MobileFaceNet).
/// Para cada foto, detecta faces (SCRFD) e gera embeddings (MobileFaceNet).
/// Depois agrupa por similaridade de cosseno (mesma pessoa = ângulo pequeno).
/// `threshold` (0..1): similaridade mínima p/ considerar mesma pessoa (default ~0.5).
pub fn group_by_similarity(
  photos: &[PhotoPath],
  threshold: f32,
) -> Result<Vec<PersonGroup>, String> {
  if !crate::ml::embedding_available() {
    return Err("modelo MobileFaceNet ausente (reconhecimento facial indisponível)".to_string());
  }

  // 1) Detecta faces + embeddings de cada foto — em paralelo (rayon) e com
  // cache persistente no catálogo (gap G2: 59s/foto → reuso entre execuções).
  // O SCRFD/MobileFaceNet continuam serializados (Mutex da sessão ONNX), mas
  // decode + letterbox + crop rodam em todos os cores.
  let per_photo: Vec<(i64, String, Vec<Vec<f32>>)> = photos
    .par_iter()
    .filter_map(|p| {
      let cached = crate::catalog::get_face_embedding(p.id)
        .ok()
        .flatten()
        .and_then(|blob| parse_embeddings(&blob));
      let embs = match cached {
        Some(e) if !e.is_empty() => e,
        _ => {
          let path = Path::new(&p.path);
          let (rgb, w, h) = match crate::ml::load_rgb(path, 512) {
            Ok(v) => v,
            Err(e) => {
              crate::catalog::log_debug(&format!("[group] {}: {}", p.path, e));
              return None;
            }
          };
          let bboxes = match crate::ml::detect_faces(&rgb, w, h, 0.5) {
            Ok(f) => f,
            Err(e) => {
              crate::catalog::log_debug(&format!("[group] faces {}: {}", p.path, e));
              return None;
            }
          };
          let mut embs: Vec<Vec<f32>> = Vec::new();
          for bbox in bboxes {
            match crate::ml::face_embedding(&rgb, w, h, bbox) {
              Ok(emb) => embs.push(emb),
              Err(e) => {
                crate::catalog::log_debug(&format!("[group] emb {}: {}", p.path, e));
              }
            }
          }
          if !embs.is_empty() {
            let _ = crate::catalog::set_face_embedding(p.id, &serialize_embeddings(&embs));
          }
          embs
        }
      };
      if embs.is_empty() {
        None
      } else {
        Some((p.id, p.path.clone(), embs))
      }
    })
    .collect();

  let mut faces: Vec<FaceItem> = Vec::new();
  for (photo_id, photo_path, embs) in per_photo {
    for emb in embs {
      faces.push(FaceItem {
        photo_id,
        photo_path: photo_path.clone(),
        embedding: emb,
      });
    }
  }

  // 2) Agrupa por cosseno (agrupamento guloso por similaridade).
  let mut groups: Vec<Vec<usize>> = Vec::new(); // índices das faces em cada grupo
  for i in 0..faces.len() {
    let mut placed = false;
    for g in groups.iter_mut() {
      // Compara com a face representativa do grupo (a primeira).
      let rep = &faces[g[0]];
      if cosine(&faces[i].embedding, &rep.embedding) >= threshold {
        g.push(i);
        placed = true;
        break;
      }
    }
    if !placed {
      groups.push(vec![i]);
    }
  }

  // 3) Converte para PersonGroup (uma pessoa por grupo com >=1 face).
  let mut out = Vec::new();
  for (gi, g) in groups.iter().enumerate() {
    if g.is_empty() {
      continue;
    }
    // Diferentes fotos no grupo (uma pessoa pode aparecer em várias).
    let mut photo_ids: Vec<i64> = Vec::new();
    let mut photo_paths: Vec<String> = Vec::new();
    for &fi in g {
      let f = &faces[fi];
      if !photo_ids.contains(&f.photo_id) {
        photo_ids.push(f.photo_id);
        photo_paths.push(f.photo_path.clone());
      }
    }
    out.push(PersonGroup {
      person_id: gi,
      count: photo_ids.len(),
      sample_path: faces[g[0]].photo_path.clone(),
      photo_ids,
      photo_paths,
    });
  }
  Ok(out)
}

/// Serializa N embeddings (todos com o mesmo dim) num BLOB:
 /// [count: u32 LE][f32 LE × count × dim].
fn serialize_embeddings(embs: &[Vec<f32>]) -> Vec<u8> {
  let dim = embs.first().map(|e| e.len()).unwrap_or(0);
  let mut out = Vec::with_capacity(4 + embs.len() * dim * 4);
  out.extend_from_slice(&(embs.len() as u32).to_le_bytes());
  for e in embs {
    for v in e {
      out.extend_from_slice(&v.to_le_bytes());
    }
  }
  out
}

/// Inverso de `serialize_embeddings`; None se o blob estiver corrompido.
fn parse_embeddings(blob: &[u8]) -> Option<Vec<Vec<f32>>> {
  if blob.len() < 4 {
    return None;
  }
  let count = u32::from_le_bytes([blob[0], blob[1], blob[2], blob[3]]) as usize;
  let rest = &blob[4..];
  if count == 0 || rest.len() % 4 != 0 {
    return None;
  }
  let total = rest.len() / 4;
  if total % count != 0 {
    return None;
  }
  let dim = total / count;
  let vals: Vec<f32> = rest
    .chunks_exact(4)
    .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
    .collect();
  Some(
    vals
      .chunks(dim)
      .map(|c| c.to_vec())
      .collect::<Vec<Vec<f32>>>(),
  )
}

/// Similaridade de cosseno entre dois vetores normalizados.
fn cosine(a: &[f32], b: &[f32]) -> f32 {
  if a.len() != b.len() || a.is_empty() {
    return 0.0;
  }
  let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
  dot.clamp(-1.0, 1.0)
}

/// Exporta fotos agrupadas por pessoa para pastas.
/// Cria `out_dir/Pessoa N/` e copia as fotos de cada grupo.
/// Fotos sem rosto vão para `out_dir/Sem rosto/`.
pub fn export_grouped(
  photos: &[PhotoPath],
  out_dir: &Path,
  threshold: f32,
) -> Result<serde_json::Value, String> {
  let groups = group_by_similarity(photos, threshold)?;

  let root = out_dir.to_path_buf();
  std::fs::create_dir_all(&root).map_err(|e| format!("criar {root:?}: {e}"))?;

  // Fotos com rosto → grupo; fotos sem rosto → "Sem rosto".
  let mut exported = 0i64;
  let no_face = 0i64;
  let mut result_groups: Vec<serde_json::Value> = Vec::new();

  for (gi, g) in groups.iter().enumerate() {
    let folder = root.join(format!("Pessoa {}", gi + 1));
    std::fs::create_dir_all(&folder).map_err(|e| format!("criar {folder:?}: {e}"))?;
    let mut copied = 0i64;
    for p in &g.photo_paths {
      let src = PathBuf::from(p);
      let name = src
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("foto_{}.jpg", copied));
      let dest = folder.join(&name);
      if let Err(e) = std::fs::copy(&src, &dest) {
        crate::catalog::log_debug(&format!("[group] copiar {p}: {e}"));
        continue;
      }
      copied += 1;
      exported += 1;
    }
    result_groups.push(serde_json::json!({
      "person_id": gi,
      "folder": folder.display().to_string(),
      "count": copied,
      "sample": g.sample_path,
    }));
  }

  Ok(serde_json::json!({
    "ok": true,
    "out_dir": root.display().to_string(),
    "groups": result_groups,
    "exported": exported,
    "no_face": no_face,
  }))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn cosine_identical_is_one() {
    let a = vec![0.5, 0.5, 0.7071];
    assert!((cosine(&a, &a) - 1.0).abs() < 1e-4);
  }

  #[test]
  fn cosine_orthogonal_is_zero() {
    let a = vec![0.0, 1.0];
    let b = vec![1.0, 0.0];
    assert!(cosine(&a, &b).abs() < 1e-4);
  }

  #[test]
  fn embedding_blob_roundtrip() {
    let embs = vec![vec![0.1f32, -0.2, 0.3], vec![1.0, 2.0, -3.0]];
    let blob = serialize_embeddings(&embs);
    let parsed = parse_embeddings(&blob).unwrap();
    assert_eq!(parsed, embs);
    assert!(parse_embeddings(&blob[..6]).is_none()); // truncado
    assert!(parse_embeddings(&[]).is_none());
  }
}
