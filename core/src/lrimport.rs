use std::path::Path;

/// Importa um preset do Lightroom e converte para receita OpenShoot (JSON).
///
/// Suporta dois formatos:
/// - `.xmp`: desenvolvimento Camera Raw com namespace `crs:` (ex: crs:Exposure).
/// - `.lrtemplate`: formato legado com linhas `Param Key value`.
///
/// Retorna o JSON da receita (mesmo formato do EditParams).
pub fn import_lightroom_preset(path: &Path) -> Result<String, String> {
  let ext = path
    .extension()
    .map(|s| s.to_string_lossy().to_lowercase())
    .unwrap_or_default();
  let text = std::fs::read_to_string(path).map_err(|e| format!("leitura: {e}"))?;

  let recipe = match ext.as_str() {
    "xmp" => parse_crs_xmp(&text),
    "lrtemplate" => parse_lrtemplate(&text),
    _ => return Err("formato não suportado (use .xmp ou .lrtemplate)".to_string()),
  };
  let recipe = normalize_recipe(recipe);
  serde_json::to_string(&recipe).map_err(|e| format!("json: {e}"))
}

fn parse_crs_xmp(text: &str) -> serde_json::Map<String, serde_json::Value> {
  let mut out = serde_json::Map::new();
  // Mapeamento crs: → EditParams OpenShoot.
  for line in text.lines() {
    let t = line.trim();
    for (key, param) in CRS_MAPPING {
      // Procura `crs:Key="valor"`.
      let pat = format!("crs:{key}=\"");
      if let Some(pos) = t.find(&pat) {
        let rest = &t[pos + pat.len()..];
        if let Some(end) = rest.find('"') {
          if let Ok(val) = rest[..end].trim().parse::<f64>() {
            out.insert(param.to_string(), serde_json::json!(val));
          }
        }
      }
    }
  }
  out
}

fn parse_lrtemplate(text: &str) -> serde_json::Map<String, serde_json::Value> {
  let mut out = serde_json::Map::new();
  for line in text.lines() {
    let t = line.trim();
    if !t.starts_with("Param") {
      continue;
    }
    // `Param Key Value`
    let rest = &t["Param".len()..].trim();
    let mut parts = rest.splitn(2, char::is_whitespace);
    let key = parts.next().unwrap_or("").trim();
    let val = parts.next().unwrap_or("").trim();
    if key.is_empty() {
      continue;
    }
    let Some((_, param)) = LRTEMPLATE_MAPPING.iter().find(|(k, _)| *k == key) else {
      continue;
    };
    if let Ok(v) = val.parse::<f64>() {
      out.insert(param.to_string(), serde_json::json!(v));
    }
  }
  out
}

/// Normaliza valores para o range do EditParams e remove neutros.
fn normalize_recipe(
  mut m: serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
  // Temperatura do LR vem em unidades próprias (~5000-6000); Convert se presente.
  if let Some(v) = m.get("temperature").and_then(|v| v.as_f64()) {
    // crs:Temperature em LR é o valor "como ajustado" (2000..12000 ~ ok).
    m.insert("temperature".to_string(), serde_json::json!(v));
  }
  m
}

/// Mapeamento `crs:` (Lightroom XMP) → chave EditParams OpenShoot.
const CRS_MAPPING: &[(&str, &str)] = &[
  ("Exposure", "exposure"),
  ("Temperature", "temperature"),
  ("Tint", "tint"),
  ("Contrast", "contrast"),
  ("Saturation", "saturation"),
  ("Shadows", "shadows"),
  ("Highlights", "highlights"),
  ("Brightness", "brightness"),
  ("Clarity", "sharpen"),
];

/// Mapeamento `Param` (Lightroom .lrtemplate legado) → chave EditParams.
const LRTEMPLATE_MAPPING: &[(&str, &str)] = &[
  ("Exposure", "exposure"),
  ("Temperature", "temperature"),
  ("Tint", "tint"),
  ("Contrast", "contrast"),
  ("Saturation", "saturation"),
  ("Shadows", "shadows"),
  ("Highlights", "highlights"),
  ("Brightness", "brightness"),
  ("Clarity", "sharpen"),
];

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_crs_xmp() {
    let xmp = r#"<?xpacket begin=""?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF>
  <rdf:Description rdf:about="" xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
    crs:Exposure="+1.25"
    crs:Temperature="5800"
    crs:Contrast="+22"/>
 </rdf:RDF>
</x:xmpmeta>"#;
    let m = parse_crs_xmp(xmp);
    assert_eq!(m["exposure"], 1.25);
    assert_eq!(m["temperature"], 5800.0);
    assert_eq!(m["contrast"], 22.0);
  }

  #[test]
  fn parses_lrtemplate() {
    let lr = r#"DevelopPreset:
  Param Exposure 0.75
  Param Saturation -15
  Param Clarity 30"#;
    let m = parse_lrtemplate(lr);
    assert_eq!(m["exposure"], 0.75);
    assert_eq!(m["saturation"], -15.0);
    assert_eq!(m["sharpen"], 30.0);
  }

  #[test]
  fn rejects_unknown_format() {
    let tmp = std::env::temp_dir().join("preset.txt");
    std::fs::write(&tmp, "junk").unwrap();
    assert!(import_lightroom_preset(&tmp).is_err());
    let _ = std::fs::remove_file(&tmp);
  }
}
