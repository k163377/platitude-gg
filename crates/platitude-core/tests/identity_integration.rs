//! Author identity and commit signing on real repositories.
//!
//! Signing is exercised with SSH keys rather than OpenPGP: `ssh-keygen`
//! ships with git on every supported platform, and a passphrase-less test
//! key needs no agent. What is proven here is that the application's fixed
//! environment does not get in git's way — the passphrase path itself is
//! gpg-agent's / ssh-agent's business and never touches this process.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

mod support;

use std::path::Path;

use platitude_core::commit::{self, CommitOptions};
use platitude_core::identity::{self, ConfigScope, SignatureFormat, SignatureStatus};
use platitude_core::process::GitExecutor;
use platitude_core::repo::RepoInfo;
use support::TestRepo;
use tokio_util::sync::CancellationToken;

fn env() -> (GitExecutor, CancellationToken) {
    (GitExecutor::new(), CancellationToken::new())
}

async fn info(repo: &TestRepo) -> RepoInfo {
    let (exec, cancel) = env();
    platitude_core::repo::open(&exec, &repo.path, &cancel)
        .await
        .expect("open repo")
}

/// git config values are read back by git's own parser, which takes
/// forward slashes on every platform.
fn config_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Reads what git reports, from a repository whose local config sets it.
///
/// The "missing identity" case is a unit test on the parser instead: the
/// application deliberately reads the effective configuration, so this
/// process would fall back to the developer's own `~/.gitconfig` — unlike
/// [`TestRepo`], which isolates the git commands it runs itself.
#[tokio::test]
async fn reads_the_effective_identity_and_signing_state() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (exec, cancel) = env();

    let config = identity::load(&exec, &repo.path, &cancel)
        .await
        .expect("load");
    assert_eq!(config.identity.name.as_deref(), Some("Test User"));
    assert_eq!(config.identity.email.as_deref(), Some("test@example.com"));
    assert!(config.identity.is_complete());
    assert!(!config.signing.is_active(), "nothing is signed by default");
}

#[tokio::test]
async fn writes_the_identity_and_rejects_dangerous_values() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["config", "--unset", "user.name"]);
    repo.git(&["config", "--unset", "user.email"]);
    let (exec, cancel) = env();

    identity::set_identity(
        &exec,
        &repo.path,
        "山田 太郎",
        "taro@example.com",
        ConfigScope::Local,
        &cancel,
    )
    .await
    .expect("set identity");

    let config = identity::load(&exec, &repo.path, &cancel)
        .await
        .expect("load");
    assert_eq!(config.identity.name.as_deref(), Some("山田 太郎"));
    assert_eq!(config.identity.email.as_deref(), Some("taro@example.com"));

    // A commit now works and carries what was set.
    let repo_info = info(&repo).await;
    repo.write_file("b.txt", "two\n");
    repo.git(&["add", "--", "b.txt"]);
    commit::commit(
        &exec,
        &repo_info,
        "with the new identity",
        CommitOptions::default(),
        &cancel,
    )
    .await
    .expect("commit");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%an <%ae>"]),
        "山田 太郎 <taro@example.com>"
    );

    // A line break would end the line in the config file and let the rest
    // be read as another setting.
    let err = identity::set_identity(
        &exec,
        &repo.path,
        "Evil\n[core]\n\tpager = touch /tmp/pwned",
        "e@example.com",
        ConfigScope::Local,
        &cancel,
    )
    .await
    .expect_err("line break refused");
    assert!(err.to_string().contains("line break"), "{err}");
    assert_eq!(
        repo.git(&["config", "--get", "user.name"]),
        "山田 太郎",
        "the rejected write changed nothing"
    );
}

/// A value starting with a dash must be stored as the value, not read as
/// an option. `git config <key> -- <value>` stores "--", so no separator.
#[tokio::test]
async fn a_dash_leading_identity_is_stored_verbatim() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (exec, cancel) = env();

    identity::set_identity(
        &exec,
        &repo.path,
        "-dashed name",
        "-dash@example.com",
        ConfigScope::Local,
        &cancel,
    )
    .await
    .expect("set identity");
    let config = identity::load(&exec, &repo.path, &cancel)
        .await
        .expect("load");
    assert_eq!(config.identity.name.as_deref(), Some("-dashed name"));
    assert_eq!(config.identity.email.as_deref(), Some("-dash@example.com"));
}

