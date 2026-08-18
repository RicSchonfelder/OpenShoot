use std::path::Path;

const XPACKET_UUID: &str = "W5M0MpCehiHzreSzNTczkc9d";

fn escape(s: &str) -> String {
  s.replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
    .replace('\'', "&apos;")
}

/// Escreve um sidecar XMP compatível com Lightroom/Capture One ao lado da foto.
/// rating: 0-5 estrelas (0 = sem classificação). label: "" | Red | Yellow | Green | Blue | Purple.
pub fn write_xmp(
  image_path: &Path,
  rating: i64,
  label: &str,
  keywords: &[String],
) -> Result<std::path::PathBuf, String> {
  let rating = rating.clamp(0, 5);
  let label = label.to_string();
  let valid_labels = ["", "Red", "Yellow", "Green", "Blue", "Purple"];
  if !valid_labels.contains(&label.as_str()) {
    return Err(format!("label inválido: {label}"));
  }

  let label_tag = if label.is_empty() {
    String::new()
  } else {
    format!("      <xmp:Label>{}</xmp:Label>\n", escape(&label))
  };

  let mut iptc_blocks = String::new();
  if !keywords.is_empty() {
    let items: String = keywords
      .iter()
      .filter(|k| !k.trim().is_empty())
      .map(|k| format!("          <rdf:li>{}</rdf:li>\n", escape(k)))
      .collect();
    if !items.is_empty() {
      iptc_blocks.push_str(
        "      <dc:subject>\n        <rdf:Bag>\n",
      );
      iptc_blocks.push_str(&items);
      iptc_blocks.push_str("        </rdf:Bag>\n      </dc:subject>\n");
    }
  }

  let mut ns_decls = vec!["xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\""];
  if !keywords.is_empty() {
    ns_decls.push("xmlns:dc=\"http://purl.org/dc/elements/1.1/\"");
  }
  let ns_str = ns_decls.join("\n        ");

  let body = format!(
    "<?xpacket begin=\"\u{feff}\" id=\"{uuid}\"?>\n\
     <x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"OpenShoot\">\n\
     \x20 <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
     \x20   <rdf:Description rdf:about=\"\"\n\
     \x20       {ns_str}>\n\
     \x20     <xmp:Rating>{rating}</xmp:Rating>\n\
     {label_tag}{iptc_blocks}\
     \x20   </rdf:Description>\n\
     \x20 </rdf:RDF>\n\
     </x:xmpmeta>\n\
     <?xpacket end=\"w\"?>\n",
    uuid = XPACKET_UUID,
    ns_str = ns_str,
    rating = rating,
  );

  let sidecar = image_path.with_extension("xmp");
  std::fs::write(&sidecar, body).map_err(|e| e.to_string())?;
  Ok(sidecar)
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::tempdir;

  #[test]
  fn writes_valid_sidecar() {
    let dir = tempdir().unwrap();
    let img = dir.path().join("IMG_0001.CR3");
    let sidecar = write_xmp(&img, 5, "Green", &["OpenShoot:keep".to_string()]).unwrap();
    assert_eq!(sidecar.file_name().unwrap(), "IMG_0001.xmp");
    let text = std::fs::read_to_string(&sidecar).unwrap();
    assert!(text.contains("<xmp:Rating>5</xmp:Rating>"));
    assert!(text.contains("<xmp:Label>Green</xmp:Label>"));
    assert!(text.contains("OpenShoot:keep"));
    assert!(text.contains(XPACKET_UUID));
  }

  #[test]
  fn clamps_rating() {
    let dir = tempdir().unwrap();
    let img = dir.path().join("a.jpg");
    write_xmp(&img, 99, "", &[]).unwrap();
    let text = std::fs::read_to_string(img.with_extension("xmp")).unwrap();
    assert!(text.contains("<xmp:Rating>5</xmp:Rating>"));
  }

  #[test]
  fn rejects_bad_label() {
    let dir = tempdir().unwrap();
    let img = dir.path().join("a.jpg");
    assert!(write_xmp(&img, 3, "Mauve", &[]).is_err());
  }
}
