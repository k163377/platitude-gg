//! Who commits are attributed to, and how they are signed.
//!
//! Both are ordinary git configuration. This module reads them so the UI
//! can ask for an identity before the first commit fails, and can say
//! whether signing is in play — it does not reimplement either.
//!
//! **The application never handles a signing passphrase.** git delegates to
//! gpg-agent / ssh-agent, which prompt through their own pinentry. That is
//! the same delegation the credential helper gets, and it is the reason a
//! passphrase never passes through this process, never reaches a log, and
//! never needs storing. A pinentry configured for a terminal (`pinentry-tty`,
//! `pinentry-curses`) cannot work here — the subprocess has no console — so
//! a graphical pinentry is required; that is a configuration matter, not
//! something to work around by asking the user ourselves.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

/// Author identity recorded on new commits.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Identity {
    pub name: Option<String>,
    pub email: Option<String>,
}

impl Identity {
    /// True when git has everything it needs to commit without guessing.
    pub fn is_complete(&self) -> bool {
        self.name.is_some() && self.email.is_some()
    }
}

/// How commits are signed, if at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureFormat {
    /// `gpg.format` unset or `openpgp` — gpg-agent handles the key.
    OpenPgp,
    /// `ssh` — ssh-agent, or an unencrypted key file.
    Ssh,
    /// `x509` — gpgsm.
    X509,
}

impl SignatureFormat {
    fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "ssh" => SignatureFormat::Ssh,
            "x509" => SignatureFormat::X509,
            _ => SignatureFormat::OpenPgp,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SignatureFormat::OpenPgp => "openpgp",
            SignatureFormat::Ssh => "ssh",
            SignatureFormat::X509 => "x509",
        }
    }
}

/// Signing configuration in force for this repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningConfig {
    /// `commit.gpgsign`: every commit is signed.
    pub sign_commits: bool,
    /// `tag.gpgsign`: every annotated tag is signed.
    pub sign_tags: bool,
    pub format: SignatureFormat,
    /// `user.signingkey`; `None` means git picks by the committer identity.
    pub key: Option<String>,
}

impl Default for SigningConfig {
    fn default() -> Self {
        Self {
            sign_commits: false,
            sign_tags: false,
            format: SignatureFormat::OpenPgp,
            key: None,
        }
    }
}

impl SigningConfig {
    pub fn is_active(&self) -> bool {
        self.sign_commits || self.sign_tags
    }
}

/// Identity and signing configuration, read in one `git config` call.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthorConfig {
    pub identity: Identity,
    pub signing: SigningConfig,
}

/// Keys read together; `--get-regexp` normalizes them to lower case, so
/// `tag.gpgSign` arrives as `tag.gpgsign`.
const CONFIG_PATTERN: &str =
    r"^(user\.(name|email|signingkey)|commit\.gpgsign|tag\.gpgsign|gpg\.format)$";

/// Loads the author identity and signing configuration.
pub async fn load(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<AuthorConfig, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["config", "-z", "--get-regexp", CONFIG_PATTERN]);
    // Exit 1 only means no key matched, which is a valid (empty) answer.
    let out = executor.run_unchecked(cmd, cancel).await?;
    if out.code == 1 {
        return Ok(AuthorConfig::default());
    }
    if out.code != 0 {
        return Err(GitError::Failed {
            command: "git config --get-regexp".to_string(),
            code: out.code,
            stderr: out.failure_message(),
        });
    }
    Ok(parse_config(&out.stdout))
}

/// Parses `git config -z --get-regexp`: `key\nvalue` records, NUL-terminated.
fn parse_config(bytes: &[u8]) -> AuthorConfig {
    let mut config = AuthorConfig::default();
    for record in bytes.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let text = String::from_utf8_lossy(record);
        // A valueless key (`[commit] gpgsign`) has no newline and means true.
        let (key, value) = match text.split_once('\n') {
            Some((k, v)) => (k, v),
            None => (text.as_ref(), ""),
        };
        let value = value.trim();
        match key.trim() {
            "user.name" => config.identity.name = non_empty(value),
            "user.email" => config.identity.email = non_empty(value),
            "user.signingkey" => config.signing.key = non_empty(value),
            "commit.gpgsign" => config.signing.sign_commits = parse_bool(value),
            "tag.gpgsign" => config.signing.sign_tags = parse_bool(value),
            "gpg.format" => config.signing.format = SignatureFormat::parse(value),
            _ => {}
        }
    }
    config
}

fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}

/// git's boolean spelling; a key present with no value is true.
fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "true" | "yes" | "on" | "1"
    )
}

