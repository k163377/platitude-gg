//! Inflate (RFC 1951), all three block types: `write` emits only stored
//! blocks, but Qt saves a screenshot with dynamic Huffman codes.
//!
//! Symbols come out a bit at a time (Mark Adler's `puff` walk): this reads
//! one screenshot, once, and a decoding table is several times the code
//! for time nobody is waiting on.

use super::checksum::adler32;

const SHORT: &str = "the compressed data ends mid-stream";

/// The bytes a zlib stream (RFC 1950) holds. The Adler sum it ends
/// with is worked out again, so a stream that inflates to the wrong
/// bytes is caught here.
pub(super) fn zlib(stream: &[u8]) -> Result<Vec<u8>, String> {
    let [cmf, flg, deflated @ ..] = stream else {
        return Err("the compressed data is too short to hold a zlib header".to_string());
    };
    if cmf & 0x0f != 8 {
        return Err(format!("compression method {} is not deflate", cmf & 0x0f));
    }
    if (u16::from(*cmf) * 256 + u16::from(*flg)) % 31 != 0 {
        return Err("the zlib header fails its own check".to_string());
    }
    if flg & 0x20 != 0 {
        return Err(
            "the zlib stream wants a preset dictionary, which nothing here has".to_string(),
        );
    }
    let (out, read) = deflate(deflated)?;
    let sum = deflated
        .get(read..read + 4)
        .ok_or_else(|| "the zlib stream ends before its checksum".to_string())?;
    if u32::from_be_bytes([sum[0], sum[1], sum[2], sum[3]]) != adler32(&out) {
        return Err("the inflated bytes disagree with the checksum they came with".to_string());
    }
    Ok(out)
}

/// The blocks of a deflate stream, and how many of `stream`'s bytes
/// they took — the caller reads the Adler sum from there.
fn deflate(stream: &[u8]) -> Result<(Vec<u8>, usize), String> {
    let mut bits = Bits::new(stream);
    let mut out = Vec::new();
    loop {
        let last = bits.take(1)? == 1;
        match bits.take(2)? {
            0 => stored(&mut bits, &mut out)?,
            1 => {
                let (literals, distances) = fixed()?;
                coded(&mut bits, &mut out, &literals, &distances)?;
            }
            2 => {
                let (literals, distances) = described(&mut bits)?;
                coded(&mut bits, &mut out, &literals, &distances)?;
            }
            _ => return Err("deflate block type 3 is reserved".to_string()),
        }
        if last {
            return Ok((out, bits.byte_pos()));
        }
    }
}

/// A block that compresses nothing.
fn stored(bits: &mut Bits, out: &mut Vec<u8>) -> Result<(), String> {
    bits.align();
    let len = bits.u16le()?;
    let check = bits.u16le()?;
    if len != !check {
        return Err("a stored block's length disagrees with its own check".to_string());
    }
    out.extend_from_slice(bits.bytes(usize::from(len))?);
    Ok(())
}

/// A block whose bytes come out of two Huffman codes: literals and the
/// length of a repeat in one, how far back the repeat starts in the
/// other.
fn coded(
    bits: &mut Bits,
    out: &mut Vec<u8>,
    literals: &Code,
    distances: &Code,
) -> Result<(), String> {
    loop {
        let symbol = literals.read(bits)?;
        match symbol {
            0..=255 => out.push(symbol as u8),
            256 => return Ok(()),
            _ => {
                let length = span(bits, &LENGTHS, usize::from(symbol) - 257, "length")?;
                let far = usize::from(distances.read(bits)?);
                let back = span(bits, &DISTANCES, far, "distance")?;
                if back > out.len() {
                    return Err("a repeat reaches back past the start of the stream".to_string());
                }
                // Byte by byte, because a repeat may overlap what it
                // copies — which is how deflate writes a run of one byte.
                let from = out.len() - back;
                for i in 0..length {
                    out.push(out[from + i]);
                }
            }
        }
    }
}

