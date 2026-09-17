//! A digest both machines compute the same way.
//!
//! Evidence taken on two sides of a mount is only evidence if the two
//! sides can be compared (`gate::evidence`): equal byte counts and two
//! digests of different kinds cannot tell "the same file" from "a
//! different file of the same length", which is the question the
//! evidence is there to answer. The container's side is `sha256sum`,
//! which every coreutils has; this is the host's side of the same
//! answer.
//!
//! In here, because a dependency is a human's decision (CLAUDE.md
//! 絶対制約) and the task runner's are std only. It is a fingerprint
//! a second machine can also produce, pinned to the published
//! vectors below — a chosen collision is outside what it
//! answers.

/// FIPS 180-4 §4.2.2. Laid out as the standard prints it — eight rows
/// of eight — so a reader can check it against the table it came from.
#[rustfmt::skip]
const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// FIPS 180-4 §5.3.3.
#[rustfmt::skip]
const H0: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

/// The SHA-256 of these bytes, lowercase hex — the same 64 characters
/// `sha256sum` prints for the same bytes.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = H0;
    // The message, padded: 0x80, zeros to 56 mod 64, then the length in
    // bits, big-endian (FIPS 180-4 §5.1.1). Built a block at a time so
    // nothing here holds a second copy of the file.
    let blocks = bytes.chunks(64);
    let mut tail = Vec::with_capacity(128);
    for block in blocks {
        if block.len() == 64 {
            compress(&mut h, block);
        } else {
            tail.extend_from_slice(block);
        }
    }
    tail.push(0x80);
    while tail.len() % 64 != 56 {
        tail.push(0);
    }
    let bits = (bytes.len() as u64).wrapping_mul(8);
    tail.extend_from_slice(&bits.to_be_bytes());
    for block in tail.chunks(64) {
        compress(&mut h, block);
    }
    h.iter().map(|word| format!("{word:08x}")).collect()
}

/// One 64-byte block into the state (FIPS 180-4 §6.2.2). The caller
/// hands whole blocks: a short slice would read as zeros, which is a
/// different message.
fn compress(h: &mut [u32; 8], block: &[u8]) {
    let mut w = [0u32; 64];
    for (at, word) in w.iter_mut().take(16).enumerate() {
        let four = block.get(at * 4..at * 4 + 4).unwrap_or(&[0; 4]);
        *word = u32::from_be_bytes([four[0], four[1], four[2], four[3]]);
    }
    for at in 16..64 {
        let s0 = w[at - 15].rotate_right(7) ^ w[at - 15].rotate_right(18) ^ (w[at - 15] >> 3);
        let s1 = w[at - 2].rotate_right(17) ^ w[at - 2].rotate_right(19) ^ (w[at - 2] >> 10);
        w[at] = w[at - 16]
            .wrapping_add(s0)
            .wrapping_add(w[at - 7])
            .wrapping_add(s1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = *h;
    for at in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let choose = (e & f) ^ ((!e) & g);
        let one = hh
            .wrapping_add(s1)
            .wrapping_add(choose)
            .wrapping_add(K[at])
            .wrapping_add(w[at]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let most = (a & b) ^ (a & c) ^ (b & c);
        let two = s0.wrapping_add(most);
        hh = g;
        g = f;
        f = e;
        e = d.wrapping_add(one);
        d = c;
        c = b;
        b = a;
        a = one.wrapping_add(two);
    }
    for (state, add) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
        *state = state.wrapping_add(add);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The published vectors (FIPS 180-4 / NIST CAVP): the empty
    /// message, one short of a block, one that pads into a second block,
    /// and one long enough to run many.
    #[test]
    fn the_published_vectors_come_out() {
        for (message, digest) in [
            (
                "",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                "abc",
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
                "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            ),
            (
                "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmn\
                 hijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu",
                "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1",
            ),
        ] {
            assert_eq!(sha256_hex(message.as_bytes()), digest, "{message:?}");
        }
    }

    /// A million bytes, which is the vector that says the block loop
    /// carries state across as many blocks as a lock file has.
    #[test]
    fn the_long_vector_comes_out() {
        assert_eq!(
            sha256_hex(&b"a".repeat(1_000_000)),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    /// A length that lands exactly on the boundary where the padding
    /// needs a whole block of its own.
    #[test]
    fn a_message_that_fills_its_last_block_pads_into_another() {
        assert_eq!(
            sha256_hex(&b"a".repeat(56)),
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a"
        );
        assert_eq!(
            sha256_hex(&b"a".repeat(64)),
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb"
        );
    }
}
