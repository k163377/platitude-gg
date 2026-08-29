//! Author identity and commit signing on real repositories.
//!
//! Signing is exercised with SSH keys rather than OpenPGP: `ssh-keygen`
//! ships with git on every supported platform, and a passphrase-less test
//! key needs no agent. What is proven here is that the application's fixed
//! environment does not get in git's way — the passphrase path itself is
//! gpg-agent's / ssh-agent's business and never touches this process.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::support::exec::{env, isolated_global, observed_env};
use crate::support::{TestRepo, info};
use platitude_core::commit::{self, CommitOptions};
use platitude_core::identity::{self, ConfigScope, SignatureFormat, SignatureStatus};
use platitude_core::process::{CommandEnd, CommandObserver, GitExecutor};
use tokio_util::sync::CancellationToken;

/// Records what was spawned, so a test can count processes rather than
/// take the answer's word for how it was reached.
#[derive(Default)]
struct Spawns(Mutex<Vec<String>>);

impl Spawns {
    fn seen(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

impl CommandObserver for Spawns {
    fn records(&self, _user: bool) -> bool {
        true
    }

    fn started(&self, display: &str, _full: &str, _user: bool) -> u64 {
        let mut seen = self.0.lock().unwrap();
        seen.push(display.to_string());
        seen.len() as u64
    }

    fn finished(&self, _id: u64, _end: CommandEnd, _elapsed_ms: u64, _message: &str) {}
}

fn counted() -> (GitExecutor, Arc<Spawns>, CancellationToken) {
    let spawns = Arc::new(Spawns::default());
    // `false`: these are the reads a session makes on its own, and what the
    // test counts is that they are spawned, not that they reach the log.
    let (exec, cancel) = observed_env(spawns.clone(), false);
    (exec, spawns, cancel)
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
async fn writes_the_identity_and_leaves_sanitizing_to_git() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["config", "--unset", "user.name"]);
    repo.git(&["config", "--unset", "user.email"]);
    let (exec, cancel) = env();

    let written = identity::set_identity(
        &exec,
        &repo.path,
        "山田 太郎",
        "taro@example.com",
        ConfigScope::Local,
        &cancel,
    )
    .await
    .expect("set identity");
    assert!(written.is_saved(), "both halves landed: {written:?}");
    assert!(written.message.is_empty(), "nothing to report");

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

    // A newline cannot smuggle in another setting: git escapes it as `\n`
    // when it writes the config file. Nothing has to guard against it.
    let injection = "Evil\n[core]\n\tpager = touch /tmp/pwned";
    identity::set_identity(
        &exec,
        &repo.path,
        injection,
        "e@example.com",
        ConfigScope::Local,
        &cancel,
    )
    .await
    .expect("set identity");
    assert_eq!(repo.git(&["config", "--get", "user.name"]), injection);
    repo.git_expect_failure(&["config", "--get", "core.pager"]);

    // What git dislikes it drops itself: `<` and `>` never reach an author
    // line, and neither do surrounding spaces.
    identity::set_identity(
        &exec,
        &repo.path,
        "  Ada Lovelace  ",
        "ada<at>example.com",
        ConfigScope::Local,
        &cancel,
    )
    .await
    .expect("set identity");
    assert_eq!(
        repo.git(&["config", "--get", "user.email"]),
        "ada<at>example.com",
        "stored as typed"
    );
    repo.write_file("c.txt", "three\n");
    repo.git(&["add", "--", "c.txt"]);
    commit::commit(
        &exec,
        &repo_info,
        "with a sanitized identity",
        CommitOptions::default(),
        &cancel,
    )
    .await
    .expect("commit");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%an <%ae>"]),
        "Ada Lovelace <adaatexample.com>"
    );

    // The one thing git does refuse, refused before commit time (where it
    // would surface as "Author identity unknown").
    let err = identity::set_identity(
        &exec,
        &repo.path,
        "   ",
        "e@example.com",
        ConfigScope::Local,
        &cancel,
    )
    .await
    .expect_err("an empty name is refused");
    assert!(err.to_string().contains("must not be empty"), "{err}");
}

