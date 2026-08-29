//! What git will attribute a commit to, and whether it will sign it —
//! read out of the configuration in one call.

use super::*;

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

pub async fn load(
    executor: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<AuthorConfig, GitError> {
    let out = config::get_regexp(
        executor,
        workdir,
        CONFIG_PATTERN,
        "git config --get-regexp",
        cancel,
    )
    .await?;
    Ok(parse_config(&out))
}

fn parse_config(bytes: &[u8]) -> AuthorConfig {
    let mut parsed = AuthorConfig::default();
    for record in config::parse_z_records(bytes) {
        // A valueless key (`[commit] gpgsign`) means true, so it reads as
        // an empty value rather than being skipped.
        let value = record.value().unwrap_or_default().trim();
        match record.key().trim() {
            "user.name" => parsed.identity.name = non_empty(value),
            "user.email" => parsed.identity.email = non_empty(value),
            "user.signingkey" => parsed.signing.key = non_empty(value),
            "commit.gpgsign" => parsed.signing.sign_commits = parse_bool(value),
            "tag.gpgsign" => parsed.signing.sign_tags = parse_bool(value),
            "gpg.format" => parsed.signing.format = SignatureFormat::parse(value),
            _ => {}
        }
    }
    parsed
}

pub(super) fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}

/// git's boolean spelling; a key present with no value is true.
fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "true" | "yes" | "on" | "1"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::z;

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
}