/// `LENGTHS` or `DISTANCES`, read at `at`: the base, plus whatever the
/// extra bits that follow the symbol add to it.
fn span(bits: &mut Bits, table: &[(u16, u32)], at: usize, what: &str) -> Result<usize, String> {
    let (base, extra) = table
        .get(at)
        .ok_or_else(|| format!("{what} symbol {at} is not one deflate has"))?;
    Ok(usize::from(*base) + bits.take(*extra)? as usize)
}

/// The two codes a dynamic block describes before its data: their code
/// lengths, themselves written under a third code.
fn described(bits: &mut Bits) -> Result<(Code, Code), String> {
    let literal_count = bits.take(5)? as usize + 257;
    let distance_count = bits.take(5)? as usize + 1;
    let described_count = bits.take(4)? as usize + 4;
    // RFC 1951's order for the lengths of the code that carries the other
    // two.
    const ORDER: [usize; 19] = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut header = [0u8; 19];
    for at in ORDER.iter().take(described_count) {
        header[*at] = bits.take(3)? as u8;
    }
    let describing = Code::new(&header)?;
    let wanted = literal_count + distance_count;
    let mut lengths: Vec<u8> = Vec::with_capacity(wanted);
    while lengths.len() < wanted {
        let symbol = describing.read(bits)?;
        let (repeat, length) = match symbol {
            0..=15 => (1, symbol as u8),
            16 => (
                3 + bits.take(2)?,
                *lengths
                    .last()
                    .ok_or_else(|| "a run repeats a length before there is one".to_string())?,
            ),
            17 => (3 + bits.take(3)?, 0),
            18 => (11 + bits.take(7)?, 0),
            _ => {
                return Err(format!(
                    "code length symbol {symbol} is not one deflate has"
                ));
            }
        };
        for _ in 0..repeat {
            lengths.push(length);
        }
    }
    if lengths.len() > wanted {
        return Err("the code lengths run past the codes they describe".to_string());
    }
    let (literals, distances) = lengths.split_at(literal_count);
    Ok((Code::new(literals)?, Code::new(distances)?))
}

/// The code deflate fixes for the blocks that describe none of their
/// own.
fn fixed() -> Result<(Code, Code), String> {
    let mut literals = [8u8; 288];
    literals[144..256].fill(9);
    literals[256..280].fill(7);
    Ok((Code::new(&literals)?, Code::new(&[5u8; 30])?))
}

/// A canonical Huffman code: how many codes each bit length holds, and
/// its symbols in the order those codes run.
struct Code {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl Code {
    fn new(lengths: &[u8]) -> Result<Self, String> {
        let mut counts = [0u16; 16];
        for length in lengths {
            let at = usize::from(*length);
            if at > 15 {
                return Err(format!("a code of {at} bits is longer than deflate allows"));
            }
            counts[at] += 1;
        }
        counts[0] = 0;
        // Kraft: a code cannot promise more codes than its bits hold. An
        // incomplete one is allowed — a block with a single distance
        // writes one — so only the over-subscribed side is refused.
        let mut room = 1i32;
        for held in counts.iter().skip(1) {
            room = (room << 1) - i32::from(*held);
            if room < 0 {
                return Err("the code claims more symbols than its bit lengths hold".to_string());
            }
        }
        // Where the symbols of each length begin, laid out shortest
        // code first.
        let mut start = [0u16; 16];
        for length in 1..15 {
            start[length + 1] = start[length] + counts[length];
        }
        let mut symbols = vec![0u16; lengths.iter().filter(|length| **length > 0).count()];
        for (symbol, length) in lengths.iter().enumerate() {
            if *length > 0 {
                let at = usize::from(start[usize::from(*length)]);
                symbols[at] = symbol as u16;
                start[usize::from(*length)] += 1;
            }
        }
        Ok(Self { counts, symbols })
    }

