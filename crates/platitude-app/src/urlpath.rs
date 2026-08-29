//! `file://` URL ↔ local path conversion for the QML FolderDialog: its
//! result comes back as a URL, and the folder it opens at is given as one.
//!
//! Kept dependency-free: only the shapes QML actually produces need to be
//! handled (`file:///C:/dir`, `file:///home/user/dir`, percent-encoded).

use std::path::{Path, PathBuf};

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
        // `/C:/Users/...` → `C:/Users/...`.
        if let Some(drive) = decoded
            .strip_prefix('/')
            .filter(|r| r.chars().nth(1) == Some(':'))
        {
            return PathBuf::from(drive);
        }
        // A rest that does not start at `/` names a host — `file://server/
        // share` is the URL form of a UNC path (what `path_to_file_url`
        // writes and the FolderDialog returns). Read as-is it would be a
        // relative path resolving against the working directory.
        if !decoded.is_empty() && !decoded.starts_with('/') {
            return PathBuf::from(format!("//{decoded}"));
        }
        PathBuf::from(decoded)
    }
    #[cfg(not(windows))]
    {
        PathBuf::from(decoded)
    }
}

/// The folder the picker opens at for a repository at `path`: the one it
/// sits in — or the repository itself when it sits at a root, where there
/// is nothing above to go up to and the root lists like any other folder.
pub fn picker_folder_url(path: &Path) -> String {
    path_to_file_url(path.parent().unwrap_or(path))
}

/// A `file:` URL for a file QML is to load — an assigned avatar.
pub fn file_url(path: &Path) -> String {
    path_to_file_url(path)
}

/// The last segment of a path, whichever separator wrote it — the name a
/// folder is shown by where the whole path would not be read (a question
/// naming another working copy, a tooltip naming its folder).
pub fn path_leaf(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Converts a local path to a `file:` URL for QML. Empty for a path with
/// no root: a dialog cannot be opened at a folder that is not one.
fn path_to_file_url(path: &Path) -> String {
    let encoded = percent_encode(&path.to_string_lossy().replace('\\', "/"));
    if encoded.starts_with("//") {
        // UNC: the leading pair names the host, which the URL keeps.
        format!("file:{encoded}")
    } else if encoded.starts_with('/') {
        format!("file://{encoded}")
    } else if encoded.as_bytes().get(1) == Some(&b':') {
        // A drive letter is not a root the URL can borrow, so it brings
        // its own slash: `C:/x` → `file:///C:/x`.
        format!("file:///{encoded}")
    } else {
        String::new()
    }
}

/// Percent-encodes a path for a `file:` URL, leaving the separators and
/// the drive colon to be read as themselves. `#` and `?` are the ones
/// that matter: unencoded, a folder named after either cuts the URL short.
fn percent_encode(input: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                out.push(byte as char);
            }
            _ => {
                out.push('%');
                out.push(HEX[(byte >> 4) as usize] as char);
                out.push(HEX[(byte & 0x0f) as usize] as char);
            }
        }
    }
    out
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
    fn the_leaf_is_the_last_segment_whichever_separator_wrote_it() {
        assert_eq!(path_leaf("C:\\Users\\dev\\repo"), "repo");
        assert_eq!(path_leaf("/home/dev/repo"), "repo");
        assert_eq!(path_leaf("C:/mixed\\separators/leaf"), "leaf");
        assert_eq!(path_leaf("bare"), "bare");
        assert_eq!(path_leaf(""), "");
    }

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
    fn paths_become_file_urls() {
        assert_eq!(
            path_to_file_url(Path::new("C:/Users/dev/repo")),
            "file:///C:/Users/dev/repo"
        );
        assert_eq!(
            path_to_file_url(Path::new(r"C:\Users\dev\repo")),
            "file:///C:/Users/dev/repo"
        );
        assert_eq!(
            path_to_file_url(Path::new("/home/dev/repo")),
            "file:///home/dev/repo"
        );
        assert_eq!(
            path_to_file_url(Path::new(r"\\server\share\repo")),
            "file://server/share/repo"
        );
    }

    #[test]
    fn a_repository_opens_the_folder_it_sits_in() {
        assert_eq!(
            picker_folder_url(Path::new("C:/Users/dev/repo")),
            "file:///C:/Users/dev"
        );
        assert_eq!(
            picker_folder_url(Path::new("/home/dev/repo")),
            "file:///home/dev"
        );
    }

    // What counts as a root is the platform's own reading of the path, so
    // the drive and UNC shapes are only roots where they mean anything.
    #[test]
    fn a_repository_at_a_root_opens_the_root() {
        // Nothing is above a root, so the picker stays there rather than
        // falling back on wherever the dialog happens to have been left.
        assert_eq!(picker_folder_url(Path::new("/")), "file:///");
        // A repository one step in still has the root to go up to.
        assert_eq!(picker_folder_url(Path::new("/repo")), "file:///");
    }

    #[test]
    #[cfg(windows)]
    fn a_repository_at_a_drive_root_opens_the_drive() {
        assert_eq!(picker_folder_url(Path::new("C:/")), "file:///C:/");
        assert_eq!(picker_folder_url(Path::new(r"C:\")), "file:///C:/");
        assert_eq!(picker_folder_url(Path::new("C:/repo")), "file:///C:/");
        // A share is a root of its own: `\\server` is not a folder. Handed
        // back by `parent()` that root carries its separator and the URL
        // keeps it, which is what a folder URL looks like anyway
        // (`file:///C:/`); given as the repository itself it has none.
        assert_eq!(
            picker_folder_url(Path::new(r"\\server\share")),
            "file://server/share"
        );
        assert_eq!(
            picker_folder_url(Path::new(r"\\server\share\repo")),
            "file://server/share/"
        );
    }

    #[test]
    fn a_path_with_no_root_has_no_folder_to_open() {
        assert_eq!(picker_folder_url(Path::new("")), "");
        assert_eq!(picker_folder_url(Path::new("repo")), "");
    }

    #[test]
    fn rootless_paths_have_no_url() {
        assert_eq!(path_to_file_url(Path::new("repo/sub")), "");
        assert_eq!(path_to_file_url(Path::new("")), "");
    }

    #[test]
    fn url_form_survives_the_round_trip() {
        for path in [
            "/home/dev/with space/repo",
            "/home/dev/日本語/repo",
            "/home/dev/50%off/repo",
            "/home/dev/a#b?c/repo",
        ] {
            let url = path_to_file_url(Path::new(path));
            assert_eq!(file_url_to_path(&url), PathBuf::from(path), "{url}");
        }
    }

    #[test]
    #[cfg(windows)]
    fn unc_urls_survive_the_round_trip() {
        let url = path_to_file_url(Path::new(r"\\server\share\repo"));
        assert_eq!(url, "file://server/share/repo");
        assert_eq!(
            file_url_to_path(&url),
            PathBuf::from("//server/share/repo"),
            "the host segment stays a host, not the head of a relative path"
        );
    }

    #[test]
    #[cfg(windows)]
    fn drive_urls_survive_the_round_trip() {
        let path = Path::new(r"C:\Users\dev\with space\日本語");
        let url = path_to_file_url(path);
        assert_eq!(
            url,
            "file:///C:/Users/dev/with%20space/%E6%97%A5%E6%9C%AC%E8%AA%9E"
        );
        assert_eq!(
            file_url_to_path(&url),
            PathBuf::from("C:/Users/dev/with space/日本語")
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