/// Verdict of verifying one commit's signature (git's `%G?` codes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureStatus {
    /// Signed by a key the user trusts.
    Good,
    /// Signed, but the content does not match — treat as hostile.
    Bad,
    /// Signature is fine; the key's validity is unknown.
    GoodUnknownValidity,
    /// Signature is fine but has expired.
    GoodExpired,
    /// Signature is fine but the key has expired.
    GoodExpiredKey,
    /// Signature is fine but the key was revoked.
    GoodRevokedKey,
    /// The key is missing, or no allowed-signers file is configured.
    CannotCheck,
    /// Not signed at all — the ordinary case.
    Absent,
}

impl SignatureStatus {
    fn parse(code: &str) -> Self {
        match code.trim() {
            "G" => SignatureStatus::Good,
            "B" => SignatureStatus::Bad,
            "U" => SignatureStatus::GoodUnknownValidity,
            "X" => SignatureStatus::GoodExpired,
            "Y" => SignatureStatus::GoodExpiredKey,
            "R" => SignatureStatus::GoodRevokedKey,
            "E" => SignatureStatus::CannotCheck,
            _ => SignatureStatus::Absent,
        }
    }

    /// git's own letter for this verdict, as `%G?` spells it. The UI
    /// shows one of three outcomes, so the letter is what carries the
    /// exact reason to a tooltip.
    pub fn code(self) -> &'static str {
        match self {
            SignatureStatus::Good => "G",
            SignatureStatus::Bad => "B",
            SignatureStatus::GoodUnknownValidity => "U",
            SignatureStatus::GoodExpired => "X",
            SignatureStatus::GoodExpiredKey => "Y",
            SignatureStatus::GoodRevokedKey => "R",
            SignatureStatus::CannotCheck => "E",
            SignatureStatus::Absent => "N",
        }
    }

    /// True when a signature exists, whatever its verdict.
    pub fn is_signed(self) -> bool {
        self != SignatureStatus::Absent
    }

    /// True when the signature verifies against a trusted key. Anything
    /// else — including "cannot check" — must not be shown as verified.
    pub fn is_trusted(self) -> bool {
        self == SignatureStatus::Good
    }
}

/// One commit's signature as git reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub status: SignatureStatus,
    /// Signer as git names them (`%GS`); empty when unsigned.
    pub signer: String,
    /// Key or fingerprint used (`%GK`); empty when unsigned.
    pub key: String,
}

/// Verifies one commit's signature.
///
/// Deliberately not part of [`crate::details`]: verification runs gpg or
/// ssh-keygen, and the details pane has a 100ms budget. The UI asks for
/// this only where it shows the result.
///
/// **The object is read first, and for an unsigned commit it is the only
/// thing read.** `%G?` cannot be trusted on its own — an SSH-signed commit
/// in a repository with no `gpg.ssh.allowedSignersFile` reports `N`, the
/// same code as an unsigned one, and measured on git 2.55 so does every
/// other placeholder git has for the question (`%GS` `%GK` `%GG` `%GF`
/// `%GP` are all empty for both, `%GT` is `undefined` for both). So the
/// header is what says whether a signature exists at all, and asking for
/// it first means a commit that carries none costs one process rather than
/// two — and never starts gpg or ssh-keygen to be told there was nothing
/// to check. Most commits in most repositories are that commit.
///
/// With a header present, `N` no longer reads as "unsigned": it means git
/// could not judge what is there, which is its own answer.
pub async fn verify_commit(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    cancel: &CancellationToken,
) -> Result<Signature, GitError> {
    if !has_signature_header(executor, workdir, rev, cancel).await? {
        return Ok(Signature {
            status: SignatureStatus::Absent,
            signer: String::new(),
            key: String::new(),
        });
    }
    let cmd =
        GitCommand::new()
            .cwd(workdir)
            .args(["log", "-1", "-z", "--format=%G?%x00%GS%x00%GK", rev]);
    let out = executor.run(cmd, cancel).await?;
    let text = out.stdout_utf8();
    let mut fields = text.split('\0');
    let mut signature = Signature {
        status: SignatureStatus::parse(fields.next().unwrap_or_default()),
        signer: fields.next().unwrap_or_default().trim().to_string(),
        key: fields.next().unwrap_or_default().trim().to_string(),
    };
    if signature.status == SignatureStatus::Absent {
        signature.status = SignatureStatus::CannotCheck;
    }
    Ok(signature)
}

