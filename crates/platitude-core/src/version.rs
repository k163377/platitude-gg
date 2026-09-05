//! Detection of the installed git version, and how it stands against the
//! supported minimum.
//!
//! Below that minimum is a fact reported, not a door closed: what the app
//! does about it is the app's to decide, and it keeps working while saying
//! so (規約 §ウィンドウの縁 の 4 つ目のバッジ).

use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// Minimum supported git version (source of truth:
/// internal-docs/git最低バージョン整合.md — Ubuntu 24.04 LTS
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

/// What one candidate git answered when it was asked its version.
///
/// **Four answers rather than a `Result`**, because two of them are not
/// failures: a git below the minimum runs the app all the same (§git が
/// 無い時・古い時), and the difference between "nothing to run there" and
/// "it ran and said something else" is the whole of what a reader needs
/// to fix a path they typed. The words are the caller's — this says which
/// of the four it is, and carries only what git or the OS said itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Probe {
    /// It ran, and named a version at or above [`MINIMUM_GIT`].
    Supported(GitVersion),
    /// It ran, and named an older one.
    Old(GitVersion),
    /// There is nothing to run at that path.
    Missing,
    /// It could not be started, or it started and did not answer like
    /// git. `message` is the reason in git's or the OS's own words.
    Failed { message: String },
}

impl Probe {
    /// Whether git answered at all — the two versions, old or not.
    pub fn answered(&self) -> bool {
        matches!(self, Self::Supported(_) | Self::Old(_))
    }

    /// The version string git printed, or empty where it printed none.
    pub fn version(&self) -> &str {
        match self {
            Self::Supported(v) | Self::Old(v) => &v.raw,
            _ => "",
        }
    }
}

/// Asks one git binary for its version, without disturbing the one the
/// application is already running on.
///
/// `program` is a path, or empty for whichever git `PATH` resolves — the
/// same vocabulary [`crate::settings::Defaults::git_path`] holds, because
/// this is what reads it.
pub async fn probe(program: &str, cancel: &CancellationToken) -> Probe {
    let executor = if program.is_empty() {
        GitExecutor::new()
    } else {
        GitExecutor::with_program(program)
    };
    match detect(&executor, cancel).await {
        Ok(version) if version.supported() => Probe::Supported(version),
        Ok(version) => Probe::Old(version),
        Err(GitError::GitNotFound { .. }) => Probe::Missing,
        Err(error) => Probe::Failed {
            message: error.to_string(),
        },
    }
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
