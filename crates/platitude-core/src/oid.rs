//! Compact object id storage (SHA-1 and SHA-256 repositories).

use std::fmt;

/// Error returned when a string is not a valid full object id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid object id")]
pub struct OidParseError;

/// A full git object id, stored as raw bytes.
///
/// Holds either 20 bytes (SHA-1) or 32 bytes (SHA-256) so commit metadata for
/// very large repositories stays compact (no heap allocation per id).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Oid {
    len: u8,
    bytes: [u8; 32],
}

impl Oid {
    /// Parses a full 40- or 64-character hex object id.
    pub fn from_hex(hex: &[u8]) -> Result<Self, OidParseError> {
        if hex.len() != 40 && hex.len() != 64 {
            return Err(OidParseError);
        }
        let mut bytes = [0u8; 32];
        for (i, pair) in hex.chunks_exact(2).enumerate() {
            bytes[i] = (hex_val(pair[0])? << 4) | hex_val(pair[1])?;
        }
        Ok(Self {
            len: (hex.len() / 2) as u8,
            bytes,
        })
    }

    /// Parses from a `&str` (convenience over [`Oid::from_hex`]).
    pub fn from_hex_str(hex: &str) -> Result<Self, OidParseError> {
        Self::from_hex(hex.as_bytes())
    }

    /// Raw bytes of the id.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }

    /// Lowercase hex representation (40 or 64 chars).
    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(self.len as usize * 2);
        for b in self.as_bytes() {
            push_hex(&mut s, *b);
        }
        s
    }

    /// Lowercase hex prefix of `n` characters (clamped to the full length).
    pub fn short_hex(&self, n: usize) -> String {
        let mut s = self.to_hex();
        s.truncate(n);
        s
    }
}

fn hex_val(c: u8) -> Result<u8, OidParseError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(OidParseError),
    }
}

fn push_hex(s: &mut String, b: u8) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    s.push(HEX[(b >> 4) as usize] as char);
    s.push(HEX[(b & 0xf) as usize] as char);
}

impl fmt::Display for Oid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl fmt::Debug for Oid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Oid({})", self.short_hex(8))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_sha1() {
        let hex = "0123456789abcdef0123456789abcdef01234567";
        let oid = Oid::from_hex_str(hex).unwrap();
        assert_eq!(oid.to_hex(), hex);
        assert_eq!(oid.as_bytes().len(), 20);
    }

    #[test]
    fn roundtrip_sha256() {
        let hex = "a".repeat(64);
        let oid = Oid::from_hex_str(&hex).unwrap();
        assert_eq!(oid.to_hex(), hex);
        assert_eq!(oid.as_bytes().len(), 32);
    }

    #[test]
    fn uppercase_normalizes_to_lowercase() {
        let oid = Oid::from_hex_str(&"AB".repeat(20)).unwrap();
        assert_eq!(oid.to_hex(), "ab".repeat(20));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(Oid::from_hex_str("").is_err());
        assert!(Oid::from_hex_str(&"a".repeat(39)).is_err());
        assert!(Oid::from_hex_str(&"a".repeat(41)).is_err());
        assert!(Oid::from_hex_str(&"g".repeat(40)).is_err());
    }

    #[test]
    fn short_hex_clamps() {
        let oid = Oid::from_hex_str(&"ab".repeat(20)).unwrap();
        assert_eq!(oid.short_hex(8), "abababab");
        assert_eq!(oid.short_hex(999).len(), 40);
    }

    #[test]
    fn distinct_lengths_are_not_equal() {
        let sha1 = Oid::from_hex_str(&"0".repeat(40)).unwrap();
        let sha256 = Oid::from_hex_str(&"0".repeat(64)).unwrap();
        assert_ne!(sha1, sha256);
    }
}
