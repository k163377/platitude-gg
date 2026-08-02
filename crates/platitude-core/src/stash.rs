//! Stash listing: `git stash list -z --format=...`.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::oid::Oid;
use crate::process::{GitCommand, GitExecutor};

/// `--format=` for `stash list`; fields: reflog selector, commit id,
/// committer time, reflog subject. Records are NUL-terminated via `-z`
/// (stash list forwards options to `git log`).
pub const STASH_FORMAT_ARG: &str = "--format=%gd%x00%H%x00%ct%x00%gs";

const STASH_FIELDS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashEntry {
    /// Reflog selector (`stash@{0}`), the argument for later stash ops.
    pub name: String,
    pub oid: Oid,
    /// Committer time (unix seconds).
    pub time: i64,
    /// Reflog subject (`WIP on main: ...` or the custom message).
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("malformed stash list output")]
pub struct StashParseError;

/// Parses `stash list` output produced with [`STASH_FORMAT_ARG`] + `-z`.
pub fn parse_stashes(bytes: &[u8]) -> Result<Vec<StashEntry>, StashParseError> {
    let tokens: Vec<&[u8]> = bytes.split(|b| *b == 0).collect();
    // Trailing empty token after the final NUL (or a single empty token
    // for empty output).
    let tokens = match tokens.split_last() {
        Some((last, rest)) if last.iter().all(|b| b.is_ascii_whitespace()) => rest,
        _ => &tokens[..],
    };
    if tokens.is_empty() {
        return Ok(Vec::new());
    }
    if tokens.len() % STASH_FIELDS != 0 {
        return Err(StashParseError);
    }
    let mut out = Vec::with_capacity(tokens.len() / STASH_FIELDS);
    for record in tokens.chunks_exact(STASH_FIELDS) {
        let name = String::from_utf8_lossy(record[0]).into_owned();
        let oid = Oid::from_hex(record[1]).map_err(|_| StashParseError)?;
        let time = std::str::from_utf8(record[2])
            .map_err(|_| StashParseError)?
            .trim()
            .parse()
            .map_err(|_| StashParseError)?;
        let message = String::from_utf8_lossy(record[3]).into_owned();
        out.push(StashEntry {
            name,
            oid,
            time,
            message,
        });
    }
    Ok(out)
}

/// Loads the stash list.
pub async fn load(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<Vec<StashEntry>, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["stash", "list", "-z", STASH_FORMAT_ARG]);
    let out = executor.run(cmd, cancel).await?;
    parse_stashes(&out.stdout).map_err(|e| GitError::UnexpectedOutput {
        command: "git stash list".to_string(),
        message: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn z(tokens: &[&str]) -> Vec<u8> {
        let mut v = Vec::new();
        for t in tokens {
            v.extend_from_slice(t.as_bytes());
            v.push(0);
        }
        v
    }

    #[test]
    fn parses_entries() {
        let bytes = z(&[
            "stash@{0}",
            SHA,
            "1700000000",
            "WIP on main: 1234567 subject",
            "stash@{1}",
            SHA,
            "1699999999",
            "On feature: custom message",
        ]);
        let stashes = parse_stashes(&bytes).unwrap();
        assert_eq!(stashes.len(), 2);
        assert_eq!(stashes[0].name, "stash@{0}");
        assert_eq!(stashes[0].time, 1_700_000_000);
        assert_eq!(stashes[1].message, "On feature: custom message");
    }

    #[test]
    fn empty_output_means_no_stashes() {
        assert!(parse_stashes(b"").unwrap().is_empty());
        assert!(parse_stashes(b"\n").unwrap().is_empty());
    }

    #[test]
    fn wrong_arity_is_an_error() {
        let bytes = z(&["stash@{0}", SHA]);
        assert!(parse_stashes(&bytes).is_err());
    }
}
