use std::path::Path;

use base64::Engine;

use crate::imageproc::read_embedded_jpeg;

/// Carrega a imagem (preview embutido ou arquivo) como RGB8.
fn load_rgb(path: &Path) -> Result<image::RgbImage, String> {
  let img = match read_embedded_jpeg(path) {
    Some(jpeg) => image::ImageReader::new(std::io::Cursor::new(jpeg))
      .with_guessed_format()
      .map_err(|e| e.to_string())?
      .decode()
      .map_err(|e| e.to_string())?,
    None => image::ImageReader::open(path)
      .map_err(|e| e.to_string())?
      .decode()
      .map_err(|e| e.to_string())?,
  };
  Ok(img.to_rgb8())
}

fn to_base64(img: &image::RgbImage) -> Result<String, String> {
  let dynimg = image::DynamicImage::ImageRgb8(img.clone());
  let mut out = std::io::Cursor::new(Vec::new());
  dynimg
    .write_to(&mut out, image::ImageFormat::Jpeg)
    .map_err(|e| e.to_string())?;
  Ok(format!(
    "data:image/jpeg;base64,{}",
    base64::engine::general_purpose::STANDARD.encode(out.into_inner())
  ))
}

/// Ajuste de horizonte automático: detecta linhas dominantes (Hough),
/// estima o ângulo da "linha do horizonte" e rotaciona a imagem.
/// Retorna (base64 do preview nivelado, ângulo aplicado em graus).
pub fn auto_level_base64(
  path: &Path,
  max_dim: u32,
) -> Result<(String, f64), String> {
  let img = load_rgb(path)?;
  let (w, h) = img.dimensions();
  let angle = estimate_horizon_angle(&img);

  // Aplica a rotação (anti-horário; Hough usa y pra baixo → invertemos o sinal).
  let theta = -angle as f32 * std::f32::consts::PI / 180.0;
  let rotated = imageproc::geometric_transformations::rotate_about_center(
    &img,
    theta,
    imageproc::geometric_transformations::Interpolation::Bilinear,
    imageproc::geometric_transformations::Border::Constant(image::Rgb([0u8, 0, 0])),
  );
  let rgb8 = rotated;

  // Thumbnail para o preview (max_dim).
  let (rw, rh) = rgb8.dimensions();
  let scale = (max_dim as f32 / rw as f32).min(max_dim as f32 / rh as f32).min(1.0);
  let out_img = if scale < 1.0 {
    image::DynamicImage::ImageRgb8(rgb8).resize(
      (rw as f32 * scale).max(1.0) as u32,
      (rh as f32 * scale).max(1.0) as u32,
      image::imageops::FilterType::Lanczos3,
    )
  } else {
    image::DynamicImage::ImageRgb8(rgb8)
  };
  let _ = (w, h);
  let b64 = to_base64(&out_img.to_rgb8())?;
  Ok((b64, angle))
}

/// Estima o ângulo (graus) da linha dominante "quase horizontal" via Hough.
fn estimate_horizon_angle(img: &image::RgbImage) -> f64 {
  let gray = image::DynamicImage::ImageRgb8(img.clone()).to_luma8();
  // Suaviza levemente para reduzir ruído nas bordas.
  let blurred = image::imageops::blur(&gray, 2.0);

  let sobel_x = imageproc::gradients::sobel_gradients(&blurred);
  let edges = image::GrayImage::from_fn(sobel_x.width(), sobel_x.height(), |x, y| {
    let px = sobel_x.get_pixel(x, y);
    let mag = ((px[0] as f32) * (px[0] as f32)
      + (px[1] as f32) * (px[1] as f32))
      .sqrt();
    // Limiar simples: bordas fortes.
    if mag > 120.0 {
      image::Luma([255u8])
    } else {
      image::Luma([0u8])
    }
  });

  let options = imageproc::hough::LineDetectionOptions {
    vote_threshold: 60,
    suppression_radius: 8,
  };
  let lines = imageproc::hough::detect_lines(&edges, options);
  if lines.is_empty() {
    return 0.0;
  }

  // Só considera linhas "quase horizontais" (ângulo perto de 0° ou 180°).
  // PolarLine.angle_in_degrees é o ângulo da linha em relação ao eixo x.
  let mut angles: Vec<f64> = Vec::new();
  for l in &lines {
    let a = l.angle_in_degrees as f64;
    // Desvio da horizontal: min(a, 180-a). Aceita ±15°.
    let delta = if a > 90.0 { 180.0 - a } else { a };
    if delta < 15.0 {
      // Inclinação real: se a < 90 a linha sobe à direita (+), se a > 90 desce (-).
      let line_angle = if a <= 90.0 { a } else { a - 180.0 };
      angles.push(line_angle);
    }
  }
  if angles.is_empty() {
    return 0.0;
  }
  // Mediana dos ângulos.
  angles.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
  let mid = angles[angles.len() / 2];
  mid.clamp(-10.0, 10.0)
}

/// Recorte automático com IA: centraliza no sujeito (faces detectadas via SCRFD)
/// e remove margens vazias. Retorna o preview recortado (base64).
pub fn ai_crop_base64(path: &Path, max_dim: u32) -> Result<String, String> {
  let img = load_rgb(path)?;
  let (w, h) = img.dimensions();
  let rgb = img.as_raw();

  // Detecta faces (se modelos disponíveis).
  let mut face_boxes: Vec<[f32; 4]> = Vec::new();
  if crate::ml::models_available() {
    if let Ok(faces) = crate::ml::detect_faces(rgb, w, h, 0.5) {
      face_boxes = faces;
    }
  }

  // Centro de interesse: média dos centros das faces, senão centro da imagem.
  let (cx, cy) = if !face_boxes.is_empty() {
    let cx = face_boxes.iter().map(|f| (f[0] + f[2]) / 2.0).sum::<f32>() / face_boxes.len() as f32;
    let cy = face_boxes.iter().map(|f| (f[1] + f[3]) / 2.0).sum::<f32>() / face_boxes.len() as f32;
    (cx, cy)
  } else {
    (0.5, 0.5)
  };

  // Recorte quadrado de 80% da menor dimensão, centrado no sujeito.
  let side = (w.min(h) as f32 * 0.8) as u32;
  let x0 = ((cx * w as f32) - side as f32 / 2.0).clamp(0.0, (w - side).max(0) as f32) as u32;
  let y0 = ((cy * h as f32) - side as f32 / 2.0).clamp(0.0, (h - side).max(0) as f32) as u32;
  let cropped = image::imageops::crop_imm(&img, x0, y0, side.min(w), side.min(h)).to_image();

  // Thumbnail para o preview.
  let (cw, ch) = cropped.dimensions();
  let scale = (max_dim as f32 / cw as f32).min(max_dim as f32 / ch as f32).min(1.0);
  let out_img = if scale < 1.0 {
    image::DynamicImage::ImageRgb8(cropped).resize(
      (cw as f32 * scale).max(1.0) as u32,
      (ch as f32 * scale).max(1.0) as u32,
      image::imageops::FilterType::Lanczos3,
    )
  } else {
    image::DynamicImage::ImageRgb8(cropped)
  };
  to_base64(&out_img.to_rgb8())
}
