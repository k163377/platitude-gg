//! What a picture becomes on its way into the store: decoded, cut to its
//! centre square (a face is drawn into a square), averaged down to
//! [`SIDE`] and written back out as PNG. The store holds that alone.
//!
//! Shrinking here is what bounds memory: a file's bytes say nothing about
//! its pixels, and each decode stays resident per person given a picture
//! (`ci/baseline/avatar-shrink-windows-x64.md`). Decoding here keeps the
//! store drawable: the drawing end cannot say what went wrong (an
//! undecodable file draws as an empty ring), so it is refused while the
//! person is still there.
//!
//! Reading is PNG and JPEG only, by content: each additional decoder is
//! surface that runs on a file this application did not make.

use std::io::Cursor;

/// Largest side the store keeps. The biggest face is 40
/// (`Metrics.detailsAvatar`) and a canvas rasterises at device resolution,
/// so this covers a 6.4x display — or a doubled token at 3.2x.
///
/// Smaller pictures keep their own size: upscaling would store only a
/// blurred copy.
pub const SIDE: u32 = 256;

/// Most pixels a picture may have before it is refused: the peak worth
/// holding while one is shrunk (32 megapixels is 128MB decoded).
pub const MAX_PIXELS: u64 = 32_000_000;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PictureError {
    /// Not a PNG or a JPEG, or one that will not come apart. Names the
    /// formats, not a category word (デザイン規約 §アバターを与える).
    #[error("this file is not a PNG or a JPEG that can be read")]
    Unreadable,
    /// Names the dimensions: the ceiling a person was told about is a file
    /// size, so a small file refused here needs them to make sense.
    #[error("this file is {width}x{height}, past the {} megapixels this can take", MAX_PIXELS / 1_000_000)]
    TooManyPixels { width: u32, height: u32 },
    /// Decoded, but the small copy could not be written. Only a bug
    /// reaches this, so the wording asks nothing of anybody.
    #[error("the avatar could not be stored")]
    Unstorable,
}

/// The bytes to store for a picture: PNG, square, at most [`SIDE`].
pub fn normalize(bytes: &[u8]) -> Result<Vec<u8>, PictureError> {
    let kind = kind_of(bytes).ok_or(PictureError::Unreadable)?;
    // From the header alone: a picture too big to hold is never held.
    let (width, height) = dimensions(bytes, kind).ok_or(PictureError::Unreadable)?;
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(PictureError::TooManyPixels { width, height });
    }
    let raw = decode(bytes, kind).ok_or(PictureError::Unreadable)?;
    encode(&shrink(&raw)).ok_or(PictureError::Unstorable)
}

/// One decoded picture, four bytes to a pixel.
struct Raw {
    width: u32,
    height: u32,
    px: Vec<u8>,
}

#[derive(Clone, Copy)]
enum Kind {
    Png,
    Jpeg,
}

/// By content: the extension is the file dialog's business
/// (`avatar::EXTENSIONS`), and a picture saved under the wrong one still
/// draws.
fn kind_of(bytes: &[u8]) -> Option<Kind> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        Some(Kind::Png)
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(Kind::Jpeg)
    } else {
        None
    }
}

/// Width and height from the header alone, without decoding.
fn dimensions(bytes: &[u8], kind: Kind) -> Option<(u32, u32)> {
    match kind {
        Kind::Png => {
            let reader = png::Decoder::new(Cursor::new(bytes)).read_info().ok()?;
            let info = reader.info();
            Some((info.width, info.height))
        }
        Kind::Jpeg => {
            let mut decoder = zune_jpeg::JpegDecoder::new(Cursor::new(bytes));
            decoder.decode_headers().ok()?;
            let info = decoder.info()?;
            Some((u32::from(info.width), u32::from(info.height)))
        }
    }
}

fn decode(bytes: &[u8], kind: Kind) -> Option<Raw> {
    match kind {
        Kind::Png => decode_png(bytes),
        Kind::Jpeg => decode_jpeg(bytes),
    }
}

fn decode_png(bytes: &[u8]) -> Option<Raw> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(
        png::Transformations::normalize_to_color8() | png::Transformations::ALPHA,
    );
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0u8; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buffer).ok()?;
    let px = match info.color_type {
        png::ColorType::Rgba => buffer,
        png::ColorType::Rgb => widen(&buffer, 3, |c| [c[0], c[1], c[2], 255]),
        png::ColorType::GrayscaleAlpha => widen(&buffer, 2, |c| [c[0], c[0], c[0], c[1]]),
        png::ColorType::Grayscale => widen(&buffer, 1, |c| [c[0], c[0], c[0], 255]),
        // The transformations expand a palette: only a misbehaving reader
        // lands here.
        png::ColorType::Indexed => return None,
    };
    Some(Raw {
        width: info.width,
        height: info.height,
        px,
    })
}

