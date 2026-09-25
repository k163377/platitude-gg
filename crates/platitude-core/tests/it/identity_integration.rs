//! Author identity and commit signing on real repositories.
//!
//! Signing uses SSH keys: `ssh-keygen` ships with git on every supported
//! platform, and a passphrase-less key needs no agent. Proven here: the
//! fixed environment does not get in git's way. The passphrase path is the
//! agent's and never touches this process.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::support::exec::{env, isolated_global, observed_env};
use crate::support::{TestRepo, info};
use platitude_core::commit::{self, CommitOptions};
use platitude_core::identity::{self, ConfigScope, SignatureFormat, SignatureStatus};
use platitude_core::process::{CommandEnd, CommandObserver, GitExecutor, Kept};
use tokio_util::sync::CancellationToken;

/// Records what was spawned, to count the processes a reading spends.
#[derive(Default)]
struct Spawns(Mutex<Vec<String>>);

impl Spawns {
    fn seen(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

impl CommandObserver for Spawns {
    fn records(&self, _kept: Kept) -> bool {
        true
    }

    fn started(
        &self,
        display: &str,
        _full: &str,
        _kept: Kept,
        _operation: Option<platitude_core::OperationId>,
    ) -> u64 {
        let mut seen = self.0.lock().unwrap();
        seen.push(display.to_string());
        seen.len() as u64
    }

    fn finished(
        &self,
        _id: u64,
        _end: CommandEnd,
        _waited_ms: u64,
        _elapsed_ms: u64,
        _message: &str,
    ) {
    }
}

fn counted() -> (GitExecutor, Arc<Spawns>, CancellationToken) {
    let spawns = Arc::new(Spawns::default());
    // `Kept::Unasked`: the reads a session makes on its own; `records`
    // keeps them all, since the count is the point.
    let (exec, cancel) = observed_env(spawns.clone(), Kept::Unasked);
    (exec, spawns, cancel)
}

/// git config values are read back by git's own parser, which takes
/// forward slashes on every platform.
fn config_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// An identity is two `git config` calls, each taking the file lock, so
/// the second can fail alone — and then `is_complete()` says yes while the
/// address is the one being replaced. A multi-valued `user.email` refuses
/// a plain set every time, which makes it the shape to test against.
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

    // What is reported is what git now holds.
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

/// `git config <key> -- <value>` stores "--", so no separator.
#[tokio::test]
async fn a_dash_leading_identity_is_stored_verbatim() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (exec, cancel) = env();

    let written = identity::set_identity(
        &exec,
        &repo.path,
        "-dashed name",
        "-dash@example.com",
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
    assert_eq!(config.identity.name.as_deref(), Some("-dashed name"));
    assert_eq!(config.identity.email.as_deref(), Some("-dash@example.com"));
}

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

#[tokio::test]
async fn commits_are_signed_and_verify_against_a_trusted_key() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");

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
    // ssh-keygen wants the key readable by its owner alone, which holds by
    // default here.

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

    // Without allowed signers `%G?` is `N`, as for an unsigned commit;
    // reading the object's header keeps this from showing as unsigned.
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

/// Every selected row asks this, so an unsigned commit must not pay for a
/// verification run.
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
    assert_eq!(signature.status, SignatureStatus::CannotCheck);
    let seen = spawns.seen();
    assert_eq!(seen.len(), 2, "{seen:?}");
    assert!(seen[1].starts_with("git log -1"), "{seen:?}");
}

// ---- what one repository sets for itself -----------------------------
//
// The screen's two boxes stand empty for "not written here", so the levels
// must be told apart: an inherited value arrives as inherited, and an
// emptied box takes the key out of one file only.

/// It inherits, and the two reads say so differently: the reason there are
/// two.
#[tokio::test]
async fn a_repository_that_sets_nothing_of_its_own_reads_as_empty() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    // `TestRepo` writes the identity into the repository's own file.
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

/// The combinations are a unit test (`identity::local::keys_to_write`).
/// Here: the list reaches git as written, at `--local`, and writing
/// nothing spawns nothing — `--unset` of an absent key exits 5 like a real
/// refusal, so a blind unset would fail a save that had nothing to do.
#[tokio::test]
async fn the_keys_the_decision_names_are_the_ones_git_is_asked_for() {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    let (exec, spawns, cancel) = counted();

    let written =
        identity::set_local_identity(&exec, &repo.path, "Test User", "work@example.com", &cancel)
            .await
            .expect("set_local_identity");
    assert!(written.is_saved(), "{written:?}");
    assert_eq!(
        writes(&spawns),
        vec!["git config --local user.email 'work@example.com'"],
        "the name was already this, so nothing was spawned for it"
    );

    // Both emptied while both are held: both come out.
    let (exec, spawns, cancel) = counted();
    let written = identity::set_local_identity(&exec, &repo.path, "", "", &cancel)
        .await
        .expect("set_local_identity");
    assert!(written.is_saved(), "{written:?}");
    assert_eq!(written.identity, identity::Identity::default());
    assert_eq!(
        writes(&spawns),
        vec![
            "git config --local --unset user.name",
            "git config --local --unset user.email"
        ]
    );

    // Again, over a file that holds neither: the `--unset` git would
    // refuse must not be sent.
    let (exec, spawns, cancel) = counted();
    let written = identity::set_local_identity(&exec, &repo.path, "", "", &cancel)
        .await
        .expect("set_local_identity");
    assert!(
        written.is_saved(),
        "there was nothing to take out: {written:?}"
    );
    assert_eq!(writes(&spawns), Vec::<String>::new());
}

/// The spawned commands that were not reads.
fn writes(spawns: &Spawns) -> Vec<String> {
    spawns
        .seen()
        .into_iter()
        .filter(|command| !command.contains("--get-regexp"))
        .collect()
}

/// What git makes of an identity it was handed. What this end writes and
/// reads back is held above, and the empty-name refusal is a unit test
/// beside `set_identity`; the rest moves only with git, so the full gate
/// (`-- --ignored ::periodic::`) runs it, not every change.
mod periodic {
    use super::*;

    #[tokio::test]
    #[ignore = "what git makes of the identity it was handed: not worth the pre-merge run"]
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

        // git escapes a newline as `\n` in the config file, so nothing here
        // guards against a smuggled setting.
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

        // git drops `<`, `>` and surrounding spaces from the author line.
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

        // The one thing git refuses, refused before commit time (where it
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

    /// The errand this exists for: another address for this project, under
    /// the everyday name.
    #[tokio::test]
    #[ignore = "git assembling an author out of two files: not worth the pre-merge run"]
    async fn one_key_is_overridden_while_the_other_stays_inherited() {
        let mut repo = TestRepo::init();
        repo.commit_file("a.txt", "one\n", "root");
        repo.git(&["config", "--local", "--unset", "user.name"]);
        repo.git(&["config", "--local", "--unset", "user.email"]);
        repo.git(&["config", "--global", "user.name", "Ada Lovelace"]);
        repo.git(&["config", "--global", "user.email", "ada@example.com"]);
        let exec = isolated_global(repo.global_config());
        let cancel = CancellationToken::new();

        let written =
            identity::set_local_identity(&exec, &repo.path, "", "work@example.com", &cancel)
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

    #[tokio::test]
    #[ignore = "git's --local unset leaving the global file alone: not worth the pre-merge run"]
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
}