/// An identity is two `git config` calls, and the second one can fail on
/// its own — the lock on the configuration file is taken and released per
/// call, so another process can hold it for the second and not the first.
/// The half that landed must not read as a finished identity: both halves
/// are set, so `is_complete()` says yes while the address belongs to the
/// identity the user was replacing.
///
/// A `user.email` with two values in the file refuses a plain set the same
/// way (measured: exit 5, `cannot overwrite multiple values`) and refuses
/// it every time, which is what makes it the shape to test against.
#[tokio::test]
async fn a_write_that_only_half_lands_says_which_half() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["config", "--unset-all", "user.email"]);
    repo.git(&["config", "--add", "user.email", "personal@example.com"]);
    repo.git(&["config", "--add", "user.email", "second@example.com"]);
    let (exec, cancel) = env();

    let written = identity::set_identity(
        &exec,
        &repo.path,
        "Work Name",
        "work@example.com",
        ConfigScope::Local,
        &cancel,
    )
    .await
    .expect("the read-back still answers");

    assert!(!written.is_saved(), "{written:?}");
    assert!(written.name_saved, "the first call landed");
    assert!(!written.email_saved, "the second one did not");
    assert!(!written.message.is_empty(), "git's own message comes back");

    // What is reported is what git now holds, not what was asked for: the
    // name moved, the address did not.
    assert_eq!(written.identity.name.as_deref(), Some("Work Name"));
    assert_ne!(
        written.identity.email.as_deref(),
        Some("work@example.com"),
        "the address is still the one being replaced"
    );
    assert!(
        written.identity.is_complete(),
        "and it looks finished, which is the whole reason to say otherwise"
    );
}

/// A configuration file already locked by somebody else takes neither
/// half. Nothing changes, and the answer says nothing changed rather than
/// leaving the screen to guess.
#[tokio::test]
async fn a_locked_configuration_takes_neither_half() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let lock = repo.path.join(".git").join("config.lock");
    std::fs::write(&lock, b"").expect("hold the configuration lock");
    let (exec, cancel) = env();

    let written = identity::set_identity(
        &exec,
        &repo.path,
        "Work Name",
        "work@example.com",
        ConfigScope::Local,
        &cancel,
    )
    .await
    .expect("reading needs no lock");

    assert!(!written.is_saved(), "{written:?}");
    assert!(!written.name_saved && !written.email_saved);
    assert!(!written.message.is_empty(), "git's own message comes back");
    assert_eq!(written.identity.name.as_deref(), Some("Test User"));
    assert_eq!(written.identity.email.as_deref(), Some("test@example.com"));
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

/// A commit carrying no signature is answered by the object alone. Every
/// selected row asks this question, so the common case paying for a
/// verification run that has nothing to verify is a process per click.
#[tokio::test]
async fn an_unsigned_commit_is_answered_without_asking_git_to_verify() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "unsigned");
    let (exec, spawns, cancel) = counted();

    let signature = identity::verify_commit(&exec, &repo.path, "HEAD", &cancel)
        .await
        .expect("verify");
    assert_eq!(signature.status, SignatureStatus::Absent);

    let seen = spawns.seen();
    assert_eq!(seen.len(), 1, "one process, not two: {seen:?}");
    assert!(seen[0].starts_with("git cat-file commit"), "{seen:?}");
}

/// A signed one costs the second process, and it is the verification —
/// the order only spares the case that had nothing to verify.
#[tokio::test]
async fn a_signed_commit_still_costs_the_verification() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let key = repo.path.join("id_test");
    let out = std::process::Command::new("ssh-keygen")
        .args(["-t", "ed25519", "-N", "", "-C", "test@example.com", "-f"])
        .arg(&key)
        .output()
        .expect("run ssh-keygen");
    assert!(out.status.success(), "ssh-keygen failed");
    repo.git(&["config", "gpg.format", "ssh"]);
    repo.git(&[
        "config",
        "user.signingkey",
        &config_path(&key.with_extension("pub")),
    ]);
    repo.write_file("b.txt", "two\n");
    repo.git(&["add", "--", "b.txt"]);
    repo.git(&["commit", "-S", "-m", "signed"]);

    let (exec, spawns, cancel) = counted();
    let signature = identity::verify_commit(&exec, &repo.path, "HEAD", &cancel)
        .await
        .expect("verify");
    // No allowed-signers file, so git cannot judge it — and the header is
    // what keeps that from reading as "unsigned".
    assert_eq!(signature.status, SignatureStatus::CannotCheck);
    let seen = spawns.seen();
    assert_eq!(seen.len(), 2, "{seen:?}");
    assert!(seen[1].starts_with("git log -1"), "{seen:?}");
}

// ---- what one repository sets for itself -----------------------------
//
// The screen these answer for offers a repository from the tab strip and
// two boxes standing empty for "not written here", so what has to hold is
// that the two levels are told apart at all: an inherited value must not
// arrive looking like an override, and an emptied box must take the key
// out of one file without reaching the other.

/// A repository that writes nothing of its own inherits — and the two
/// reads say so differently, which is the whole reason there are two.
#[tokio::test]
async fn a_repository_that_sets_nothing_of_its_own_reads_as_empty() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    // `TestRepo` writes the identity into the repository's own file, which
    // is the very thing being taken away here.
    repo.git(&["config", "--local", "--unset", "user.name"]);
    repo.git(&["config", "--local", "--unset", "user.email"]);
    repo.git(&["config", "--global", "user.name", "Ada Lovelace"]);
    repo.git(&["config", "--global", "user.email", "ada@example.com"]);
    let exec = isolated_global(repo.global_config());
    let cancel = CancellationToken::new();

    let held = identity::load_local(&exec, &repo.path, &cancel)
        .await
        .expect("load_local");
    assert_eq!(
        held,
        identity::Identity::default(),
        "nothing is written in this repository's own file"
    );

    let effective = identity::load(&exec, &repo.path, &cancel)
        .await
        .expect("load")
        .identity;
    assert_eq!(effective.name.as_deref(), Some("Ada Lovelace"));
    assert_eq!(effective.email.as_deref(), Some("ada@example.com"));
}

