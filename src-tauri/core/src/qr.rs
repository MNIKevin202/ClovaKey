//! Local QR decoding and encoding.
//!
//! Decoding uses the pure-Rust `rqrr` detector on an image decoded with strict
//! resource limits (a defence against decompression-bomb images). **All QR
//! processing happens locally** — nothing is ever uploaded.

use crate::error::{CoreError, Result};
use std::io::Cursor;

/// Refuse absurdly large inputs outright.
const MAX_IMAGE_BYTES: usize = 32 * 1024 * 1024;
const MAX_DIMENSION: u32 = 10_000;

/// Decode an image with conservative limits and return its grayscale form.
fn load_luma(bytes: &[u8]) -> Result<image::GrayImage> {
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return Err(CoreError::QrDecode("image is empty or too large"));
    }

    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(256 * 1024 * 1024);

    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| CoreError::QrDecode("could not read the image"))?;
    reader.limits(limits);

    let img = reader
        .decode()
        .map_err(|_| CoreError::QrDecode("unsupported or corrupt image"))?;
    Ok(img.to_luma8())
}

/// Decode every QR code found in an image, returning their text contents.
pub fn decode_all(bytes: &[u8]) -> Result<Vec<String>> {
    let luma = load_luma(bytes)?;
    let (w, h) = luma.dimensions();
    let mut prepared =
        rqrr::PreparedImage::prepare_from_greyscale(w as usize, h as usize, |x, y| {
            luma.get_pixel(x as u32, y as u32)[0]
        });

    let mut out = Vec::new();
    for grid in prepared.detect_grids() {
        if let Ok((_meta, content)) = grid.decode() {
            if !content.is_empty() {
                out.push(content);
            }
        }
    }
    Ok(out)
}

/// Decode a single QR code from an image (the first one found).
pub fn decode_one(bytes: &[u8]) -> Result<String> {
    decode_all(bytes)?
        .into_iter()
        .next()
        .ok_or(CoreError::QrDecode("no QR code found in the image"))
}

/// Render `data` as a scannable QR code in SVG form (dark-on-light for
/// reliable scanning regardless of the app theme).
pub fn encode_svg(data: &str) -> Result<String> {
    use qrcode::render::svg;
    use qrcode::{EcLevel, QrCode};

    let code = QrCode::with_error_correction_level(data.as_bytes(), EcLevel::M)
        .map_err(|_| CoreError::Message("could not encode a QR code for this data".into()))?;
    let image = code
        .render::<svg::Color>()
        .min_dimensions(220, 220)
        .quiet_zone(true)
        .dark_color(svg::Color("#101915"))
        .light_color(svg::Color("#ffffff"))
        .build();
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_image_bytes() {
        assert!(decode_one(b"not an image").is_err());
        assert!(decode_all(&[]).is_err());
    }

    #[test]
    fn encode_produces_svg() {
        let svg = encode_svg("otpauth://totp/x?secret=JBSWY3DPEHPK3PXP").unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("path") || svg.contains("rect"));
    }

    /// Rasterize a QR code to a PNG so we can exercise the real decode path.
    fn render_png(data: &str, scale: u32, quiet: u32) -> Vec<u8> {
        use qrcode::{Color, QrCode};
        let code = QrCode::new(data.as_bytes()).unwrap();
        let width = code.width();
        let colors = code.to_colors();
        let total = (width as u32 + 2 * quiet) * scale;
        let mut img = image::GrayImage::from_pixel(total, total, image::Luma([255]));
        for my in 0..width {
            for mx in 0..width {
                if colors[my * width + mx] == Color::Dark {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let px = (quiet + mx as u32) * scale + dx;
                            let py = (quiet + my as u32) * scale + dy;
                            img.put_pixel(px, py, image::Luma([0]));
                        }
                    }
                }
            }
        }
        let mut buf = Vec::new();
        image::DynamicImage::ImageLuma8(img)
            .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        buf
    }

    #[test]
    fn encode_then_decode_round_trips() {
        let data = "otpauth://totp/GitHub:kevin@example.com?secret=JBSWY3DPEHPK3PXP&issuer=GitHub";
        let png = render_png(data, 6, 4);
        let decoded = decode_one(&png).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn decodes_migration_qr() {
        let data = "otpauth-migration://offline?data=CjEKCkhlbGxvIXt9Kv8SBnNlY3JldA";
        let png = render_png(data, 6, 4);
        assert_eq!(decode_one(&png).unwrap(), data);
    }
}
