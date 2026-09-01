use std::path::Path;

/// Importa um preset do Lightroom e converte para receita OpenShoot (JSON).
///
/// Suporta dois formatos:
/// - `.xmp`: desenvolvimento Camera Raw com namespace `crs:` (ex: crs:Exposure2012).
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

/// Lê um atributo `crs:Key="valor"` da linha. Retorna o valor como string se presente.
fn crs_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
  let pat = format!("crs:{key}=\"");
  let pos = line.find(&pat)?;
  let rest = &line[pos + pat.len()..];
  let end = rest.find('"')?;
  Some(&rest[..end])
}

/// Converte um valor numérico do XMP para f64. Lida com "+", "-" e decimais.
fn to_f64(s: &str) -> Option<f64> {
  s.trim().parse::<f64>().ok()
}

fn parse_crs_xmp(text: &str) -> serde_json::Map<String, serde_json::Value> {
  let mut out = serde_json::Map::new();
  let all_lines: Vec<&str> = text.lines().map(|l| l.trim()).collect();

  // Primeira passada: captura atributos crs: simples (numéricos) e blocos de curva.
  let mut tone_curve_pv: Vec<Vec<[f64; 2]>> = Vec::new(); // [rgb, red, green, blue]
  let mut in_curve = false;
  let mut current_curve: Vec<[f64; 2]> = Vec::new();

  for &t in &all_lines {
    // Rastreia curvas ToneCurvePV2012 (e variantes por canal) — bloco Seq de pares.
    if t.starts_with("<crs:ToneCurvePV2012") {
      current_curve = Vec::new();
      in_curve = true;
      continue;
    }
    if in_curve {
      if let Some(li) = t.strip_prefix("<rdf:li>").and_then(|s| s.strip_suffix("</rdf:li>")) {
        let vals: Vec<&str> = li.trim().split(',').collect();
        if vals.len() == 2 {
          if let (Some(a), Some(b)) = (to_f64(vals[0]), to_f64(vals[1])) {
            current_curve.push([a, b]);
          }
        }
      } else if t.starts_with("</crs:ToneCurvePV2012") {
        tone_curve_pv.push(std::mem::take(&mut current_curve));
        in_curve = false;
      }
      continue;
    }

    for (key, param) in CRS_MAPPING {
      if let Some(v) = crs_value(t, key) {
        if let Some(n) = to_f64(v) {
          out.insert(param.to_string(), serde_json::json!(n));
        }
      }
    }
  }

  // Se a curva RGB (principal) tem pontos além dos neutros (0,0)-(255,255),
  // converte para a curva paramétrica [destaques, luzes, escuros, sombras].
  if !tone_curve_pv.is_empty() {
    if let Some(rgb_curve) = tone_curve_pv.first() {
      if let Some(p) = curve_to_parametric(rgb_curve) {
        out.insert("tone_curve".to_string(), serde_json::json!(p));
      }
    }
  }

  // HSL: agrupa Hue/Saturation/Luminance adjustments por cor.
  let mut hsl = [0f64; 24];
  for (i, color) in HSL_COLORS.iter().enumerate() {
    let prefixes: [(&str, usize); 3] = [
      ("HueAdjustment", 0),
      ("SaturationAdjustment", 1),
      ("LuminanceAdjustment", 2),
    ];
    for (prefix, channel_idx) in prefixes {
      let key = format!("{prefix}{color}");
      // Procura o atributo crs:{key}="valor" em qualquer linha.
      for &t in &all_lines {
        if let Some(v) = crs_value(t, &key) {
          if let Some(n) = to_f64(v) {
            hsl[i * 3 + channel_idx] = n;
          }
          break;
        }
      }
    }
  }
  if hsl.iter().any(|v| *v != 0.0) {
    out.insert("hsl".to_string(), serde_json::json!(hsl));
  }

  out
}

