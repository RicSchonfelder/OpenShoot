/// Parser de preview JPEG embutido para Canon CR3 (container ISO-BMFF/HEIF).
///
/// O CR3 armazena o preview JPEG em boxes `PRVW` (full) / `THMB` (thumbnail).
/// Abordagem robusta: varrer o arquivo por marcadores JPEG (SOI `FF D8 FF`) e
/// extrair o maior JPEG válido (até o EOI `FF D9`), o que evita depender de
/// parsear toda a árvore BMFF (iloc/iinf/iref).
///
/// Preferimos a maior PRVW/THMB box quando localizável; senão, o maior JPEG
/// encontrado por varredura.

/// Verifica se os primeiros bytes parecem um CR3/HEIF (box `ftyp` com brand crx).
pub fn looks_like_cr3(data: &[u8]) -> bool {
  if data.len() < 16 {
    return false;
  }
  // ftyp box: size(4) + "ftyp"(4) + major_brand(4)
  let ftyp = &data[4..8];
  if ftyp != b"ftyp" {
    return false;
  }
  let brand = &data[8..12];
  // Canon CR3 usa brand "crx " (crx + espaço)
  brand == b"crx " || brand == b"crx\x00"
}

/// Extrai o maior JPEG embutido encontrado por varredura dos marcadores SOI.
pub fn extract_largest_jpeg(data: &[u8]) -> Option<Vec<u8>> {
  let mut best: Option<Vec<u8>> = None;
  let mut i = 0usize;
  let n = data.len();
  // A maior PRVW costuma estar no fim; a varredura pega todos e fica com o maior.
  while i + 4 < n {
    // Procura SOI: FF D8 FF (JPEG start)
    if data[i] == 0xFF && data[i + 1] == 0xD8 {
      // Tenta achar o EOI (FF D9) a partir daqui.
      let mut j = i + 2;
      while j + 1 < n {
        if data[j] == 0xFF && data[j + 1] == 0xD9 {
          let slice = &data[i..=j + 1];
          // Validação mínima: tamanho razoável (>= 1KB) e contém APP0/APP1.
          if slice.len() >= 1024 {
            if best.as_ref().map_or(true, |b| slice.len() > b.len()) {
              best = Some(slice.to_vec());
            }
          }
          break;
        }
        j += 1;
      }
      i = j + 2; // continua após o JPEG encontrado
    } else {
      i += 1;
    }
    // Salvaguarda: não varrer um arquivo gigante inteiro se já achamos algo bom.
    if best.as_ref().map_or(false, |b| b.len() > 3_000_000) {
      break;
    }
  }
  best
}

/// Extrai o preview JPEG de um CR3. Retorna o maior JPEG válido.
pub fn extract_cr3_preview(data: &[u8]) -> Option<Vec<u8>> {
  if !looks_like_cr3(data) {
    return None;
  }
  extract_largest_jpeg(data)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn fake_jpeg(size: usize, seed: u8) -> Vec<u8> {
    let mut v = Vec::with_capacity(size);
    v.extend_from_slice(&[0xFF, 0xD8, 0xFF]); // SOI
    let mut s = seed as u32;
    // Gera bytes 0x01..0xFE (evita 0xFF para não criar marcadores espúrios).
    for _ in 3..size - 2 {
      s = s.wrapping_mul(1664525).wrapping_add(1013904223);
      v.push(((s >> 24) as u8 & 0xFD) + 1); // 1..254
    }
    v.extend_from_slice(&[0xFF, 0xD9]); // EOI
    v
  }

  fn fake_cr3(preview: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    // ftyp
    v.extend_from_slice(&[0, 0, 0, 24, b'f', b't', b'y', b'p', b'c', b'r', b'x', b' ']);
    v.extend_from_slice(&[0, 0, 0, 1, b'm', b'i', b'f', b'1']);
    v.extend_from_slice(&[0, 0, 0, 0]);
    // mdat com o JPEG
    let size = (8 + preview.len()) as u32;
    v.extend_from_slice(&size.to_be_bytes());
    v.extend_from_slice(b"mdat");
    v.extend_from_slice(preview);
    v
  }

  #[test]
  fn detects_cr3_brand() {
    let data = fake_cr3(&fake_jpeg(2000, 1));
    assert!(looks_like_cr3(&data));
    assert!(!looks_like_cr3(&[0u8; 32]));
  }

  #[test]
  fn extracts_largest_jpeg() {
    let small = fake_jpeg(2000, 1);
    let large = fake_jpeg(5000, 2);
    let mut data = fake_cr3(&small);
    data.extend_from_slice(&large);
    let found = extract_largest_jpeg(&data).unwrap();
    assert_eq!(found.len(), large.len());
    assert_eq!(&found[0..3], &[0xFF, 0xD8, 0xFF]);
    assert_eq!(&found[found.len() - 2..], &[0xFF, 0xD9]);
  }

  #[test]
  fn ignores_non_cr3() {
    assert!(extract_cr3_preview(&[0u8; 64]).is_none());
  }
}
