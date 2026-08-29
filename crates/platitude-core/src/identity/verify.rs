//! Verifying one commit's signature: what git makes of it, and the one
//! thing it will not say for itself.

use super::*;

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

    /// git's own letter for this verdict, as `%G?` spells it.
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
/// to check.
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

#[cfg(test)]
mod tests {
    use super::*;

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