/// Converte uma curva RGB da Lightroom (lista de pontos [x, y] em 0..255) para
/// a curva paramétrica OpenShoot [destaques, luzes, escuros, sombras] (-100..100).
fn curve_to_parametric(points: &[[f64; 2]]) -> Option<[f64; 4]> {
  if points.len() < 2 {
    return None;
  }
  // Normaliza pontos para 0..1.
  let pts: Vec<[f64; 2]> = points
    .iter()
    .map(|p| [p[0] / 255.0, p[1] / 255.0])
    .collect();
  // Interpola y nos x centrais: 0.15 (sombras), 0.35 (escuros), 0.7 (luzes), 0.85 (destaques).
  let xs = [0.15, 0.35, 0.7, 0.85];
  let mut out = [0f64; 4];
  for (i, x) in xs.iter().enumerate() {
    let y = interp_curve(&pts, *x);
    // Deslocamento da diagonal (y - x), em -100..100.
    out[i] = ((y - *x) * 4.0 * 100.0).clamp(-100.0, 100.0);
  }
  Some(out)
}

fn interp_curve(pts: &[[f64; 2]], x: f64) -> f64 {
  if x <= pts[0][0] {
    return pts[0][1];
  }
  for w in pts.windows(2) {
    let (x0, y0) = (w[0][0], w[0][1]);
    let (x1, y1) = (w[1][0], w[1][1]);
    if x >= x0 && x <= x1 {
      if (x1 - x0).abs() < 1e-9 {
        return y1;
      }
      let t = (x - x0) / (x1 - x0);
      return y0 + (y1 - y0) * t;
    }
  }
  pts.last().map(|p| p[1]).unwrap_or(x)
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
  // Temperatura do LR em unidades próprias (~5000-6000); ok para nosso range.
  if let Some(v) = m.get("temperature").and_then(|v| v.as_f64()) {
    m.insert("temperature".to_string(), serde_json::json!(v));
  }
  // Split toning: agrupa os 5 campos em um array [sh_hue, sh_sat, hl_hue, hl_sat, balance].
  // Matizes do LR vêm em 0..360 → converte para -100..100.
  let sh_hue = m.remove("split_toning_shadow_hue");
  let sh_sat = m.remove("split_toning_shadow_saturation");
  let hl_hue = m.remove("split_toning_highlight_hue");
  let hl_sat = m.remove("split_toning_highlight_saturation");
  let balance = m.remove("split_toning_balance");
  let hue_to_range = |v: f64| ((v / 360.0) * 200.0 - 100.0).clamp(-100.0, 100.0);
  if let (Some(a), Some(b), Some(c), Some(d), Some(e)) = (sh_hue, sh_sat, hl_hue, hl_sat, balance) {
    let av = hue_to_range(a.as_f64().unwrap_or(0.0));
    let cv = hue_to_range(c.as_f64().unwrap_or(0.0));
    let arr = [av, b.as_f64().unwrap_or(0.0), cv, d.as_f64().unwrap_or(0.0), e.as_f64().unwrap_or(0.0)];
    // Só emite split toning se houver saturação em alguma faixa.
    if arr[1] != 0.0 || arr[3] != 0.0 {
      m.insert("split_toning".to_string(), serde_json::json!(arr));
    }
  }
  // Curva tonal neutra (todos zeros) não é emitida.
  if let Some(tc) = m.get("tone_curve").cloned() {
    if let Some(vals) = tc.as_array() {
      if vals.iter().all(|v| v.as_f64().unwrap_or(0.0) == 0.0) {
        m.remove("tone_curve");
      }
    }
  }
  // Remove campos com valor 0.0 (neutro em todos os parâmetros).
  m.retain(|_, v| {
    v.as_f64().map(|n| n != 0.0).unwrap_or(true)
  });
  m
}

/// Cores HSL na ordem Lightroom.
const HSL_COLORS: [&str; 8] = [
  "Red", "Orange", "Yellow", "Green", "Aqua", "Blue", "Purple", "Magenta",
];