    /// The next symbol, a bit at a time: the code read so far is
    /// measured against the codes of that length before another bit is
    /// taken.
    fn read(&self, bits: &mut Bits) -> Result<u16, String> {
        let (mut code, mut first, mut index) = (0u32, 0u32, 0u32);
        for length in 1..16 {
            code |= bits.take(1)?;
            let held = u32::from(self.counts[length]);
            if code < first + held {
                let at = (index + code - first) as usize;
                return self
                    .symbols
                    .get(at)
                    .copied()
                    .ok_or_else(|| "the stream names a symbol its code lacks".to_string());
            }
            index += held;
            first = (first + held) << 1;
            code <<= 1;
        }
        Err("a symbol runs past the longest code deflate allows".to_string())
    }
}

/// The stream read the way deflate writes it: bits out of each byte
/// from the least significant end up.
struct Bits<'a> {
    stream: &'a [u8],
    pos: usize,
    hold: u64,
    count: u32,
}

impl<'a> Bits<'a> {
    fn new(stream: &'a [u8]) -> Self {
        Self {
            stream,
            pos: 0,
            hold: 0,
            count: 0,
        }
    }

    /// The next `wanted` bits, low bit first. Never more than 16, which
    /// is what keeps the holding register inside a u64.
    fn take(&mut self, wanted: u32) -> Result<u32, String> {
        while self.count < wanted {
            let byte = *self.stream.get(self.pos).ok_or_else(|| SHORT.to_string())?;
            self.pos += 1;
            self.hold |= u64::from(byte) << self.count;
            self.count += 8;
        }
        let taken = (self.hold & ((1 << wanted) - 1)) as u32;
        self.hold >>= wanted;
        self.count -= wanted;
        Ok(taken)
    }

    /// Drop to the next byte boundary. Whole bytes already held go back
    /// to the stream, so what follows is read from the slice itself.
    fn align(&mut self) {
        self.pos -= (self.count / 8) as usize;
        self.hold = 0;
        self.count = 0;
    }

    /// Where the next byte is, once the bits stop.
    fn byte_pos(&mut self) -> usize {
        self.align();
        self.pos
    }

    /// The next `len` bytes, from a stream sitting on a boundary.
    fn bytes(&mut self, len: usize) -> Result<&'a [u8], String> {
        let taken = self
            .stream
            .get(self.pos..self.pos + len)
            .ok_or_else(|| SHORT.to_string())?;
        self.pos += len;
        Ok(taken)
    }

    fn u16le(&mut self) -> Result<u16, String> {
        let pair = self.bytes(2)?;
        Ok(u16::from_le_bytes([pair[0], pair[1]]))
    }
}

/// How long a repeat is: the base for each length symbol, and how many
/// extra bits follow it.
const LENGTHS: [(u16, u32); 29] = [
    (3, 0),
    (4, 0),
    (5, 0),
    (6, 0),
    (7, 0),
    (8, 0),
    (9, 0),
    (10, 0),
    (11, 1),
    (13, 1),
    (15, 1),
    (17, 1),
    (19, 2),
    (23, 2),
    (27, 2),
    (31, 2),
    (35, 3),
    (43, 3),
    (51, 3),
    (59, 3),
    (67, 4),
    (83, 4),
    (99, 4),
    (115, 4),
    (131, 5),
    (163, 5),
    (195, 5),
    (227, 5),
    (258, 0),
];

/// How far back it starts, the same way.
const DISTANCES: [(u16, u32); 30] = [
    (1, 0),
    (2, 0),
    (3, 0),
    (4, 0),
    (5, 1),
    (7, 1),
    (9, 2),
    (13, 2),
    (17, 3),
    (25, 3),
    (33, 4),
    (49, 4),
    (65, 5),
    (97, 5),
    (129, 6),
    (193, 6),
    (257, 7),
    (385, 7),
    (513, 8),
    (769, 8),
    (1025, 9),
    (1537, 9),
    (2049, 10),
    (3073, 10),
    (4097, 11),
    (6145, 11),
    (8193, 12),
    (12289, 12),
    (16385, 13),
    (24577, 13),
];

