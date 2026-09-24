//! Line-ending readings against real git.
//!
//! The unit tests next to [`platitude_core::eol`] read the byte shapes git
//! was measured to print. The pre-merge part here holds what only git can
//! answer before a notice is shown: what the index lists around a new file,
//! and what the configuration and `.gitattributes` decide about a path. The
//! periodic part records that git still prints those shapes — through the
//! same `file_diff_raw` the panes use, and in the whole-tree diff the
//! pending marks read — because a reading built on an assumed patch shape
//! and a test built on the same assumption go green together while the app
//! says nothing.

// Test scaffolding may panic; `allow-*-in-tests` only covers `#[test]` fns.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use crate::support::TestRepo;
use crate::support::exec::env;
use platitude_core::details::{self, DiffTarget};
use platitude_core::eol::{self, Baseline, Eol, Reading, Ruling, Scope};

async fn baseline(repo: &TestRepo, path: &str) -> Option<Baseline> {
    let (executor, cancel) = env();
    eol::baseline(&executor, &repo.path, path, &cancel)
        .await
        .expect("resolve baseline")
}

async fn ruling(repo: &TestRepo, path: &str) -> Ruling {
    let (executor, cancel) = env();
    eol::ruling(&executor, &repo.path, path, &cancel)
        .await
        .expect("resolve ruling")
}

// --------------------------------------------------------------- baselines
//
// Which neighbours are picked, in what order, and what their readings come
// to are a rule over two listings and a handful of endings, and are held in
// the unit tests beside `platitude_core::eol::sample` — an arrangement there
// costs nothing rather than a repository apiece. **What is left here is what
// only git can answer**: that the listing really is the index and the
// endings really are what `ls-files --eol` prints, and that a file with no
// single ending to give declines to vote rather than voting for one.

#[tokio::test]
async fn neighbours_in_the_same_directory_answer_for_a_new_file() {
    let mut repo = TestRepo::init();
    for name in ["a.kt", "b.kt", "c.kt"] {
        repo.commit_file(&format!("src/{name}"), "fun x() {}\n", "code");
    }
    // Elsewhere in the repository the house style is the other one, so a
    // baseline that reached past the directory would answer differently.
    repo.commit_file("docs/note.md", "text\r\n", "docs");

    assert_eq!(
        baseline(&repo, "src/new.kt").await,
        Some(Baseline {
            eol: Eol::Lf,
            scope: Scope::Here("kt".to_string())
        })
    );
}

#[tokio::test]
async fn a_neighbour_that_cannot_vote_is_replaced_rather_than_counted() {
    // Mixed has no single answer to give and a file with no ending at all
    // has nothing to say; both have to drop out and be made up for.
    let mut repo = TestRepo::init();
    repo.commit_file("src/mixed.kt", "one\r\ntwo\nthree\r\n", "mixed");
    repo.commit_file("src/bare.kt", "no ending", "bare");
    for name in ["a.kt", "b.kt", "c.kt"] {
        repo.commit_file(&format!("src/{name}"), "fun x() {}\n", "code");
    }
    assert_eq!(
        baseline(&repo, "src/new.kt").await,
        Some(Baseline {
            eol: Eol::Lf,
            scope: Scope::Here("kt".to_string())
        })
    );
}

#[tokio::test]
async fn git_deciding_the_endings_itself_skips_the_sample() {
    let mut repo = TestRepo::init_autocrlf();
    for name in ["a.kt", "b.kt", "c.kt", "d.kt"] {
        repo.commit_file(&format!("src/{name}"), "fun x() {}\n", "code");
    }
    // `autocrlf=true` normalises every new file on the way in, so there is
    // nothing for a new file to disagree with.
    assert_eq!(ruling(&repo, "src/new.kt").await, Ruling::Normalised);
    assert_eq!(baseline(&repo, "src/new.kt").await, None);
}

#[tokio::test]
async fn attributes_that_settle_the_ending_skip_the_sample_too() {
    let mut repo = TestRepo::init();
    repo.commit_file(".gitattributes", "*.kt text eol=lf\n", "attributes");
    for name in ["a.kt", "b.kt", "c.kt", "d.kt"] {
        repo.commit_file(&format!("src/{name}"), "fun x() {}\n", "code");
    }
    assert_eq!(ruling(&repo, "src/new.kt").await, Ruling::Normalised);
    assert_eq!(baseline(&repo, "src/new.kt").await, None);
    // A path the attributes say nothing about is still open.
    assert_eq!(ruling(&repo, "src/new.md").await, Ruling::Open);
}

#[tokio::test]
async fn a_path_marked_as_not_text_is_left_alone() {
    // The only exclusion the app honours, and it is git's word for it —
    // generated output and test data are `.gitattributes`' business.
    let mut repo = TestRepo::init();
    repo.commit_file(".gitattributes", "*.bin -text\n", "attributes");
    repo.commit_file("blob.bin", "content\n", "data");
    assert_eq!(ruling(&repo, "blob.bin").await, Ruling::NotText);
    assert_eq!(baseline(&repo, "blob.bin").await, None);
}

/// **What the pre-merge run leaves out**: that git still prints the patch
/// shapes the unit tests beside [`platitude_core::eol`] read — one diff
/// naming every file it has something about, what `core.autocrlf=true`
/// does and does not convert on the way to a patch, and the
/// `\ No newline at end of file` it marks a gained or a lost final newline
/// with (`read_tests::a_file_with_no_ending_at_all_reports_its_first`,
/// `read_tests::losing_the_final_newline_is_not_an_ending_change`). How
/// those bytes read is the unit tests', and what git prints moves only with
/// git, so the full gate (`-- --ignored ::periodic::`) runs these rather
/// than every change.
mod periodic {
    use super::*;

    async fn reading(repo: &TestRepo, target: DiffTarget) -> Reading {
        let (executor, cancel) = env();
        let raw = details::file_diff_raw(&executor, &repo.path, &target, &cancel)
            .await
            .expect("run diff");
        eol::read_one(&raw)
    }

    #[tokio::test]
    #[ignore = "what core.autocrlf converts on the way to a patch: not worth the pre-merge run"]
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
    #[ignore = "a CR blob core.autocrlf leaves alone: not worth the pre-merge run"]
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
    #[ignore = "the multi-file patch shape git prints: not worth the pre-merge run"]
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

    #[tokio::test]
    #[ignore = "how git marks a file that held no ending: not worth the pre-merge run"]
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
    #[ignore = "how git marks a dropped trailing newline: not worth the pre-merge run"]
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
}
