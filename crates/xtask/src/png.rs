//! A PNG written by hand, the way everything in this crate is: std only.
//!
//! Stored (uncompressed) deflate blocks, so the file is as big as its
//! pixels — nothing here needs to be small, it needs to decode. What it
//! is for is fixtures: the picture an avatar verb files, the pictures a
//! demo repository holds in every bucket the diff pane previews from.

/// An RGBA image of `width` × `height`, each pixel drawn by `pixel(x, y)`.
pub(crate) fn rgba(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    let stride = width as usize * 4 + 1;
    let mut raw = Vec::with_capacity(stride * height as usize);
    for y in 0..height {
        // Filter byte first: none.
        raw.push(0);
        for x in 0..width {
            raw.extend_from_slice(&pixel(x, y));
        }
    }
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    // 8 bits per sample, colour type 6 (RGBA), deflate, no filter method
    // but the one, no interlace.
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &zlib_stored(&raw));
    chunk(&mut out, b"IEND", &[]);
    out
}

/// One chunk: length, type, body, and the CRC of type and body.
fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(body);
    let crc = crc32(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

/// A zlib stream that compresses nothing: the header, the bytes in
/// stored blocks of the largest size a block may have, and the Adler
/// checksum the decoder checks them against.
fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    const BLOCK: usize = 65_535;
    let mut out = Vec::with_capacity(raw.len() + raw.len() / BLOCK * 5 + 11);
    out.extend_from_slice(&[0x78, 0x01]);
    let mut blocks = raw.chunks(BLOCK).peekable();
    // An empty image still has to carry one (final) block.
    if blocks.peek().is_none() {
        out.extend_from_slice(&[1, 0, 0, 0xff, 0xff]);
    }
    while let Some(block) = blocks.next() {
        let last = blocks.peek().is_none();
        out.push(u8::from(last));
        let len = block.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

fn adler32(bytes: &[u8]) -> u32 {
    // Sums are reduced every so many bytes rather than at each one:
    // 5552 is the most bytes the sums can take before either overflows.
    const NMAX: usize = 5552;
    let (mut a, mut b) = (1u32, 0u32);
    for run in bytes.chunks(NMAX) {
        for byte in run {
            a += u32::from(*byte);
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

fn crc32(bytes: &[u8]) -> u32 {
    let table = crc_table();
    let mut crc = 0xffff_ffffu32;
    for byte in bytes {
        crc = table[((crc ^ u32::from(*byte)) & 0xff) as usize] ^ (crc >> 8);
    }
    !crc
}

fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for (n, slot) in table.iter_mut().enumerate() {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 == 1 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *slot = c;
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference implementation's own answer for the string the
    /// PNG spec quotes, and the empty input.
    #[test]
    fn the_checksums_are_the_standard_ones() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(crc32(b""), 0);
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
        assert_eq!(adler32(b""), 1);
    }

    #[test]
    fn a_stored_stream_holds_the_bytes_whole_and_ends_on_the_last_block() {
        let raw: Vec<u8> = (0..70_000u32).map(|i| (i % 253) as u8).collect();
        let z = zlib_stored(&raw);
        assert_eq!(&z[..2], &[0x78, 0x01]);
        // First block: not final, 65535 bytes.
        assert_eq!(z[2], 0);
        assert_eq!(u16::from_le_bytes([z[3], z[4]]), 65_535);
        assert_eq!(&z[7..7 + 65_535], &raw[..65_535]);
        // Second block: final, the rest.
        let second = 7 + 65_535;
        assert_eq!(z[second], 1);
        assert_eq!(u16::from_le_bytes([z[second + 1], z[second + 2]]), 4_465);
        assert_eq!(&z[second + 5..second + 5 + 4_465], &raw[65_535..]);
        assert_eq!(z.len(), second + 5 + 4_465 + 4);
    }

    #[test]
    fn a_picture_carries_its_size_in_the_header_and_ends_properly() {
        let png = rgba(3, 2, |x, y| [x as u8, y as u8, 0, 255]);
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(u32::from_be_bytes([png[16], png[17], png[18], png[19]]), 3);
        assert_eq!(u32::from_be_bytes([png[20], png[21], png[22], png[23]]), 2);
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
        // Two rows of a filter byte and three RGBA pixels, stored.
        assert_eq!(png.len(), 8 + (12 + 13) + (12 + 2 + 5 + 2 * 13 + 4) + 12);
    }
}
