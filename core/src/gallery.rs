//! Geração de galeria web estática (equivalente ao "Criar galeria" do AfterShoot).
//!
//! `generate_html` produz um `index.html` self-contained (dark theme, grid
//! responsivo, lightbox via CSS `:target`) sem dependências externas.
//! Convenção de layout gerada pelo chamador:
//! - `photos/<arquivo>`  — imagens em tamanho original (copiadas)
//! - `thumbs/<stem>.jpg` — thumbnails de ~400px (mesmo stem do arquivo)

/// Escapa texto para uso seguro em HTML (títulos e legendas).
fn escape_html(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for c in s.chars() {
    match c {
      '&' => out.push_str("&amp;"),
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      '"' => out.push_str("&quot;"),
      '\'' => out.push_str("&#39;"),
      _ => out.push(c),
    }
  }
  out
}

/// Deriva o caminho relativo do thumbnail a partir do caminho relativo da foto
/// copiada: "photos/foo.CR3" -> "thumbs/foo.jpg".
fn thumb_rel_for(photo_rel: &str) -> String {
  let file_name = std::path::Path::new(photo_rel)
    .file_stem()
    .map(|s| s.to_string_lossy().to_string())
    .unwrap_or_else(|| photo_rel.to_string());
  format!("thumbs/{file_name}.jpg")
}

/// Gera o HTML completo da galeria. Cada item é `(caminho_relativo_da_foto_copiada, legenda)`.
pub fn generate_html(photos: &[(String, String)], title: &str) -> Result<String, String> {
  if title.trim().is_empty() {
    return Err("título vazio".to_string());
  }
  let title = escape_html(title);

  let mut cards = String::new();
  let mut boxes = String::new();
  for (i, (photo_rel, caption)) in photos.iter().enumerate() {
    let id = format!("p{i}");
    let thumb = thumb_rel_for(photo_rel);
    let alt = escape_html(caption);
    cards.push_str(&format!(
      "<a class=\"card\" href=\"#{id}\"><img loading=\"lazy\" src=\"{}\" alt=\"{}\"></a>\n",
      escape_html(&thumb),
      alt
    ));
    boxes.push_str(&format!(
      "<figure class=\"lightbox\" id=\"{id}\">\n<a class=\"close\" href=\"#\" aria-label=\"Close\">&times;</a>\n<img src=\"{}\" alt=\"\">\n<figcaption>{}</figcaption>\n</figure>\n",
      escape_html(photo_rel),
      alt
    ));
  }

  Ok(format!(
    r#"<!DOCTYPE html>
<html lang="pt-BR">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
:root {{ --bg:#0f1115; --panel:#171a21; --text:#e6e8ee; --muted:#8b90a0; --accent:#4f8cff; }}
* {{ box-sizing:border-box; margin:0; padding:0; }}
body {{ background:var(--bg); color:var(--text); font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif; min-height:100vh; }}
header {{ padding:32px 24px 16px; text-align:center; border-bottom:1px solid #232734; }}
header h1 {{ font-size:1.6rem; font-weight:600; letter-spacing:.02em; }}
header p {{ color:var(--muted); margin-top:6px; font-size:.9rem; }}
main {{ max-width:1400px; margin:0 auto; padding:24px; display:grid; gap:12px; grid-template-columns:repeat(auto-fill,minmax(200px,1fr)); }}
.card {{ display:block; aspect-ratio:1/1; overflow:hidden; border-radius:10px; background:var(--panel); transition:transform .15s ease, box-shadow .15s ease; }}
.card img {{ width:100%; height:100%; object-fit:cover; display:block; }}
.card:hover {{ transform:translateY(-2px); box-shadow:0 8px 24px rgba(0,0,0,.45); }}
.lightbox {{ position:fixed; inset:0; background:rgba(8,9,12,.92); display:none; align-items:center; justify-content:center; flex-direction:column; z-index:10; padding:24px; }}
.lightbox:target {{ display:flex; }}
.lightbox img {{ max-width:min(92vw,1600px); max-height:78vh; object-fit:contain; border-radius:8px; }}
.lightbox figcaption {{ margin-top:14px; color:var(--muted); font-size:.95rem; text-align:center; }}
.lightbox .close {{ position:absolute; top:16px; right:24px; font-size:2.2rem; line-height:1; color:var(--text); text-decoration:none; opacity:.8; }}
.lightbox .close:hover {{ opacity:1; }}
footer {{ text-align:center; color:var(--muted); font-size:.8rem; padding:24px 0 40px; }}
</style>
</head>
<body>
<header><h1>{title}</h1><p>{count}</p></header>
<main>
{cards}</main>
{boxes}<footer>OpenShoot</footer>
</body>
</html>
"#,
    count = if photos.len() == 1 {
      "1 foto".to_string()
    } else {
      format!("{} fotos", photos.len())
    },
  ))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn html_contains_doctype_and_title() {
    let photos = vec![
      ("photos/a.jpg".to_string(), "Foto A".to_string()),
      ("photos/b.jpg".to_string(), "Foto <B>".to_string()),
    ];
    let html = generate_html(&photos, "Minha Galeria").expect("html");
    assert!(html.contains("<html"), "deve conter tag <html>");
    assert!(html.contains("Minha Galeria"), "deve conter o título");
    assert!(html.contains("thumbs/a.jpg"));
    assert!(html.contains("photos/a.jpg"));
    // Escape de HTML na legenda.
    assert!(!html.contains("Foto <B>"));
    assert!(html.contains("Foto &lt;B&gt;"));
    assert!(html.contains(":target"), "lightbox via CSS :target");
    assert!(!html.contains("http://") && !html.contains("https://"), "sem dependências externas");
  }
}
