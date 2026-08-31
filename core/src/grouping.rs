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
  /// Face da pessoa representada na foto de amostra (coordenadas normalizadas).
  pub sample_face: Option<[f32; 4]>,
  pub photo_ids: Vec<i64>,
  pub photo_paths: Vec<String>,
}

fn representative_face(path: &str, embedding: &[f32]) -> Option<[f32; 4]> {
  let (rgb, width, height) = crate::ml::load_rgb(Path::new(path), 512).ok()?;
  let boxes = crate::ml::detect_faces(&rgb, width, height, 0.5).ok()?;
  let candidates = boxes.into_iter().filter_map(|bbox| {
    let face_embedding = crate::ml::face_embedding(&rgb, width, height, bbox).ok()?;
    Some((bbox, face_embedding))
  });
  best_matching_face(candidates, embedding)
}

fn best_matching_face(
  candidates: impl IntoIterator<Item = ([f32; 4], Vec<f32>)>,
  target: &[f32],
) -> Option<[f32; 4]> {
  candidates
    .into_iter()
    .max_by(|a, b| cosine(target, &a.1).total_cmp(&cosine(target, &b.1)))
    .map(|(bbox, _)| bbox)
}

/// Uma face detectada com embedding, bbox e metadados.
struct FaceItem {
  photo_id: i64,
  photo_path: String,
  embedding: Vec<f32>,
  bbox: [f32; 4],
}

/// Face associada a um grupo, preservando a bbox normalizada.
#[derive(Serialize)]
pub struct GroupedFace {
  pub group_index: usize,
  pub photo_id: i64,
  pub bbox: [f32; 4],
}

/// Resultado completo de uma análise facial. As contagens permitem à interface
/// distinguir "não havia rostos" de fotos que não puderam ser abertas.
pub struct GroupingResult {
  pub groups: Vec<PersonGroup>,
  pub grouped_faces: Vec<GroupedFace>,
  pub photos_scanned: usize,
  pub photos_unavailable: usize,
}

fn unavailable_photos_error(total: usize, unavailable: usize) -> Option<String> {
  if total > 0 && unavailable == total {
    Some(format!(
    "Nenhuma das {total} foto(s) do álbum pôde ser aberta. Verifique se a pasta original continua disponível e atualize o álbum."
  ))
  } else {
    None
  }
}

