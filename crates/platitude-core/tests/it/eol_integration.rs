//! Line-ending readings against real git.
//!
//! The unit tests next to [`platitude_core::eol`] hold the byte shapes; these
//! prove the shapes are the ones git actually prints, through the same
//! `file_diff_raw` the panes use. Written before the UI existed, because a
//! reading built on an assumed patch shape and a test built on the same
//! assumption go green together while the app says nothing.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use platitude_core::details::{self, DiffTarget};
use platitude_core::eol::{self, Eol, Reading};
use platitude_core::{GitExecutor, Oid};
use tokio_util::sync::CancellationToken;

async fn reading(repo: &TestRepo, target: DiffTarget) -> Reading {
    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();
    let raw = details::file_diff_raw(&executor, &repo.path, &target, &cancel)
        .await
        .expect("run diff");
    eol::read_one(&raw)
}

#[tokio::test]
async fn a_worktree_flipped_to_crlf_reads_as_a_flip() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\ntwo\nthree\n", "root");
    repo.write_file("f.txt", "one\r\ntwo\r\nthree\r\n");

    assert_eq!(
        reading(
            &repo,
            DiffTarget::Unstaged {
                path: "f.txt".to_string()
            }
        )
        .await,
        Reading::Flipped {
            from: Eol::Lf,
            to: Eol::Crlf
        }
    );
}

#[tokio::test]
async fn one_crlf_line_dropped_into_an_lf_file_reads_as_mixed() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "a\nb\nc\nd\ne\nf\ng\nh\n", "root");
    repo.write_file("f.txt", "a\nb\nc\nd\r\ne\nf\ng\nh\n");

    assert_eq!(
        reading(
            &repo,
            DiffTarget::Unstaged {
                path: "f.txt".to_string()
            }
        )
        .await,
        Reading::Mixed {
            lines: 1,
            added: Eol::Crlf,
            file: Eol::Lf
        }
    );
}

#[tokio::test]
async fn a_flip_that_is_already_staged_reads_the_same() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\ntwo\n", "root");
    repo.write_file("f.txt", "one\r\ntwo\r\n");
    repo.git(&["add", "--", "f.txt"]);

    assert_eq!(
        reading(
            &repo,
            DiffTarget::Staged {
                path: "f.txt".to_string(),
                orig_path: None,
            }
        )
        .await,
        Reading::Flipped {
            from: Eol::Lf,
            to: Eol::Crlf
        }
    );
}

#[tokio::test]
async fn an_untracked_file_reads_the_ending_it_arrived_with() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\n", "root");
    repo.write_file("fresh.txt", "new\r\nfile\r\n");

    assert_eq!(
        reading(
            &repo,
            DiffTarget::Untracked {
                path: "fresh.txt".to_string()
            }
        )
        .await,
        Reading::NewFile { eol: Eol::Crlf }
    );
}

#[tokio::test]
async fn a_file_that_held_no_ending_reports_its_first() {
    let mut repo = TestRepo::init();
    repo.commit_file("bare.txt", "no-newline-here", "root");
    repo.write_file("bare.txt", "no-newline-here\n");

    assert_eq!(
        reading(
            &repo,
            DiffTarget::Unstaged {
                path: "bare.txt".to_string()
            }
        )
        .await,
        Reading::FirstEnding { eol: Eol::Lf }
    );
}

#[tokio::test]
async fn dropping_the_trailing_newline_is_not_an_ending_change() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\ntwo\n", "root");
    repo.write_file("f.txt", "one\ntwo");

    assert_eq!(
        reading(
            &repo,
            DiffTarget::Unstaged {
                path: "f.txt".to_string()
            }
        )
        .await,
        Reading::Quiet
    );
}

#[tokio::test]
async fn a_flip_that_was_committed_reads_out_of_history() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "one\ntwo\n", "root");
    let flipped = repo.commit_file("f.txt", "one\r\ntwo\r\n", "endings");
    let parent = repo.git(&["rev-parse", "HEAD^"]);

    assert_eq!(
        reading(
            &repo,
            DiffTarget::Commit {
                oid: Oid::from_hex_str(&flipped).unwrap(),
                parent: Some(Oid::from_hex_str(&parent).unwrap()),
                path: "f.txt".to_string(),
                orig_path: None,
            }
        )
        .await,
        Reading::Flipped {
            from: Eol::Lf,
            to: Eol::Crlf
        }
    );
}

