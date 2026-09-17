//! A PNG taken apart: the ones Qt saves a window as, and the ones
//! `write` puts out.
//!
//! Eight bits a sample, no interlace, colour type 2
//! or 6 — that is `QImage::save` for a screenshot
//! (RGB) and `write` for a fixture (RGBA), and
//! nothing here is ever pointed at anything else.
//! Every other shape is refused by name.
//!
//! Both sums the file carries are worked out again. A
//! crop is read to settle a question about single pixels,
//! so a file that arrived damaged has to say so.

use super::{checksum::crc32, inflate};

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// A decoded picture: four bytes a pixel, row after row.
pub(crate) struct Image {
    pub(crate) width: u32,
    pub(crate) height: u32,
    rgba: Vec<u8>,
}

impl Image {
    /// The pixel at `x`, `y`, both of which the caller has already kept
    /// inside the picture.
    pub(crate) fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let at = (y as usize * self.width as usize + x as usize) * 4;
        [
            self.rgba[at],
            self.rgba[at + 1],
            self.rgba[at + 2],
            self.rgba[at + 3],
        ]
    }
}

pub(crate) fn decode(file: &[u8]) -> Result<Image, String> {
    if file.get(..8) != Some(&SIGNATURE[..]) {
        return Err("not a png: the file does not open with the signature".to_string());
    }
    let mut header: Option<Header> = None;
    let mut compressed = Vec::new();
    let mut at = 8;
    while at < file.len() {
        let length = word(file, at)? as usize;
        let kind = file
            .get(at + 4..at + 8)
            .ok_or_else(|| "a chunk ends before its type".to_string())?;
        let named = String::from_utf8_lossy(kind).into_owned();
        let body = file
            .get(at + 8..at + 8 + length)
            .ok_or_else(|| format!("the {named} chunk ends before the {length} bytes it claims"))?;
        if word(file, at + 8 + length)? != crc32(&file[at + 4..at + 8 + length]) {
            return Err(format!("the {named} chunk fails its own checksum"));
        }
        match kind {
            b"IHDR" => header = Some(Header::read(body)?),
            // One stream across as many chunks as the writer felt like:
            // Qt's runs to a dozen, and a block may straddle any two.
            b"IDAT" => compressed.extend_from_slice(body),
            b"IEND" => break,
            _ => {}
        }
        at += 12 + length;
    }
    let header = header.ok_or_else(|| "the file carries no header chunk".to_string())?;
    let (width, height) = (header.width as usize, header.height as usize);
    let (stride, wanted) = scanlines(width, height, header.channels)
        .ok_or_else(|| format!("{width}x{height} is more picture than a machine holds"))?;
    let mut raw = inflate::zlib(&compressed)?;
    if raw.len() != wanted {
        return Err(format!(
            "the picture holds {} bytes where {width}x{height} needs {wanted}",
            raw.len()
        ));
    }
    unfilter(&mut raw, stride, height, header.channels)?;
    Ok(Image {
        width: header.width,
        height: header.height,
        rgba: expanded(&raw, stride, height, header.channels),
    })
}

/// What one scanline takes, and what all of them take with their filter
/// bytes — or nothing at all, because both sizes come out of the file
/// and a header naming billions square would otherwise wrap around into
/// a length that looks reasonable.
fn scanlines(width: usize, height: usize, channels: usize) -> Option<(usize, usize)> {
    let stride = width.checked_mul(channels)?;
    Some((stride, stride.checked_add(1)?.checked_mul(height)?))
}

/// What the header chunk says, once the shapes this cannot read are out.
struct Header {
    width: u32,
    height: u32,
    /// Bytes a pixel: three for colour type 2, four for type 6. Also
    /// how far back a filter looks for the pixel to its left.
    channels: usize,
}

