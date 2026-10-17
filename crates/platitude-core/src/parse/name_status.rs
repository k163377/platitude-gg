//! Parser for `--name-status -z` output (diff-tree / diff).
//!
//! Token stream: `<status>\0<path>\0` per entry, where R/C statuses carry a
//! score suffix and are followed by **two** paths (source then dest).

/// One changed file of a commit or diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    /// Raw status letter: `A M D T R C U X B`.
    pub status: char,
    /// Similarity score for renames/copies (0-100).
    pub score: Option<u8>,
    /// Path (destination path for renames/copies).
    pub path: String,
    /// Source path for renames/copies.
    pub orig_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("malformed --name-status -z output near `{0}`")]
pub struct NameStatusParseError(pub String);

pub fn parse_name_status(bytes: &[u8]) -> Result<Vec<FileChange>, NameStatusParseError> {
    let mut out = Vec::new();
    let mut tokens = bytes
        .split(|b| *b == 0)
        .filter(|t| !t.is_empty())
        .map(String::from_utf8_lossy);

    while let Some(status_tok) = tokens.next() {
        let mut chars = status_tok.chars();
        let status = chars
            .next()
            .ok_or_else(|| NameStatusParseError(status_tok.to_string()))?;
        if !status.is_ascii_uppercase() {
            return Err(NameStatusParseError(status_tok.to_string()));
        }
        let score: Option<u8> = {
            let digits = chars.as_str();
            if digits.is_empty() {
                None
            } else {
                Some(
                    digits
                        .parse()
                        .map_err(|_| NameStatusParseError(status_tok.to_string()))?,
                )
            }
        };
        let first = tokens
            .next()
            .ok_or_else(|| NameStatusParseError(status_tok.to_string()))?;
        let (path, orig_path) = if matches!(status, 'R' | 'C') {
            let dest = tokens
                .next()
                .ok_or_else(|| NameStatusParseError(status_tok.to_string()))?;
            (dest.into_owned(), Some(first.into_owned()))
        } else {
            (first.into_owned(), None)
        };
        out.push(FileChange {
            status,
            score,
            path,
            orig_path,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn z(tokens: &[&str]) -> Vec<u8> {
        let mut v = Vec::new();
        for t in tokens {
            v.extend_from_slice(t.as_bytes());
            v.push(0);
        }
        v
    }

    #[test]
    fn parses_plain_and_rename_entries() {
        let bytes = z(&[
            "M",
            "modified.txt",
            "A",
            "added file.txt",
            "R100",
            "old/path.txt",
            "new/path.txt",
            "D",
            "deleted.txt",
            "C075",
            "src.txt",
            "copy.txt",
        ]);
        let changes = parse_name_status(&bytes).unwrap();
        assert_eq!(changes.len(), 5);
        assert_eq!(changes[0].status, 'M');
        assert_eq!(changes[1].path, "added file.txt");
        let rename = &changes[2];
        assert_eq!(rename.status, 'R');
        assert_eq!(rename.score, Some(100));
        assert_eq!(rename.path, "new/path.txt");
        assert_eq!(rename.orig_path.as_deref(), Some("old/path.txt"));
        let copy = &changes[4];
        assert_eq!(copy.status, 'C');
        assert_eq!(copy.score, Some(75));
        assert_eq!(copy.path, "copy.txt");
        assert_eq!(copy.orig_path.as_deref(), Some("src.txt"));
    }

    #[test]
    fn empty_output_is_no_changes() {
        assert!(parse_name_status(b"").unwrap().is_empty());
    }

    #[test]
    fn truncated_rename_is_an_error() {
        let bytes = z(&["R100", "only-one-path"]);
        assert!(parse_name_status(&bytes).is_err());
    }

    #[test]
    fn garbage_status_is_an_error() {
        let bytes = z(&["zz", "path"]);
        assert!(parse_name_status(&bytes).is_err());
    }

    #[test]
    fn empty_tokens_do_not_shift_the_status_and_paths() {
        let bytes = z(&["", "R100", "", "old", "", "new", "", "M", "file", ""]);
        assert_eq!(
            parse_name_status(&bytes),
            parse_name_status(&z(&["R100", "old", "new", "M", "file"])),
        );
    }

    #[test]
    fn invalid_utf8_is_lossy_in_both_paths_and_errors() {
        let changes = parse_name_status(b"R100\0old\xff\0new\xfe\0").unwrap();
        assert_eq!(changes[0].orig_path.as_deref(), Some("old\u{fffd}"));
        assert_eq!(changes[0].path, "new\u{fffd}");
        assert_eq!(
            parse_name_status(b"R\xff\0old\0new\0"),
            Err(NameStatusParseError("R\u{fffd}".to_string())),
        );
    }

    #[test]
    fn invalid_scores_report_the_whole_status_token() {
        for status in ["R256", "R-1", "Rno"] {
            assert_eq!(
                parse_name_status(&z(&[status, "old", "new"])),
                Err(NameStatusParseError(status.to_string())),
            );
        }
    }
}
