//! Detection of the installed git version and the supported-minimum gate.

use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// Minimum supported git version (source of truth: 要望.md — Ubuntu 24.04 LTS
/// ships this; modern features like `rebase --update-refs` are assumed).
pub const MINIMUM_GIT: (u32, u32) = (2, 43);

/// [`MINIMUM_GIT`] as the "2.43" the UI and error messages print.
pub fn minimum_string() -> String {
    format!("{}.{}", MINIMUM_GIT.0, MINIMUM_GIT.1)
}

/// Parsed `git --version` output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    /// The full version string as printed by git (for display).
    pub raw: String,
}

impl GitVersion {
    /// Parses lines like `git version 2.51.0.windows.1` or
    /// `git version 2.39.2 (Apple Git-143)`.
    pub fn parse(line: &str) -> Option<Self> {
        let rest = line.trim().strip_prefix("git version ")?;
        let mut parts = rest.split(['.', ' ']);
        let major = leading_number(parts.next()?)?;
        let minor = leading_number(parts.next()?)?;
        let patch = parts.next().and_then(leading_number).unwrap_or(0);
        Some(Self {
            major,
            minor,
            patch,
            raw: rest.trim().to_string(),
        })
    }

    pub fn supported(&self) -> bool {
        (self.major, self.minor) >= MINIMUM_GIT
    }
}

fn leading_number(part: &str) -> Option<u32> {
    let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Runs `git --version` and parses the result.
pub async fn detect(
    executor: &GitExecutor,
    cancel: &CancellationToken,
) -> Result<GitVersion, GitError> {
    let cmd = GitCommand::new()
        .arg("--version")
        .timeout(Duration::from_secs(10));
    let out = executor.run(cmd, cancel).await?;
    let text = out.stdout_utf8();
    GitVersion::parse(&text).ok_or_else(|| GitError::UnexpectedOutput {
        command: "git --version".to_string(),
        message: text.trim().to_string(),
    })
}

/// Detects the installed git and fails with [`GitError::UnsupportedVersion`]
/// when it is older than [`MINIMUM_GIT`].
pub async fn ensure_supported(
    executor: &GitExecutor,
    cancel: &CancellationToken,
) -> Result<GitVersion, GitError> {
    let version = detect(executor, cancel).await?;
    if !version.supported() {
        return Err(GitError::UnsupportedVersion {
            found: version.raw,
            minimum: minimum_string(),
        });
    }
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_version() {
        let v = GitVersion::parse("git version 2.43.0").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (2, 43, 0));
        assert_eq!(v.raw, "2.43.0");
    }

    #[test]
    fn parses_windows_version() {
        let v = GitVersion::parse("git version 2.51.0.windows.1\n").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (2, 51, 0));
    }

    #[test]
    fn parses_apple_suffix() {
        let v = GitVersion::parse("git version 2.39.2 (Apple Git-143)").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (2, 39, 2));
        assert!(!v.supported());
    }

    #[test]
    fn parses_rc_version() {
        let v = GitVersion::parse("git version 2.44.0-rc1").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (2, 44, 0));
    }

    #[test]
    fn rejects_garbage() {
        assert!(GitVersion::parse("not git").is_none());
        assert!(GitVersion::parse("git version x.y").is_none());
    }

    #[test]
    fn support_boundaries() {
        let mk = |maj, min| GitVersion {
            major: maj,
            minor: min,
            patch: 0,
            raw: String::new(),
        };
        assert!(!mk(2, 42).supported());
        assert!(mk(2, 43).supported());
        assert!(mk(2, 44).supported());
        assert!(mk(3, 0).supported());
    }
}