/// Signing configuration is detected so the UI can say what is in force.
#[tokio::test]
async fn detects_signing_configuration() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["config", "commit.gpgsign", "true"]);
    repo.git(&["config", "tag.gpgSign", "true"]);
    repo.git(&["config", "gpg.format", "ssh"]);
    repo.git(&["config", "user.signingkey", "/keys/id.pub"]);
    let (exec, cancel) = env();

    let config = identity::load(&exec, &repo.path, &cancel)
        .await
        .expect("load");
    assert!(config.signing.sign_commits);
    assert!(
        config.signing.sign_tags,
        "tag.gpgSign is case-folded by git"
    );
    assert_eq!(config.signing.format, SignatureFormat::Ssh);
    assert_eq!(config.signing.key.as_deref(), Some("/keys/id.pub"));
    assert!(config.signing.is_active());
}

/// The whole signing path: a repository configured to sign produces signed
/// commits through this application's commit API, and the signature
/// verifies once the key is trusted.
#[tokio::test]
async fn commits_are_signed_and_verify_against_a_trusted_key() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");

    // A passphrase-less test key; a real one is unlocked by ssh-agent,
    // which this process never talks to.
    let key = repo.path.join("id_test");
    let out = std::process::Command::new("ssh-keygen")
        .args(["-t", "ed25519", "-N", "", "-C", "test@example.com", "-f"])
        .arg(&key)
        .output()
        .expect("run ssh-keygen");
    assert!(
        out.status.success(),
        "ssh-keygen failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let public = std::fs::read_to_string(key.with_extension("pub")).expect("read public key");

    repo.git(&["config", "gpg.format", "ssh"]);
    repo.git(&[
        "config",
        "user.signingkey",
        &config_path(&key.with_extension("pub")),
    ]);
    repo.git(&["config", "commit.gpgsign", "true"]);
    // The key file must not be world-readable for ssh-keygen to use it;
    // that is the platform's business and holds by default here.

    let (exec, cancel) = env();
    let repo_info = info(&repo).await;
    repo.write_file("b.txt", "two\n");
    repo.git(&["add", "--", "b.txt"]);
    commit::commit(
        &exec,
        &repo_info,
        "signed commit",
        CommitOptions::default(),
        &cancel,
    )
    .await
    .expect("signing commit succeeded under the fixed environment");

    // Without an allowed-signers file git reports `%G?` as `N` — the same
    // code as an unsigned commit. Reading the object itself keeps a signed
    // commit from being shown as unsigned, and "cannot check" must never
    // read as verified.
    let signature = identity::verify_commit(&exec, &repo.path, "HEAD", &cancel)
        .await
        .expect("verify");
    assert_eq!(
        signature.status,
        SignatureStatus::CannotCheck,
        "signed, but git has no key to judge it against"
    );
    assert!(signature.status.is_signed());
    assert!(!signature.status.is_trusted());

    // Trust the key, and the same commit verifies.
    let allowed = repo.path.join("allowed_signers");
    std::fs::write(&allowed, format!("test@example.com {public}")).expect("write allowed signers");
    repo.git(&[
        "config",
        "gpg.ssh.allowedSignersFile",
        &config_path(&allowed),
    ]);

    let signature = identity::verify_commit(&exec, &repo.path, "HEAD", &cancel)
        .await
        .expect("verify");
    assert_eq!(signature.status, SignatureStatus::Good, "{signature:?}");
    assert!(signature.status.is_trusted());
    assert_eq!(signature.signer, "test@example.com");

    // An unsigned commit is reported as such, not as a failure.
    repo.git(&["config", "commit.gpgsign", "false"]);
    repo.write_file("c.txt", "three\n");
    repo.git(&["add", "--", "c.txt"]);
    commit::commit(
        &exec,
        &repo_info,
        "unsigned commit",
        CommitOptions::default(),
        &cancel,
    )
    .await
    .expect("commit");
    let signature = identity::verify_commit(&exec, &repo.path, "HEAD", &cancel)
        .await
        .expect("verify");
    assert_eq!(signature.status, SignatureStatus::Absent);
    assert!(!signature.status.is_signed());
}
