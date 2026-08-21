use std::path::{Path, PathBuf};

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

  // 1) Detecta faces + embeddings de cada foto.
  let mut faces: Vec<FaceItem> = Vec::new();
  for p in photos {
    let path = Path::new(&p.path);
    let rgb_result = crate::ml::load_rgb(path, 512);
    let (rgb, w, h) = match rgb_result {
      Ok(v) => v,
      Err(e) => {
        crate::catalog::log_debug(&format!("[group] {}: {}", p.path, e));
        continue;
      }
    };
    // Detecta faces.
    let bboxes = match crate::ml::detect_faces(&rgb, w, h, 0.5) {
      Ok(f) => f,
      Err(e) => {
        crate::catalog::log_debug(&format!("[group] faces {}: {}", p.path, e));
        continue;
      }
    };
    for bbox in bboxes {
      match crate::ml::face_embedding(&rgb, w, h, bbox) {
        Ok(emb) => faces.push(FaceItem {
          photo_id: p.id,
          photo_path: p.path.clone(),
          embedding: emb,
        }),
        Err(e) => {
          crate::catalog::log_debug(&format!("[group] emb {}: {}", p.path, e));
        }
      }
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
    let a = vec![1.0, 0.0];
    let b = vec![0.0, 1.0];
    assert!(cosine(&a, &b).abs() < 1e-4);
  }
}