/// Whether the commit object carries a signature header at all.
async fn has_signature_header(
    executor: &GitExecutor,
    workdir: &Path,
    rev: &str,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let cmd = GitCommand::new()
        .cwd(workdir)
        .args(["cat-file", "commit", rev]);
    let out = executor.run(cmd, cancel).await?;
    Ok(header_has_signature(&out.stdout))
}

/// Scans a raw commit object's headers, which end at the first blank line.
/// Stopping there matters: a message body may say anything at all.
fn header_has_signature(object: &[u8]) -> bool {
    for line in object.split(|b| *b == b'\n') {
        if line.is_empty() {
            return false;
        }
        // `gpgsig`, and `gpgsig-sha256` in a SHA-256 repository.
        if line.starts_with(b"gpgsig") {
            return true;
        }
    }
    false
}

/// Which configuration file a write lands in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigScope {
    /// This repository only.
    Local,
    /// The user's global configuration — the right default for a first-run
    /// prompt, since the answer is about the person, not the project.
    Global,
}

/// What a [`set_identity`] left behind.
///
/// Read back from git rather than echoed, for two reasons. An identity is
/// two `git config` calls and the lock on the configuration file is taken
/// and released per call, so the second one can fail on its own and leave
/// half of an identity — which reads as a whole one to everything
/// downstream, because both halves are set. And a repository-local
/// setting can sit over a global write, so even two calls that both
/// succeeded do not say what a commit will carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityWrite {
    /// The effective identity now, as git reports it.
    pub identity: Identity,
    /// git reports the name that was asked for.
    pub name_saved: bool,
    /// git reports the address that was asked for.
    pub email_saved: bool,
    /// git's own message from the last call that failed; empty when every
    /// one of them succeeded. A write can end unsaved with nothing here —
    /// that is the local setting sitting over the global one.
    pub message: String,
}

impl IdentityWrite {
    /// Both halves are what was asked for. Anything less is a half-written
    /// identity and must not be shown as a finished one.
    pub fn is_saved(&self) -> bool {
        self.name_saved && self.email_saved
    }
}

/// Records `user.name` and `user.email`, and answers what git holds after.
///
/// The pair is not atomic and cannot be made so: `git config` writes one
/// key per invocation and has no batch form, `--edit` holds no lock at all
/// (measured: a concurrent write lands while the editor is open), and
/// taking the lock directly would mean reimplementing git's own. So the
/// write reads itself back and, when what came out is not what was asked
/// for, goes again — once. Every one of these writes is idempotent, and
/// losing the lock is somebody else holding it for the moment it took to
/// spawn the second call, so a second pass settles contention. A failure
/// that is not contention answers the same way however often it is asked,
/// and the screen is a better place to wait than a loop.
///
/// Only an empty name is refused, because that is the only thing git
/// itself refuses — and it refuses it at commit time ("Author identity
/// unknown"), long after the value was entered. Everything else git
/// handles on its own, so nothing here second-guesses it (measured
/// against git 2.51):
///
/// - `<`, `>` and newlines are dropped when git builds an author line, and
///   leading/trailing spaces and punctuation are stripped
/// - a newline in a value is escaped as `\n` when git writes the config
///   file, so it cannot smuggle in another setting
/// - an empty *email* is accepted; the author line simply carries `<>`
///
/// Values are trimmed, which only anticipates what git does to them
/// anyway, and keeps the stored configuration equal to what commits show.
pub async fn set_identity(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    email: &str,
    scope: ConfigScope,
    cancel: &CancellationToken,
) -> Result<IdentityWrite, GitError> {
    let name = name.trim();
    let email = email.trim();
    if name.is_empty() {
        return Err(GitError::Rejected {
            message: "the name must not be empty".to_string(),
        });
    }

    let written = write_pair(executor, workdir, name, email, scope, cancel).await?;
    if written.is_saved() {
        return Ok(written);
    }
    write_pair(executor, workdir, name, email, scope, cancel).await
}