/// The errand the whole thing exists for: another address for this
/// project, under the name the person already goes by everywhere else.
#[tokio::test]
async fn one_key_is_overridden_while_the_other_stays_inherited() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["config", "--local", "--unset", "user.name"]);
    repo.git(&["config", "--local", "--unset", "user.email"]);
    repo.git(&["config", "--global", "user.name", "Ada Lovelace"]);
    repo.git(&["config", "--global", "user.email", "ada@example.com"]);
    let exec = isolated_global(repo.global_config());
    let cancel = CancellationToken::new();

    let written = identity::set_local_identity(&exec, &repo.path, "", "work@example.com", &cancel)
        .await
        .expect("set_local_identity");
    assert!(written.is_saved(), "both halves as asked: {written:?}");
    assert!(written.message.is_empty(), "nothing to report");
    assert_eq!(written.identity.name, None, "the name is not written here");
    assert_eq!(written.identity.email.as_deref(), Some("work@example.com"));

    let effective = identity::load(&exec, &repo.path, &cancel)
        .await
        .expect("load")
        .identity;
    assert_eq!(
        effective.name.as_deref(),
        Some("Ada Lovelace"),
        "inherited, because this repository says nothing about it"
    );
    assert_eq!(effective.email.as_deref(), Some("work@example.com"));

    // A commit carries the pair git assembled out of the two files.
    let repo_info = info(&repo).await;
    repo.write_file("b.txt", "two\n");
    repo.git(&["add", "--", "b.txt"]);
    commit::commit(
        &exec,
        &repo_info,
        "under the override",
        CommitOptions::default(),
        &cancel,
    )
    .await
    .expect("commit");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%an <%ae>"]),
        "Ada Lovelace <work@example.com>"
    );
}

/// Emptying a box takes the key out of this repository's file, and out of
/// that one only: what the person has set for themselves is still there
/// to fall back to.
#[tokio::test]
async fn an_emptied_box_takes_the_override_out_and_leaves_the_global_alone() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["config", "--global", "user.name", "Ada Lovelace"]);
    repo.git(&["config", "--global", "user.email", "ada@example.com"]);
    let exec = isolated_global(repo.global_config());
    let cancel = CancellationToken::new();

    let written = identity::set_local_identity(&exec, &repo.path, "", "", &cancel)
        .await
        .expect("set_local_identity");
    assert!(written.is_saved(), "both were taken out: {written:?}");
    assert_eq!(written.identity, identity::Identity::default());

    let effective = identity::load(&exec, &repo.path, &cancel)
        .await
        .expect("load")
        .identity;
    assert_eq!(effective.name.as_deref(), Some("Ada Lovelace"));
    assert_eq!(effective.email.as_deref(), Some("ada@example.com"));
    let global = std::fs::read_to_string(repo.global_config()).expect("read global config");
    assert!(global.contains("Ada Lovelace"), "{global}");
}

/// A save that asks for what the file already says spawns no write at
/// all. Not thrift: `git config --unset` fails when there was nothing to
/// unset, and it fails with the same exit code as its refusal to touch a
/// key written twice (measured, git 2.55: both are 5), so reading first is what
/// keeps a real failure from passing for a harmless one.
#[tokio::test]
async fn a_save_that_changes_nothing_asks_git_to_write_nothing() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (exec, spawns, cancel) = counted();

    let written =
        identity::set_local_identity(&exec, &repo.path, "Test User", "test@example.com", &cancel)
            .await
            .expect("set_local_identity");
    assert!(
        written.is_saved(),
        "already what was asked for: {written:?}"
    );

    let seen = spawns.seen();
    assert!(
        seen.iter().all(|command| command.contains("--get-regexp")),
        "only reads went out: {seen:?}"
    );
}

/// The same from the other side: a repository that already writes nothing
/// is not asked to take out keys it does not have.
#[tokio::test]
async fn emptying_boxes_that_are_already_empty_asks_git_to_write_nothing() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["config", "--local", "--unset", "user.name"]);
    repo.git(&["config", "--local", "--unset", "user.email"]);
    let (exec, spawns, cancel) = counted();

    let written = identity::set_local_identity(&exec, &repo.path, "", "", &cancel)
        .await
        .expect("set_local_identity");
    assert!(
        written.is_saved(),
        "there was nothing to take out: {written:?}"
    );

    let seen = spawns.seen();
    assert!(
        seen.iter().all(|command| command.contains("--get-regexp")),
        "only reads went out: {seen:?}"
    );
}
