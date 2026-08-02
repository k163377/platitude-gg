//! `file://` URL → local path conversion for QML FolderDialog results.
//!
//! Kept dependency-free: only the shapes QML actually produces need to be
//! handled (`file:///C:/dir`, `file:///home/user/dir`, percent-encoded).

use std::path::PathBuf;

/// Converts a QML `url` string to a local filesystem path.
/// Non-`file:` inputs are returned as plain paths unchanged.
pub fn file_url_to_path(url: &str) -> PathBuf {
    let Some(rest) = url.strip_prefix("file://") else {
        return PathBuf::from(url);
    };
    // Strip an authority-less host part: `file:///C:/x` → `/C:/x`.
    let decoded = percent_decode(rest);
    #[cfg(windows)]
    {
        // `/C:/Users/...` → `C:/Users/...`; UNC (`//server/share`) keeps
        // its leading slashes.
        let trimmed = decoded
            .strip_prefix('/')
            .filter(|r| r.chars().nth(1) == Some(':'))
            .unwrap_or(&decoded);
        PathBuf::from(trimmed)
    }
    #[cfg(not(windows))]
    {
        PathBuf::from(decoded)
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let (Some(h), Some(l)) = (
                bytes.get(i + 1).copied().and_then(hex_val),
                bytes.get(i + 2).copied().and_then(hex_val),
            )
        {
            out.push((h << 4) | l);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn windows_drive_urls() {
        assert_eq!(
            file_url_to_path("file:///C:/Users/dev/repo"),
            PathBuf::from("C:/Users/dev/repo")
        );
        assert_eq!(
            file_url_to_path("file:///C:/with%20space/repo"),
            PathBuf::from("C:/with space/repo")
        );
    }

    #[test]
    fn unicode_percent_sequences_decode() {
        let p = file_url_to_path("file:///tmp/%E6%97%A5%E6%9C%AC%E8%AA%9E");
        assert!(p.to_string_lossy().ends_with("日本語"));
    }

    #[test]
    fn plain_paths_pass_through() {
        assert_eq!(
            file_url_to_path("C:/plain/path"),
            PathBuf::from("C:/plain/path")
        );
    }

    #[test]
    fn malformed_percent_is_kept_literal() {
        let p = file_url_to_path("file:///tmp/50%25off");
        assert!(p.to_string_lossy().ends_with("50%off"));
        let p2 = file_url_to_path("file:///tmp/broken%2");
        assert!(p2.to_string_lossy().ends_with("broken%2"));
    }
}