#[cfg(test)]
mod tests {
    use super::super::write::zlib_stored;
    use super::*;

    /// Bits laid out the way deflate writes them: header fields low bit
    /// first, Huffman codes high bit first.
    #[derive(Default)]
    struct Pack {
        out: Vec<u8>,
        hold: u32,
        count: u32,
    }

    impl Pack {
        fn low(&mut self, value: u32, bits: u32) {
            for at in 0..bits {
                self.bit((value >> at) & 1);
            }
        }

        fn high(&mut self, code: u32, bits: u32) {
            for at in (0..bits).rev() {
                self.bit((code >> at) & 1);
            }
        }

        fn bit(&mut self, bit: u32) {
            self.hold |= bit << self.count;
            self.count += 1;
            if self.count == 8 {
                self.out.push(self.hold as u8);
                self.hold = 0;
                self.count = 0;
            }
        }

        /// The stream under a zlib wrapper, with the sum of what it is
        /// supposed to inflate to.
        fn wrapped(mut self, into: &[u8]) -> Vec<u8> {
            if self.count > 0 {
                self.out.push(self.hold as u8);
            }
            let mut stream = vec![0x78, 0x01];
            stream.extend_from_slice(&self.out);
            stream.extend_from_slice(&adler32(into).to_be_bytes());
            stream
        }
    }

    #[test]
    fn a_stored_stream_comes_back_as_the_bytes_that_went_in() {
        // Past one block's ceiling, so the walk crosses a block boundary.
        let raw: Vec<u8> = (0..70_000u32).map(|i| (i % 253) as u8).collect();
        assert_eq!(zlib(&zlib_stored(&raw)), Ok(raw));
    }

    /// The block type no file in the tree reaches: deflate's built-in
    /// code, with a repeat that overlaps what it is copying.
    #[test]
    fn a_fixed_block_reads_its_literals_and_its_repeats() {
        let mut pack = Pack::default();
        pack.low(1, 1);
        pack.low(1, 2);
        for byte in b"abc" {
            // Literals 0-143 are eight bits from 0x30 up.
            pack.high(0x30 + u32::from(*byte), 8);
        }
        // Length 3 (symbol 257, seven bits) three bytes back (symbol 2,
        // five bits): "abc" again.
        pack.high(1, 7);
        pack.high(2, 5);
        pack.high(0x30 + u32::from(b'x'), 8);
        // Length 3 one byte back — a run that reads the bytes it is
        // still writing.
        pack.high(1, 7);
        pack.high(0, 5);
        pack.high(0, 7);
        let stream = pack.wrapped(b"abcabcxxxx");
        assert_eq!(zlib(&stream).as_deref(), Ok(&b"abcabcxxxx"[..]));
    }

    #[test]
    fn a_stream_that_inflates_to_the_wrong_bytes_is_refused() {
        let mut stream = zlib_stored(b"the bytes as they were");
        let last = stream.len() - 1;
        stream[last] ^= 0xff;
        assert!(zlib(&stream).is_err_and(|why| why.contains("checksum")));
    }

    #[test]
    fn a_stream_cut_short_says_so_rather_than_reading_past_it() {
        let whole = zlib_stored(b"a picture that never arrived in full");
        for cut in [2, 4, whole.len() - 5, whole.len() - 1] {
            assert!(zlib(&whole[..cut]).is_err(), "{cut} bytes read as whole");
        }
    }

    #[test]
    fn a_code_promising_more_symbols_than_its_bits_hold_is_refused() {
        // Three one-bit codes: two is all a bit has room for.
        assert!(Code::new(&[1, 1, 1]).is_err());
        assert!(Code::new(&[1, 1]).is_ok());
        assert!(Code::new(&[16]).is_err());
    }
}