fn decode_jpeg(bytes: &[u8]) -> Option<Raw> {
    let mut decoder = zune_jpeg::JpegDecoder::new(Cursor::new(bytes));
    decoder.set_options(
        zune_core::options::DecoderOptions::default()
            .jpeg_set_out_colorspace(zune_core::colorspace::ColorSpace::RGB),
    );
    let rgb = decoder.decode().ok()?;
    let info = decoder.info()?;
    Some(Raw {
        width: u32::from(info.width),
        height: u32::from(info.height),
        px: widen(&rgb, 3, |c| [c[0], c[1], c[2], 255]),
    })
}

fn widen(src: &[u8], stride: usize, to_rgba: impl Fn(&[u8]) -> [u8; 4]) -> Vec<u8> {
    src.chunks_exact(stride).flat_map(to_rgba).collect()
}

/// The centre square, averaged down to [`SIDE`] with a box filter: every
/// source pixel counts exactly once, so nothing shimmers when the result
/// is drawn smaller still.
fn shrink(img: &Raw) -> Raw {
    let edge = img.width.min(img.height);
    let side = SIDE.min(edge).max(1);
    let left = (img.width - edge) / 2;
    let top = (img.height - edge) / 2;
    let mut out = vec![0u8; (side as usize) * (side as usize) * 4];
    for row in 0..side {
        let y0 = top + row * edge / side;
        let y1 = (top + (row + 1) * edge / side).max(y0 + 1);
        for col in 0..side {
            let x0 = left + col * edge / side;
            let x1 = (left + (col + 1) * edge / side).max(x0 + 1);
            let mut sum = [0u64; 4];
            let mut count = 0u64;
            for y in y0..y1 {
                let line = (y as usize) * (img.width as usize) * 4;
                for x in x0..x1 {
                    let at = line + (x as usize) * 4;
                    for (channel, total) in sum.iter_mut().enumerate() {
                        *total += u64::from(img.px[at + channel]);
                    }
                    count += 1;
                }
            }
            let at = ((row as usize) * (side as usize) + col as usize) * 4;
            for (channel, total) in sum.iter().enumerate() {
                // Both spans are at least one pixel wide.
                out[at + channel] = (total / count.max(1)) as u8;
            }
        }
    }
    Raw {
        width: side,
        height: side,
        px: out,
    }
}

/// PNG at the writer's ordinary setting: squeezing harder saves a few
/// kilobytes at the moment a person is waiting for the picture.
fn encode(img: &Raw) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, img.width, img.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(&img.px).ok()?;
        writer.finish().ok()?;
    }
    Some(out)
}