/// Agrupa fotos por pessoa (similaridade facial via MobileFaceNet).
/// Para cada foto, detecta faces (SCRFD) e gera embeddings (MobileFaceNet).
/// Depois agrupa por similaridade de cosseno (mesma pessoa = ângulo pequeno).
/// `threshold` (0..1): similaridade mínima p/ considerar mesma pessoa (default ~0.5).
/// Retorna (grupos, faces_agrupadas) — as faces_agrupadas trazem group_id + bbox.
pub fn group_by_similarity(photos: &[PhotoPath], threshold: f32) -> Result<GroupingResult, String> {
  if !crate::ml::embedding_available() {
    return Err(
      "modelo MobileFaceNet ausente (reconhecimento facial indisponível)".to_string(),
    );
  }

  // 1) Detecta faces + embeddings de cada foto — em paralelo (rayon) e com
  // cache persistente no catálogo (gap G2: 59s/foto → reuso entre execuções).
  let unavailable = std::sync::atomic::AtomicUsize::new(0);
  let per_photo: Vec<(i64, String, Vec<(Vec<f32>, [f32; 4])>)> = photos
    .par_iter()
    .filter_map(|p| {
      let cached = crate::catalog::get_face_embedding(p.id)
        .ok()
        .flatten()
        .and_then(|blob| parse_embeddings(&blob));
      let embs_bboxes = match cached {
        Some(e) if !e.is_empty() => {
          // Cache serializa só embeddings; detectar novamente para obter bboxes.
          let path = Path::new(&p.path);
          let (rgb, w, h) = match crate::ml::load_rgb(path, 512) {
            Ok(v) => v,
            Err(e) => {
              unavailable.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
              crate::catalog::log_debug(&format!("[group] {}: {}", p.path, e));
              return None;
            }
          };
          let bboxes = match crate::ml::detect_faces(&rgb, w, h, 0.5) {
            Ok(f) => f,
            Err(_) => return None,
          };
          let mut pairs: Vec<(Vec<f32>, [f32; 4])> = Vec::new();
          for (i, bbox) in bboxes.into_iter().enumerate() {
            if i < e.len() {
              pairs.push((e[i].clone(), bbox));
            } else {
              if let Ok(emb) = crate::ml::face_embedding(&rgb, w, h, bbox) {
                pairs.push((emb, bbox));
              }
            }
          }
          pairs
        }
        _ => {
          let path = Path::new(&p.path);
          let (rgb, w, h) = match crate::ml::load_rgb(path, 512) {
            Ok(v) => v,
            Err(e) => {
              unavailable.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
          let mut pairs: Vec<(Vec<f32>, [f32; 4])> = Vec::new();
          for bbox in bboxes {
            match crate::ml::face_embedding(&rgb, w, h, bbox) {
              Ok(emb) => {
                pairs.push((emb.clone(), bbox));
                embs.push(emb);
              }
              Err(e) => {
                crate::catalog::log_debug(&format!(
                  "[group] emb {}: {}",
                  p.path, e
                ));
              }
            }
          }
          if !embs.is_empty() {
            let _ =
              crate::catalog::set_face_embedding(p.id, &serialize_embeddings(&embs));
          }
          pairs
        }
      };
      // Mantém a foto lida mesmo que não tenha rosto, para atualizar has_face.
      Some((p.id, p.path.clone(), embs_bboxes))
    })
    .collect();

  let photos_unavailable = unavailable.load(std::sync::atomic::Ordering::Relaxed);
  if let Some(error) = unavailable_photos_error(photos.len(), photos_unavailable) {
    return Err(error);
  }

  let photos_scanned = per_photo.len();
  let analyzed_ids: std::collections::HashSet<i64> =
    per_photo.iter().map(|(photo_id, _, _)| *photo_id).collect();

  let mut faces: Vec<FaceItem> = Vec::new();
  for (photo_id, photo_path, pairs) in per_photo {
    for (emb, bbox) in pairs {
      faces.push(FaceItem {
        photo_id,
        photo_path: photo_path.clone(),
        embedding: emb,
        bbox,
      });
    }
  }

  // Atualiza has_face somente nas fotos que puderam ser lidas. Fotos indisponíveis
  // não devem ter seu estado apagado por uma análise que não chegou a processá-las.
  let mut photos_with_faces: std::collections::HashSet<i64> = std::collections::HashSet::new();
  for f in &faces {
    photos_with_faces.insert(f.photo_id);
  }
  for &id in &analyzed_ids {
    let _ = crate::catalog::set_photo_has_face(id, photos_with_faces.contains(&id));
  }

  // 2) Agrupa por cosseno (agrupamento guloso por similaridade).
  let mut groups: Vec<Vec<usize>> = Vec::new();
  for i in 0..faces.len() {
    let mut placed = false;
    for g in groups.iter_mut() {
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

  // 3) Converte para PersonGroup e produz face->group associations.
  let mut out = Vec::new();
  let mut grouped_faces = Vec::new();
  for (gi, g) in groups.iter().enumerate() {
    if g.is_empty() {
      continue;
    }
    let mut photo_ids: Vec<i64> = Vec::new();
    let mut photo_paths: Vec<String> = Vec::new();
    for &fi in g {
      let f = &faces[fi];
      grouped_faces.push(GroupedFace {
        group_index: gi,
        photo_id: f.photo_id,
        bbox: f.bbox,
      });
      if !photo_ids.contains(&f.photo_id) {
        photo_ids.push(f.photo_id);
        photo_paths.push(f.photo_path.clone());
      }
    }
    out.push(PersonGroup {
      person_id: gi,
      count: photo_ids.len(),
      sample_path: faces[g[0]].photo_path.clone(),
      sample_face: representative_face(&faces[g[0]].photo_path, &faces[g[0]].embedding),
      photo_ids,
      photo_paths,
    });
  }
  Ok(GroupingResult {
    groups: out,
    grouped_faces,
    photos_scanned,
    photos_unavailable,
  })
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
    vals.chunks(dim)
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
  let result = group_by_similarity(photos, threshold)?;
  let groups = result.groups;

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
  fn representative_face_matches_group_embedding() {
    let candidates = vec![
      ([0.1, 0.1, 0.2, 0.2], vec![0.0, 1.0]),
      ([0.7, 0.2, 0.8, 0.3], vec![1.0, 0.0]),
    ];
    assert_eq!(
      best_matching_face(candidates, &[1.0, 0.0]),
      Some([0.7, 0.2, 0.8, 0.3])
    );
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

  #[test]
  fn reports_an_actionable_error_when_every_photo_is_unavailable() {
    let error = super::unavailable_photos_error(3, 3);
    assert!(error
      .as_deref()
      .is_some_and(|message| message.contains("Nenhuma das 3 foto(s)")));
    assert_eq!(super::unavailable_photos_error(3, 2), None);
  }
}
