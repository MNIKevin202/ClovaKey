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

/// Largest pixel count we are willing to upscale to, so a big screenshot cannot
/// turn one decode attempt into a huge allocation.
const MAX_SCALED_PIXELS: u64 = 40_000_000;

/// Run the detector over one prepared image.
fn scan(luma: &image::GrayImage) -> Vec<String> {
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
    out
}

/// Second decoder. rqrr and quircs are independent implementations and fail on
/// different images — quircs is a port of quirc, which tends to cope better with
/// dense codes, which is exactly where a Google Authenticator export lands.
fn scan_quircs(luma: &image::GrayImage) -> Vec<String> {
    let (w, h) = luma.dimensions();
    let mut decoder = quircs::Quirc::default();
    let mut out = Vec::new();
    for code in decoder.identify(w as usize, h as usize, luma.as_raw()) {
        let Ok(code) = code else { continue };
        if let Ok(data) = code.decode() {
            if let Ok(text) = String::from_utf8(data.payload) {
                if !text.is_empty() {
                    out.push(text);
                }
            }
        }
    }
    out
}

/// Light modules on a dark background — what a screenshot of a dark-themed screen,
/// or a theme-inverted render, produces. The detector expects the opposite.
fn inverted(luma: &image::GrayImage) -> image::GrayImage {
    let mut out = luma.clone();
    for px in out.pixels_mut() {
        px[0] = 255 - px[0];
    }
    out
}

/// Resample by `factor`. Export codes are dense — a phone screenshot can leave each
/// module only a pixel or two wide, which is below what the detector can resolve —
/// so giving it more pixels to work with is usually what unsticks a stubborn image.
/// Returns None when the result would be unreasonably large.
fn scaled(luma: &image::GrayImage, factor: u32) -> Option<image::GrayImage> {
    let (w, h) = luma.dimensions();
    let (nw, nh) = (w.checked_mul(factor)?, h.checked_mul(factor)?);
    if u64::from(nw) * u64::from(nh) > MAX_SCALED_PIXELS {
        return None;
    }
    Some(image::imageops::resize(
        luma,
        nw,
        nh,
        image::imageops::FilterType::CatmullRom,
    ))
}

/// Run both decoders over one image, preferring whichever finds something.
fn scan_both(luma: &image::GrayImage) -> Vec<String> {
    let mut found = scan(luma);
    if found.is_empty() {
        found = scan_quircs(luma);
    }
    found
}

