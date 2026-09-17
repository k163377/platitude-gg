//! What a picture becomes on its way into the store.
//!
//! A person picks a file off their own disk and it is drawn at 20 and 40
//! pixels. Nothing else about it is kept: it is decoded, cut to its
//! centre square, averaged down and written back out as PNG. The store
//! holds that alone.
//!
//! Doing it here settles four things at once (measured:
//! `ci/baseline/avatar-shrink-windows-x64.md`).
//!
//! **Memory.** The ceiling the store had was on bytes, and bytes say
//! nothing about pixels: an ordinary telephone photograph is a fifth of
//! it and 46MB decoded, and a flat PNG decodes to about a thousand times
//! its own size. One decode is kept for as long as anything references
//! it, so that cost is paid per person given a picture, against a
//! 300MB budget for the whole application.
//!
//! **Redrawing.** `ctx.drawImage(url, …)` re-samples the source on every
//! repaint, and a repaint is not rare — every visible row redraws while
//! the graph column's divider is dragged.
//!
//! **Shape.** The face is drawn into a square, so a picture that is not
//! square is squashed into one. The centre square is taken here instead.
//!
//! **Refusal.** A file nobody can decode is refused on the way in:
//! accepted, it would be drawn as an empty ring, because the drawing end
//! has no way to say what went wrong. Decoding here means everything in
//! the store can be drawn, and everything else was refused while a
//! person was still standing there.
//!
//! Reading is deliberately narrow — PNG and JPEG, by content.
//! Every tool that makes pictures writes one of the two, anything
//! else is one export away, and each additional decoder is surface
//! that runs on a file this application did not make.

use std::io::Cursor;

/// Largest side the store keeps. The biggest a face is drawn is 40
/// (`Metrics.detailsAvatar`), and a canvas rasterises at device
/// resolution (measured), so this carries a display packing 6.4 device
/// pixels into each logical one — and a doubled token to 3.2.
///
/// Smaller pictures are left at their own size: blowing a 64 pixel icon
/// up to this would store a blurred copy of it and nothing else.
pub const SIDE: u32 = 256;

/// Most pixels a picture may have before it is refused. This is the peak
/// worth holding for the moment it takes to shrink one — 32 megapixels
/// is 128MB of memory — and it cannot be expressed as a file size: a
/// flat PNG decodes to about a thousand times its own bytes.
pub const MAX_PIXELS: u64 = 32_000_000;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PictureError {
    /// Not a PNG or a JPEG, or one that will not come apart. The sentence
    /// names the two formats: every category word for the file is a
    /// second name for the avatar, and the settings card puts this line
    /// directly under the one word it uses
    /// (デザイン規約 §アバターを与える).
    #[error("this file is not a PNG or a JPEG that can be read")]
    Unreadable,
    /// The dimensions are named because the ceiling a person was told
    /// about is the file size — without them, a 2MB file being refused
    /// has no explanation at all.
    #[error("this file is {width}x{height}, past the {} megapixels this can take", MAX_PIXELS / 1_000_000)]
    TooManyPixels { width: u32, height: u32 },
    /// The file came apart but the small copy could not be written. Only a
    /// bug or an allocator that gave up reaches this, so the wording does
    /// not ask anybody to do anything.
    #[error("the avatar could not be stored")]
    Unstorable,
}

/// The bytes to store for a picture: PNG, square, at most [`SIDE`].
pub fn normalize(bytes: &[u8]) -> Result<Vec<u8>, PictureError> {
    let kind = kind_of(bytes).ok_or(PictureError::Unreadable)?;
    // Asked of the header alone, so a picture too big to hold is refused
    // without ever being held.
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

/// What a file is, by what is in it. The name it happens to carry is
/// the file dialog's business (`avatar::EXTENSIONS`) — a picture
/// saved under the wrong extension still draws.
fn kind_of(bytes: &[u8]) -> Option<Kind> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        Some(Kind::Png)
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(Kind::Jpeg)
    } else {
        None
    }
}

/// Width and height without decoding the picture: both readers stop at
/// the header when that is all that is asked of them.
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

/// Everything is normalised to eight-bit colour with an alpha channel on
/// the way out of the reader, so palettes, greyscale and sixteen-bit
/// pictures all arrive in one shape.
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
        // The transformations above expand a palette, so this is a
        // reader that did not do what it was asked.
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

/// Rewrites `stride`-byte pixels as four-byte ones.
fn widen(src: &[u8], stride: usize, to_rgba: impl Fn(&[u8]) -> [u8; 4]) -> Vec<u8> {
    src.chunks_exact(stride).flat_map(to_rgba).collect()
}

/// The centre square, averaged down to [`SIDE`] — a box filter, which is
/// what a reduction this large wants: every source pixel is counted
/// exactly once, so nothing shimmers when the result is drawn smaller
/// still.
///
/// A picture already smaller than [`SIDE`] is only
/// squared.
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
                // `count` is at least one: both spans are widened to a
                // minimum of a single pixel above.
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

/// PNG at the writer's ordinary setting. Squeezing harder buys a few
/// kilobytes of a file measured in tens, and spends them on the one
/// moment a person is waiting for the picture to appear.
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

/// A picture built here, so that nothing in this crate needs
/// a binary fixture in the tree to have a real one to work on.
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

    /// 24x16, three colours across, quality 92 — emitted once from a
    /// throwaway encoder, since a real JPEG is the only way to prove
    /// the JPEG half comes apart at all.
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
        // 36 megapixels of one colour: a small file, since that is the
        // shape of the problem — bytes do not bound pixels.
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
