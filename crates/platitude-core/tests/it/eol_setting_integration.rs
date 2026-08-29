//! Reading and writing `core.autocrlf` against real git.
//!
//! The unit tests beside [`platitude_core::eol::setting`] hold what git's
//! own boolean vocabulary parses to; these prove the two levels really are
//! two files — that a global value is inherited where a repository writes
//! nothing, that a repository's own value stands over it, and that taking
//! one back out puts the inherited one back in force.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::isolated_global;
use platitude_core::eol;
use platitude_core::eol::setting::{self, AutoCrlf, ConfigScope};
use platitude_core::process::GitExecutor;
use tokio_util::sync::CancellationToken;

/// A repository whose own file sets nothing, so the global level is the
/// only thing left to answer — `TestRepo` writes `core.autocrlf=false`
/// into every repository it makes, which would otherwise stand over every
/// global value these tests write.
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

/// The global write, and the inheritance that is the reason the repository
/// level has an empty row at all.
#[tokio::test]
async fn a_global_value_is_what_a_repository_inherits() {
    let (repo, exec, cancel) = no_local_setting();

    let written = setting::set(
        &exec,
        &repo.path,
        ConfigScope::Global,
        Some(AutoCrlf::Input),
        &cancel,
    )
    .await
    .expect("set global");
    assert!(written.saved, "git reports what was asked for: {written:?}");
    assert!(written.message.is_empty(), "nothing to report");
    assert_eq!(written.held, Some(AutoCrlf::Input));

    assert_eq!(
        setting::held(&exec, &repo.path, ConfigScope::Local, &cancel)
            .await
            .expect("held local"),
        None,
        "the repository still writes nothing of its own"
    );
    assert_eq!(
        setting::effective(&exec, &repo.path, &cancel)
            .await
            .expect("effective"),
        Some(AutoCrlf::Input),
        "and reads the global value as its own"
    );
}

/// The repository level, standing over the global one and then giving it
/// back — the errand the empty row exists for.
#[tokio::test]
async fn a_repository_stands_over_the_global_value_until_it_is_taken_out() {
    let (repo, exec, cancel) = no_local_setting();
    setting::set(
        &exec,
        &repo.path,
        ConfigScope::Global,
        Some(AutoCrlf::True),
        &cancel,
    )
    .await
    .expect("set global");

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

/// **`--unset` fails when there was nothing to unset** (measured 2.55: exit 5,
/// the same code as its refusal to touch a key written twice), so the
/// write reads the file first and leaves a level that already says it
/// alone. Without that, the empty row would report git's failure every
/// time it was picked on a repository that had never set the key.
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

/// Writing the value that is already there is the same non-event, and for
/// the same reason: the screen writes on every pick, including a pick of
/// the row already showing.
#[tokio::test]
async fn writing_what_is_already_there_changes_nothing() {
    let (repo, exec, cancel) = no_local_setting();
    setting::set(
        &exec,
        &repo.path,
        ConfigScope::Local,
        Some(AutoCrlf::True),
        &cancel,
    )
    .await
    .expect("set local");

    let again = setting::set(
        &exec,
        &repo.path,
        ConfigScope::Local,
        Some(AutoCrlf::True),
        &cancel,
    )
    .await
    .expect("set local again");
    assert!(again.saved, "{again:?}");
    assert!(again.message.is_empty(), "{again:?}");
    assert_eq!(again.held, Some(AutoCrlf::True));
}

/// The notice reads this key through the same door, so what the settings
/// screen writes is what it decides by — including `input`, which converts
/// on the way in and is therefore git deciding the stored endings.
#[tokio::test]
async fn what_is_written_is_what_the_notice_reads() {
    let (repo, exec, cancel) = no_local_setting();

    for (wanted, converts) in [
        (Some(AutoCrlf::True), true),
        (Some(AutoCrlf::Input), true),
        (Some(AutoCrlf::False), false),
        (None, false),
    ] {
        setting::set(&exec, &repo.path, ConfigScope::Local, wanted, &cancel)
            .await
            .expect("set local");
        assert_eq!(
            eol::normalises(&exec, &repo.path, &cancel)
                .await
                .expect("normalises"),
            converts,
            "{wanted:?}"
        );
    }
}

/// git's own spellings, straight out of a file this app did not write.
/// The screen shows one of three rows whatever the file says, so a value
/// spelled `yes` has to arrive as the row that means the same thing.
#[tokio::test]
async fn a_value_written_in_gits_other_spellings_still_reads() {
    let (mut repo, exec, cancel) = no_local_setting();

    for (spelling, expected) in [
        ("yes", Some(AutoCrlf::True)),
        ("INPUT", Some(AutoCrlf::Input)),
        ("off", Some(AutoCrlf::False)),
    ] {
        repo.git(&["config", "--local", "core.autocrlf", spelling]);
        assert_eq!(
            setting::held(&exec, &repo.path, ConfigScope::Local, &cancel)
                .await
                .expect("held local"),
            expected,
            "{spelling}"
        );
    }
}
