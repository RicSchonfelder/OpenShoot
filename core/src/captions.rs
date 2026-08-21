use crate::types::PhotoMeta;

/// Gera metadados de texto (keywords, título, descrição) a partir de dados
/// LOCAIS da foto — câmera, data, rating e contagem de faces. 100% offline.
///
/// O OpenRouter (Fase 6, opt-in) poderá enriquecer este texto depois, mas o
/// gerador local sempre funciona e nunca envia nada à rede.

pub struct Caption {
  pub keywords: Vec<String>,
  pub title: String,
  pub description: String,
}

/// Extrai o fabricante/modelo de forma amigável a partir do campo camera
/// (ex: "Canon EOS R6").
fn friendly_camera(camera: &str) -> &str {
  let trimmed = camera.trim();
  if trimmed.is_empty() {
    return "fotografia";
  }
  trimmed
}

/// Converte taken_at (formato Exif "YYYY:MM:DD HH:MM:SS") numa data legível.
fn friendly_date(taken_at: Option<&str>) -> Option<String> {
  let s = taken_at?.trim();
  // Exif: "2023:05:14 14:30:00"
  if s.len() >= 10 && s.as_bytes()[4] == b':' {
    let y = &s[0..4];
    let m = &s[5..7];
    let d = &s[8..10];
    let months = [
      "", "janeiro", "fevereiro", "março", "abril", "maio", "junho",
      "julho", "agosto", "setembro", "outubro", "novembro", "dezembro",
    ];
    let mi = m.parse::<usize>().unwrap_or(0);
    let month = months.get(mi).copied().unwrap_or("mês");
    return Some(format!("{d} de {month} de {y}"));
  }
  None
}

/// Gera caption local a partir do metadado da foto.
pub fn generate(photo: &PhotoMeta, face_count: usize) -> Caption {
  let camera = friendly_camera(&photo.camera);
  let date = friendly_date(photo.taken_at.as_deref());
  let quality = match photo.rating {
    0 => "não classificada".to_string(),
    5 => "uma das melhores da série".to_string(),
    4 => "um destaque da série".to_string(),
    3 => "uma boa opção".to_string(),
    1..=2 => "candidata a revisão".to_string(),
    _ => "selecionada".to_string(),
  };

  // Keywords: construídas a partir de dados locais.
  let mut keywords = vec!["OpenShoot".to_string()];
  keywords.push(format!("camera:{}", camera));
  if let Some(d) = &date {
    // extrai o ano como keyword
    if let Some(year) = d.split_whitespace().last() {
      keywords.push(format!("year:{}", year));
    }
  }
  if face_count > 0 {
    keywords.push("pessoas".to_string());
    keywords.push(format!("faces:{}", face_count));
  }
  if photo.rating >= 4 {
    keywords.push("pick".to_string());
  } else if photo.rating >= 1 {
    keywords.push("cull".to_string());
  }

  // Título curto.
  let title = match (date.clone(), face_count) {
    (Some(d), n) if n > 0 => format!("Retrato capturado em {d}"),
    (Some(d), _) => format!("Fotografia de {d}"),
    (None, n) if n > 0 => format!("Retrato — {n} pessoa(s)"),
    (None, _) => format!("Fotografia — {camera}"),
  };

  // Descrição em prosa.
  let description = match date {
    Some(ref d) => format!(
      "Capturada com {camera} em {d}, esta foto é {quality}."
    ),
    None => format!("Capturada com {camera}, esta foto é {quality}."),
  };

  Caption {
    keywords,
    title,
    description,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::PhotoMeta;

  fn fake_photo(camera: &str, taken: Option<&str>, rating: i64) -> PhotoMeta {
    PhotoMeta {
      id: 1,
      path: "/x/a.jpg".into(),
      file_name: "a.jpg".into(),
      ext: "jpg".into(),
      file_size: 100,
      width: 0,
      height: 0,
      camera: camera.into(),
      taken_at: taken.map(|s| s.into()),
      rating,
      has_xmp: false,
      preview_available: true,
      cull_score: Some(50.0),
      hash: String::new(),
      has_face: false,
      review: false,
    }
  }

  #[test]
  fn generates_keywords() {
    let p = fake_photo("Canon EOS R6", Some("2023:05:14 14:30:00"), 5);
    let c = generate(&p, 1);
    assert!(c.keywords.iter().any(|k| k.contains("Canon")));
    assert!(c.keywords.contains(&"pessoas".to_string()));
    assert!(c.keywords.contains(&"pick".to_string()));
    assert!(c.title.contains("2023"));
  }

  #[test]
  fn friendly_date_parses_exif() {
    assert_eq!(friendly_date(Some("2023:05:14 14:30:00")), Some("14 de maio de 2023".into()));
    assert_eq!(friendly_date(Some("not a date")), None);
    assert_eq!(friendly_date(None), None);
  }

  #[test]
  fn pick_quality_word() {
    let p = fake_photo("Sony A7IV", None, 5);
    let c = generate(&p, 0);
    assert!(c.description.contains("melhores"));
  }
}
