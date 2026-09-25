//! Reading and writing `core.autocrlf` against real git. The boolean
//! vocabulary is held by the unit tests beside
//! [`platitude_core::eol::setting`]; these prove the two levels are two
//! files.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::isolated_global;
use platitude_core::eol::setting::{self, AutoCrlf, ConfigScope};
use platitude_core::process::GitExecutor;
use tokio_util::sync::CancellationToken;

/// `TestRepo` writes `core.autocrlf=false` locally, which would stand over
/// every global value these tests write.
fn no_local_setting() -> (TestRepo, GitExecutor, CancellationToken) {
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\n", "root");
    repo.git(&["config", "--local", "--unset", "core.autocrlf"]);
    let exec = isolated_global(repo.global_config());
    (repo, exec, CancellationToken::new())
}

#[tokio::test]
async fn a_level_that_sets_nothing_says_so() {
    let (repo, exec, cancel) = no_local_setting();

    for scope in [ConfigScope::Local, ConfigScope::Global] {
        assert_eq!(
            setting::held(&exec, &repo.path, scope, &cancel)
                .await
                .expect("held"),
            None,
            "{scope:?} writes nothing here"
        );
    }
    assert_eq!(
        setting::effective(&exec, &repo.path, &cancel)
            .await
            .expect("effective"),
        None,
        "and neither does anything under them, once the system is out"
    );
}

/// Giving the global value back is the errand the empty row exists for.
#[tokio::test]
async fn a_repository_stands_over_the_global_value_until_it_is_taken_out() {
    let (repo, exec, cancel) = no_local_setting();
    let inherited = setting::set(
        &exec,
        &repo.path,
        ConfigScope::Global,
        Some(AutoCrlf::True),
        &cancel,
    )
    .await
    .expect("set global");
    assert!(
        inherited.saved,
        "git reports what was asked for: {inherited:?}"
    );
    assert!(inherited.message.is_empty(), "nothing to report");
    assert_eq!(
        inherited.held,
        Some(AutoCrlf::True),
        "read back out of the global file"
    );

    let written = setting::set(
        &exec,
        &repo.path,
        ConfigScope::Local,
        Some(AutoCrlf::False),
        &cancel,
    )
    .await
    .expect("set local");
    assert!(written.saved, "{written:?}");
    assert_eq!(written.held, Some(AutoCrlf::False));
    assert_eq!(
        setting::effective(&exec, &repo.path, &cancel)
            .await
            .expect("effective"),
        Some(AutoCrlf::False),
        "the repository's own file is the last word"
    );

    let given_back = setting::set(&exec, &repo.path, ConfigScope::Local, None, &cancel)
        .await
        .expect("unset local");
    assert!(given_back.saved, "{given_back:?}");
    assert!(given_back.message.is_empty(), "nothing to report");
    assert_eq!(given_back.held, None);
    assert_eq!(
        setting::effective(&exec, &repo.path, &cancel)
            .await
            .expect("effective"),
        Some(AutoCrlf::True),
        "and the global value is back in force"
    );
}

/// `--unset` of an absent key exits 5, the same code as a key written
/// twice, so the write reads the file first and leaves a level that
/// already says nothing alone.
#[tokio::test]
async fn taking_out_a_key_that_was_never_there_is_not_a_failure() {
    let (repo, exec, cancel) = no_local_setting();

    let written = setting::set(&exec, &repo.path, ConfigScope::Local, None, &cancel)
        .await
        .expect("unset local");
    assert!(written.saved, "{written:?}");
    assert!(
        written.message.is_empty(),
        "git was never asked, so it has nothing to say: {written:?}"
    );
    assert_eq!(written.held, None);
}
