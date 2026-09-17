//! The two sums a PNG carries, written once for both directions: the
//! writer puts them in, and the reader is only sure it read the file it
//! was handed because it works them out again.

/// The Adler sum a zlib stream ends with, over the bytes it holds.
pub(super) fn adler32(bytes: &[u8]) -> u32 {
    // Sums are reduced every so many bytes:
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

/// The CRC every chunk ends with, over its type and its body.
pub(super) fn crc32(bytes: &[u8]) -> u32 {
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
}