#[tokio::test]
async fn git_converting_on_the_way_in_leaves_nothing_to_warn_about() {
    // `core.autocrlf=true` — the Windows default. git compares in index
    // space, so a worktree turned to CRLF produces an empty diff (status
    // still calls the file modified) and there is nothing to report: the
    // conversion is git's, working as configured.
    let mut repo = TestRepo::init_autocrlf();
    repo.commit_file("f.txt", "one\ntwo\n", "root");
    repo.write_file("f.txt", "one\r\ntwo\r\n");
    assert_eq!(
        reading(
            &repo,
            DiffTarget::Unstaged {
                path: "f.txt".to_string()
            }
        )
        .await,
        Reading::Quiet
    );

    // An untracked CRLF file comes through `--no-index` with its CRs
    // already gone, so the reading names LF — the ending the file will
    // actually be stored with. Whether that is worth saying is the
    // baseline's call, and on this setting the baseline declines: git
    // normalises every new file, so there is nothing to compare against.
    repo.write_file("fresh.txt", "new\r\nfile\r\n");
    assert_eq!(
        reading(
            &repo,
            DiffTarget::Untracked {
                path: "fresh.txt".to_string()
            }
        )
        .await,
        Reading::NewFile { eol: Eol::Lf }
    );
}

#[tokio::test]
async fn an_index_blob_that_already_holds_cr_keeps_its_endings() {
    // The one place `autocrlf=true` converts nothing: git leaves a blob
    // whose stored bytes already carry CR alone, so the CRs in the patch are
    // real data. Lines added in the file's own ending are not a notice.
    let mut repo = TestRepo::init_autocrlf();
    repo.write_file("kept.txt", "keep\r\nthese\r\n");
    repo.git(&["-c", "core.autocrlf=false", "add", "--", "kept.txt"]);
    repo.git(&["commit", "-m", "CR in the index"]);
    repo.write_file("kept.txt", "keep\r\nthese\r\nmore\r\n");

    assert_eq!(
        reading(
            &repo,
            DiffTarget::Unstaged {
                path: "kept.txt".to_string()
            }
        )
        .await,
        Reading::Quiet
    );

    // …and an LF line added to that CRLF file is still worth saying.
    repo.write_file("kept.txt", "keep\r\nthese\r\nmore\n");
    assert_eq!(
        reading(
            &repo,
            DiffTarget::Unstaged {
                path: "kept.txt".to_string()
            }
        )
        .await,
        Reading::Mixed {
            lines: 1,
            added: Eol::Lf,
            file: Eol::Crlf
        }
    );
}

#[tokio::test]
async fn a_whole_tree_diff_names_every_file_it_has_something_about() {
    // The pass that marks pending files reads one multi-file diff rather
    // than one command per path; the paths it returns have to match the
    // ones status reports.
    let mut repo = TestRepo::init();
    repo.commit_file("a.txt", "one\ntwo\n", "root");
    repo.commit_file("deep/dir/b.txt", "one\ntwo\n", "more");
    repo.commit_file("c.txt", "one\ntwo\n", "and more");
    repo.write_file("a.txt", "one\r\ntwo\r\n");
    repo.write_file("deep/dir/b.txt", "one\ntwo\nthree\r\n");
    repo.write_file("c.txt", "one\ntwo\nthree\n");

    let raw = repo.git_raw(&["diff", "--no-ext-diff"]);
    let seen = eol::read(&raw);
    let paths: Vec<&str> = seen.iter().map(|s| s.path.as_str()).collect();
    assert_eq!(paths, vec!["a.txt", "deep/dir/b.txt"]);
    assert_eq!(
        seen[0].reading,
        Reading::Flipped {
            from: Eol::Lf,
            to: Eol::Crlf
        }
    );
    assert_eq!(
        seen[1].reading,
        Reading::Mixed {
            lines: 1,
            added: Eol::Crlf,
            file: Eol::Lf
        }
    );
}