/// Decode every QR code found in an image, returning their text contents.
///
/// One detection pass is not enough in practice. Google Authenticator's export code
/// carries every account at once, so it is dense, and what people actually bring is
/// a screenshot — frequently downscaled, sometimes inverted by a dark theme. Each
/// variation is tried in turn and the first that yields anything wins, so the common
/// case still costs a single pass.
pub fn decode_all(bytes: &[u8]) -> Result<Vec<String>> {
    let luma = load_luma(bytes)?;

    // Cheapest first: most images decode on the very first attempt, and the ladder
    // below only runs when something has already failed.
    let mut found = scan_both(&luma);
    if found.is_empty() {
        found = scan_both(&inverted(&luma));
    }

    // Then progressively more resolution. A dense export code photographed or
    // screenshotted at phone scale can need a surprising amount of upsampling before
    // its modules separate, and one code in a multi-part export failing while its
    // siblings succeed is exactly what that looks like.
    for factor in [2u32, 3, 4] {
        if !found.is_empty() {
            break;
        }
        let Some(bigger) = scaled(&luma, factor) else {
            break;
        };
        found = scan_both(&bigger);
        if found.is_empty() {
            found = scan_both(&inverted(&bigger));
        }
    }

    let mut out: Vec<String> = Vec::new();
    for c in found {
        if !out.contains(&c) {
            out.push(c);
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

    /// Invert a rendered PNG, producing the light-on-dark image you get from a
    /// screenshot of a dark-themed screen.
    fn invert_png(png: &[u8]) -> Vec<u8> {
        let mut img = image::load_from_memory(png).unwrap().to_luma8();
        for px in img.pixels_mut() {
            px[0] = 255 - px[0];
        }
        let mut buf = Vec::new();
        image::DynamicImage::ImageLuma8(img)
            .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        buf
    }

    /// A payload the size of a real export — Google Authenticator packs every
    /// account into one code, so the result is a high-version, dense QR.
    fn dense_migration() -> String {
        format!(
            "otpauth-migration://offline?data={}",
            "CjEKCkhlbGxvIXt9Kv8SBnNlY3JldA".repeat(12)
        )
    }

    /// The regression behind "that image doesn't contain an export QR code": a
    /// light-on-dark code decoded to nothing, because the detector expects dark
    /// modules on a light field, and the UI then blamed the contents.
    #[test]
    fn decodes_an_inverted_migration_qr() {
        let data = "otpauth-migration://offline?data=CjEKCkhlbGxvIXt9Kv8SBnNlY3JldA";
        let png = invert_png(&render_png(data, 6, 4));
        assert_eq!(decode_one(&png).unwrap(), data);
    }

    /// A dense export rendered small, as a downscaled screenshot would be.
    #[test]
    fn decodes_a_dense_migration_qr_at_small_scale() {
        let data = dense_migration();
        let png = render_png(&data, 2, 4);
        assert_eq!(decode_one(&png).unwrap(), data);
    }

    /// ...and dense *and* inverted, which is the worst realistic combination.
    #[test]
    fn decodes_a_dense_inverted_migration_qr() {
        let data = dense_migration();
        let png = invert_png(&render_png(&data, 3, 4));
        assert_eq!(decode_one(&png).unwrap(), data);
    }

    /// The case behind "4 of 5 scanned": in a multi-part export one code can be
    /// denser than its siblings, and at phone-screenshot scale its modules fall
    /// below what the detector resolves — so that one image fails while the rest
    /// succeed. Rendered at one pixel per module, this needs more upsampling than
    /// the first rung of the ladder provides.
    #[test]
    fn decodes_a_dense_migration_qr_at_one_pixel_per_module() {
        let data = dense_migration();
        let png = render_png(&data, 1, 4);
        assert_eq!(decode_one(&png).unwrap(), data);
    }

    /// The second decoder must actually work on its own, not merely compile. If
    /// rqrr always answered first this would silently become dead code, and the
    /// fallback would be worthless precisely when it is needed.
    #[test]
    fn the_second_decoder_reads_a_dense_migration_qr() {
        let data = dense_migration();
        let png = render_png(&data, 4, 4);
        let luma = image::load_from_memory(&png).unwrap().to_luma8();
        let found = scan_quircs(&luma);
        assert!(found.iter().any(|c| *c == data), "quircs found {found:?}");
    }

    /// ...including inverted, since the ladder feeds it inverted images too.
    #[test]
    fn the_second_decoder_reads_an_inverted_qr() {
        let data = "otpauth-migration://offline?data=CjEKCkhlbGxvIXt9Kv8SBnNlY3JldA";
        let png = invert_png(&render_png(data, 6, 4));
        let luma = image::load_from_memory(&png).unwrap().to_luma8();
        let mut found = scan_quircs(&luma);
        if found.is_empty() {
            // quircs, like rqrr, expects dark-on-light; the ladder hands it both.
            let mut inv = luma.clone();
            for px in inv.pixels_mut() {
                px[0] = 255 - px[0];
            }
            found = scan_quircs(&inv);
        }
        assert!(found.iter().any(|c| c == data), "quircs found {found:?}");
    }

    #[test]
    fn finds_nothing_in_a_blank_image() {
        let blank = image::GrayImage::from_pixel(200, 200, image::Luma([255]));
        let mut png = Vec::new();
        image::DynamicImage::ImageLuma8(blank)
            .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        assert!(decode_all(&png).unwrap().is_empty());
    }
}