/// Built here so no test needs a binary fixture in the tree.
#[cfg(test)]
pub(crate) fn png_of(width: u32, height: u32, colour: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    let mut px = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            px.extend_from_slice(&colour(x, y));
        }
    }
    encode(&Raw { width, height, px }).expect("encode test png")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_back(bytes: &[u8]) -> Raw {
        decode_png(bytes).expect("the stored bytes are a png")
    }

    fn pixel(img: &Raw, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * img.width + x) as usize) * 4;
        [img.px[at], img.px[at + 1], img.px[at + 2], img.px[at + 3]]
    }

    /// 24x16, three colours across, quality 92: a real JPEG, since nothing
    /// in the tree encodes one.
    const WIDE_JPEG_HEX: &str = concat!(
        "ffd8ffe000104a46494600010200000100010000ffc000110800100018030111",
        "00021101031101ffdb0043000302020202020302020203030303040604040404",
        "04080606050609080a0a090809090a0c0f0c0a0b0e0b09090d110d0e0f101011",
        "100a0c12131210130f101010ffdb00430103030304030408040408100b090b10",
        "1010101010101010101010101010101010101010101010101010101010101010",
        "1010101010101010101010101010101010ffc4001f0000010501010101010100",
        "000000000000000102030405060708090a0bffc400b510000201030302040305",
        "0504040000017d01020300041105122131410613516107227114328191a10823",
        "42b1c11552d1f02433627282090a161718191a25262728292a3435363738393a",
        "434445464748494a535455565758595a636465666768696a737475767778797a",
        "838485868788898a92939495969798999aa2a3a4a5a6a7a8a9aab2b3b4b5b6b7",
        "b8b9bac2c3c4c5c6c7c8c9cad2d3d4d5d6d7d8d9dae1e2e3e4e5e6e7e8e9eaf1",
        "f2f3f4f5f6f7f8f9faffc4001f01000301010101010101010100000000000001",
        "02030405060708090a0bffc400b5110002010204040304070504040001027700",
        "0102031104052131061241510761711322328108144291a1b1c109233352f015",
        "6272d10a162434e125f11718191a262728292a35363738393a43444546474849",
        "4a535455565758595a636465666768696a737475767778797a82838485868788",
        "898a92939495969798999aa2a3a4a5a6a7a8a9aab2b3b4b5b6b7b8b9bac2c3c4",
        "c5c6c7c8c9cad2d3d4d5d6d7d8d9dae2e3e4e5e6e7e8e9eaf2f3f4f5f6f7f8f9",
        "faffda000c03010002110311003f00f12afca8fefd3dcabf3f3fc453e2faff00",
        "648fe953d7abfcad3fd253dcabf3f3fc453e2faff648fe953fffd9",
    );

    fn wide_jpeg() -> Vec<u8> {
        (0..WIDE_JPEG_HEX.len() / 2)
            .map(|i| u8::from_str_radix(&WIDE_JPEG_HEX[i * 2..i * 2 + 2], 16).expect("hex byte"))
            .collect()
    }

    #[test]
    fn a_picture_is_stored_square_at_the_stored_side() {
        let tall = png_of(600, 900, |_, _| [10, 20, 30, 255]);
        let stored = normalize(&tall).expect("normalize");
        let back = read_back(&stored);
        assert_eq!((back.width, back.height), (SIDE, SIDE));
        assert_eq!(pixel(&back, 0, 0), [10, 20, 30, 255]);
    }

    #[test]
    fn the_centre_is_what_survives_a_wide_picture() {
        // Three bands across; only the middle one is inside the square.
        let wide = png_of(900, 300, |x, _| match x / 300 {
            0 => [200, 0, 0, 255],
            1 => [0, 200, 0, 255],
            _ => [0, 0, 200, 255],
        });
        let back = read_back(&normalize(&wide).expect("normalize"));
        for at in [0, SIDE / 2, SIDE - 1] {
            assert_eq!(pixel(&back, at, SIDE / 2), [0, 200, 0, 255], "column {at}");
        }
    }

    #[test]
    fn a_picture_smaller_than_the_side_is_squared_but_not_stretched() {
        let small = png_of(64, 48, |_, _| [1, 2, 3, 255]);
        let back = read_back(&normalize(&small).expect("normalize"));
        assert_eq!((back.width, back.height), (48, 48), "its own size, squared");
    }

    #[test]
    fn a_jpeg_comes_through_the_same_way() {
        let bytes = wide_jpeg();
        assert_eq!(dimensions(&bytes, Kind::Jpeg), Some((24, 16)));
        let back = read_back(&normalize(&bytes).expect("normalize"));
        assert_eq!(
            (back.width, back.height),
            (16, 16),
            "squared, not stretched"
        );
        // The middle band is green, and lossy coding moves the numbers
        // around, so the claim is which channel won.
        let [r, g, b, a] = pixel(&back, 8, 8);
        assert!(g > r && g > b, "the centre band survived: {r},{g},{b}");
        assert_eq!(a, 255);
    }

    #[test]
    fn a_file_that_is_not_a_picture_is_refused() {
        assert_eq!(normalize(b"nothing to see"), Err(PictureError::Unreadable));
        assert_eq!(normalize(&[]), Err(PictureError::Unreadable));
        // A header that says PNG over bytes that are not.
        let mut lying = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        lying.extend_from_slice(b"and then nothing");
        assert_eq!(normalize(&lying), Err(PictureError::Unreadable));
    }

    #[test]
    fn too_many_pixels_is_refused_by_the_header_alone() {
        // 36 megapixels of one colour in a small file: bytes do not bound
        // pixels.
        let huge = png_of(6000, 6000, |_, _| [0, 0, 0, 255]);
        assert!(
            huge.len() < 1_000_000,
            "the file is small: {} bytes",
            huge.len()
        );
        assert_eq!(
            normalize(&huge),
            Err(PictureError::TooManyPixels {
                width: 6000,
                height: 6000
            })
        );
    }

    #[test]
    fn the_same_picture_stores_the_same_bytes() {
        // The store names files after the bytes it writes, so a pipeline
        // that wandered would file one picture under two names.
        let source = png_of(300, 200, |x, y| [(x % 256) as u8, (y % 256) as u8, 40, 255]);
        assert_eq!(
            normalize(&source).expect("once"),
            normalize(&source).expect("twice")
        );
    }
}