/// One pass: both keys, then what git makes of them.
///
/// Stops at the first call that fails — the second would be writing into a
/// configuration this pass has just been told it cannot change — but reads
/// back either way, because a call that failed says nothing about what the
/// file now holds.
async fn write_pair(
    executor: &GitExecutor,
    workdir: &Path,
    name: &str,
    email: &str,
    scope: ConfigScope,
    cancel: &CancellationToken,
) -> Result<IdentityWrite, GitError> {
    let mut message = String::new();
    for (key, value) in [("user.name", name), ("user.email", email)] {
        let mut cmd = GitCommand::new().cwd(workdir).arg("config");
        if scope == ConfigScope::Global {
            cmd = cmd.arg("--global");
        }
        // No `--` separator: `git config <key> -- <value>` stores "--" as
        // the value. A leading dash in the value is accepted as-is.
        cmd = cmd.args([key, value]);
        if let Err(e) = executor.run(cmd, cancel).await {
            // Shutting down is not a write that failed; it is no write.
            if e.is_cancelled() {
                return Err(e);
            }
            message = e.to_string();
            break;
        }
    }
    let config = load(executor, workdir, cancel).await?;
    Ok(IdentityWrite {
        name_saved: config.identity.name.as_deref() == Some(name),
        email_saved: config.identity.email.as_deref() == non_empty(email).as_deref(),
        identity: config.identity,
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn z(records: &[&str]) -> Vec<u8> {
        let mut v = Vec::new();
        for r in records {
            v.extend_from_slice(r.as_bytes());
            v.push(0);
        }
        v
    }

    #[test]
    fn parses_identity_and_signing() {
        let bytes = z(&[
            "user.name\nAda Lovelace",
            "user.email\nada@example.com",
            "user.signingkey\nABCD1234",
            "commit.gpgsign\ntrue",
            "gpg.format\nssh",
        ]);
        let config = parse_config(&bytes);
        assert_eq!(config.identity.name.as_deref(), Some("Ada Lovelace"));
        assert_eq!(config.identity.email.as_deref(), Some("ada@example.com"));
        assert!(config.identity.is_complete());
        assert!(config.signing.sign_commits);
        assert!(!config.signing.sign_tags);
        assert_eq!(config.signing.format, SignatureFormat::Ssh);
        assert_eq!(config.signing.key.as_deref(), Some("ABCD1234"));
        assert!(config.signing.is_active());
    }

    #[test]
    fn an_empty_configuration_is_incomplete_and_unsigned() {
        let config = parse_config(b"");
        assert!(!config.identity.is_complete());
        assert!(!config.signing.is_active());
        assert_eq!(config.signing.format, SignatureFormat::OpenPgp);
    }

    #[test]
    fn a_half_set_identity_is_incomplete() {
        let config = parse_config(&z(&["user.name\nAda"]));
        assert!(!config.identity.is_complete());
    }

    #[test]
    fn understands_gits_boolean_spellings() {
        for truthy in ["true", "yes", "on", "1", "TRUE", ""] {
            assert!(parse_bool(truthy), "{truthy:?} is true to git");
        }
        for falsy in ["false", "no", "off", "0"] {
            assert!(!parse_bool(falsy), "{falsy:?} is false to git");
        }
    }

    #[test]
    fn a_valueless_key_means_true() {
        let config = parse_config(&z(&["commit.gpgsign"]));
        assert!(config.signing.sign_commits);
    }

    #[test]
    fn only_a_good_signature_counts_as_trusted() {
        assert!(SignatureStatus::parse("G").is_trusted());
        for code in ["B", "U", "X", "Y", "R", "E"] {
            let status = SignatureStatus::parse(code);
            assert!(status.is_signed(), "{code} is a signature");
            assert!(!status.is_trusted(), "{code} must not read as verified");
        }
        let none = SignatureStatus::parse("N");
        assert!(!none.is_signed() && !none.is_trusted());
    }

    #[test]
    fn every_verdict_round_trips_through_its_git_letter() {
        for status in [
            SignatureStatus::Good,
            SignatureStatus::Bad,
            SignatureStatus::GoodUnknownValidity,
            SignatureStatus::GoodExpired,
            SignatureStatus::GoodExpiredKey,
            SignatureStatus::GoodRevokedKey,
            SignatureStatus::CannotCheck,
            SignatureStatus::Absent,
        ] {
            assert_eq!(SignatureStatus::parse(status.code()), status);
        }
    }

    #[test]
    fn finds_a_signature_header_but_only_in_the_headers() {
        let signed = b"tree abc\nparent def\nauthor A <a@x> 1 +0000\n\
gpgsig -----BEGIN SSH SIGNATURE-----\n -----END SSH SIGNATURE-----\n\nsubject\n";
        assert!(header_has_signature(signed));

        let sha256 = b"tree abc\ngpgsig-sha256 -----BEGIN PGP SIGNATURE-----\n\nsubject\n";
        assert!(header_has_signature(sha256));

        let unsigned = b"tree abc\nauthor A <a@x> 1 +0000\n\nsubject\n";
        assert!(!header_has_signature(unsigned));

        // A body that talks about signing must not count as one.
        let liar = b"tree abc\nauthor A <a@x> 1 +0000\n\ngpgsig is not here\n";
        assert!(!header_has_signature(liar));
    }
}