impl Header {
    fn read(body: &[u8]) -> Result<Self, String> {
        // Two sizes and then five bytes of shape, in a chunk that is
        // thirteen bytes or is not this chunk.
        let shape: [u8; 5] = body
            .get(8..13)
            .filter(|_| body.len() == 13)
            .and_then(|shape| shape.try_into().ok())
            .ok_or_else(|| format!("the header chunk is {} bytes where png says 13", body.len()))?;
        let [depth, colour, compression, filter, interlace] = shape;
        let (width, height) = (word(body, 0)?, word(body, 4)?);
        if width == 0 || height == 0 {
            return Err(format!("the picture is {width}x{height}"));
        }
        if depth != 8 {
            return Err(format!(
                "{depth} bits a sample: this reads the 8 a screenshot is saved with"
            ));
        }
        let channels = match colour {
            2 => 3,
            6 => 4,
            other => {
                return Err(format!(
                    "colour type {other}: this reads 2 (rgb) and 6 (rgba), \
                     which is what a screenshot and a fixture are saved as"
                ));
            }
        };
        if compression != 0 || filter != 0 {
            return Err("the file names a compression or filter method png does not".to_string());
        }
        if interlace != 0 {
            return Err("the picture is interlaced, and nothing here writes one".to_string());
        }
        Ok(Self {
            width,
            height,
            channels,
        })
    }
}

/// The scanlines, each undone from the filter its first byte names. In
/// place: a filter reads the row above it as it stands after its own
/// filter came off, which is the row this already passed.
fn unfilter(raw: &mut [u8], stride: usize, height: usize, channels: usize) -> Result<(), String> {
    let nothing = vec![0u8; stride];
    for y in 0..height {
        let start = y * (stride + 1);
        let filter = raw[start];
        let (before, rest) = raw.split_at_mut(start + 1);
        let row = &mut rest[..stride];
        let above = if y == 0 {
            &nothing[..]
        } else {
            &before[start - stride..start]
        };
        match filter {
            0 => {}
            1 => {
                for i in channels..stride {
                    row[i] = row[i].wrapping_add(row[i - channels]);
                }
            }
            2 => {
                for (byte, up) in row.iter_mut().zip(above) {
                    *byte = byte.wrapping_add(*up);
                }
            }
            3 => {
                for i in 0..stride {
                    let left = if i >= channels {
                        u16::from(row[i - channels])
                    } else {
                        0
                    };
                    row[i] = row[i].wrapping_add(((left + u16::from(above[i])) / 2) as u8);
                }
            }
            4 => {
                for i in 0..stride {
                    let (left, corner) = if i >= channels {
                        (row[i - channels], above[i - channels])
                    } else {
                        (0, 0)
                    };
                    row[i] = row[i].wrapping_add(paeth(left, above[i], corner));
                }
            }
            other => {
                return Err(format!(
                    "scanline {y} names filter {other}, which png has not"
                ));
            }
        }
    }
    Ok(())
}

/// Of the three neighbours, the one the prediction lands nearest.
fn paeth(left: u8, above: u8, corner: u8) -> u8 {
    let guess = i16::from(left) + i16::from(above) - i16::from(corner);
    let (to_left, to_above, to_corner) = (
        (guess - i16::from(left)).abs(),
        (guess - i16::from(above)).abs(),
        (guess - i16::from(corner)).abs(),
    );
    if to_left <= to_above && to_left <= to_corner {
        left
    } else if to_above <= to_corner {
        above
    } else {
        corner
    }
}

/// The rows with their filter bytes dropped, and an opaque alpha put on
/// the pixels of a picture that carries none. Named apart from the
/// `png::rgba` that writes a file: this one only widens what was read.
fn expanded(raw: &[u8], stride: usize, height: usize, channels: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(stride / channels * 4 * height);
    for y in 0..height {
        let row = &raw[y * (stride + 1) + 1..][..stride];
        for pixel in row.chunks_exact(channels) {
            out.extend_from_slice(&[pixel[0], pixel[1], pixel[2]]);
            out.push(if channels == 4 { pixel[3] } else { 255 });
        }
    }
    out
}

/// The four bytes at `at`, as the big-endian number png writes.
fn word(file: &[u8], at: usize) -> Result<u32, String> {
    match file.get(at..at + 4) {
        Some([w, x, y, z]) => Ok(u32::from_be_bytes([*w, *x, *y, *z])),
        _ => Err("the file ends where a chunk's length or checksum should be".to_string()),
    }
}

#[cfg(test)]
mod tests;