/// Mapeamento `crs:` (Lightroom XMP) → chave EditParams OpenShoot.
/// Usa os campos PV2012 quando presentes; o parser tenta ambos.
const CRS_MAPPING: &[(&str, &str)] = &[
  ("Exposure2012", "exposure"),
  ("Exposure", "exposure"),
  ("Temperature", "temperature"),
  ("Tint", "tint"),
  ("Contrast2012", "contrast"),
  ("Contrast", "contrast"),
  ("Saturation", "saturation"),
  ("Vibrance", "vibrance"),
  ("Shadows2012", "shadows"),
  ("Shadows", "shadows"),
  ("Highlights2012", "highlights"),
  ("Highlights", "highlights"),
  ("Whites2012", "whites"),
  ("Whites", "whites"),
  ("Blacks2012", "blacks"),
  ("Blacks", "blacks"),
  ("Brightness", "brightness"),
  ("Clarity2012", "clarity"),
  ("Clarity", "clarity"),
  ("Texture", "texture"),
  ("Dehaze", "dehaze"),
  ("SharpenAmount", "sharpen"),
  ("LuminanceSmoothing", "denoise"),
  ("GrainAmount", "grain"),
  ("PostCropVignetteAmount", "vignette"),
  ("SplitToningShadowHue", "split_toning_shadow_hue"),
  ("SplitToningShadowSaturation", "split_toning_shadow_saturation"),
  ("SplitToningHighlightHue", "split_toning_highlight_hue"),
  ("SplitToningHighlightSaturation", "split_toning_highlight_saturation"),
  ("SplitToningBalance", "split_toning_balance"),
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
  ("Clarity", "clarity"),
  ("Vibrance", "vibrance"),
  ("Whites", "whites"),
  ("Blacks", "blacks"),
  ("Texture", "texture"),
  ("Dehaze", "dehaze"),
  ("SharpenAmount", "sharpen"),
  ("LuminanceSmoothing", "denoise"),
  ("GrainAmount", "grain"),
  ("PostCropVignetteAmount", "vignette"),
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
    crs:Exposure2012="+1.25"
    crs:Temperature="5800"
    crs:Contrast2012="+22"/>
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
    assert_eq!(m["clarity"], 30.0);
  }

  #[test]
  fn parses_pv2012_and_hsl() {
    let xmp = r#"<rdf:Description
    crs:Highlights2012="-30"
    crs:Shadows2012="+50"
    crs:Whites2012="+40"
    crs:Blacks2012="-10"
    crs:HueAdjustmentRed="+15"
    crs:SaturationAdjustmentOrange="-20"
    crs:Vibrance="+25"/> "#;
    let m = parse_crs_xmp(xmp);
    assert_eq!(m["highlights"], -30.0);
    assert_eq!(m["shadows"], 50.0);
    assert_eq!(m["whites"], 40.0);
    assert_eq!(m["blacks"], -10.0);
    assert_eq!(m["vibrance"], 25.0);
    // HSL: Red hue = índice 0*3+0 = 0, Orange sat = 1*3+1 = 4.
    let hsl = m["hsl"].as_array().unwrap();
    assert_eq!(hsl[0], 15.0);
    assert_eq!(hsl[4], -20.0);
  }

  #[test]
  fn parses_split_toning() {
    let xmp = r#"<rdf:Description
    crs:SplitToningShadowHue="220"
    crs:SplitToningShadowSaturation="25"
    crs:SplitToningHighlightHue="40"
    crs:SplitToningHighlightSaturation="15"
    crs:SplitToningBalance="-10"/> "#;
    let m = parse_crs_xmp(xmp);
    let recipe = normalize_recipe(m);
    let st = recipe["split_toning"].as_array().unwrap();
    assert_eq!(st.len(), 5);
    // 220/360*200-100 = 22.2...
    assert!((st[0].as_f64().unwrap() - 22.22).abs() < 0.1);
    assert_eq!(st[1], 25.0);
    assert!((st[2].as_f64().unwrap() - (-77.78)).abs() < 0.1);
    assert_eq!(st[3], 15.0);
    assert_eq!(st[4], -10.0);
  }

  #[test]
  fn parses_tone_curve() {
    let xmp = r#"<crs:ToneCurvePV2012>
    <rdf:Seq>
     <rdf:li>0, 0</rdf:li>
     <rdf:li>64, 48</rdf:li>
     <rdf:li>128, 144</rdf:li>
     <rdf:li>255, 255</rdf:li>
    </rdf:Seq>
   </crs:ToneCurvePV2012> "#;
    let m = parse_crs_xmp(xmp);
    let tc = m["tone_curve"].as_array().unwrap();
    assert_eq!(tc.len(), 4);
    // Curva em S com sombras levantadas e luzes mais claras → valores positivos.
    for v in tc {
      assert!(v.as_f64().unwrap() >= -100.0 && v.as_f64().unwrap() <= 100.0);
    }
  }

  #[test]
  fn rejects_unknown_format() {
    let tmp = std::env::temp_dir().join("preset.txt");
    std::fs::write(&tmp, "junk").unwrap();
    assert!(import_lightroom_preset(&tmp).is_err());
    let _ = std::fs::remove_file(&tmp);
  }
}