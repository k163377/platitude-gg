//! Line-ending readings against real git: what the index lists around a
//! new file, and what configuration and `.gitattributes` decide about a
//! path. The byte shapes are read by the unit tests next to
//! [`platitude_core::eol`]; that git still prints them is in [`periodic`].

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
// The picking rule is held by the unit tests beside
// `platitude_core::eol::sample`. Here only what git answers: that the
// listing is the index, the endings are what `ls-files --eol` prints, and a
// file with no single ending declines to vote.

#[tokio::test]
async fn neighbours_in_the_same_directory_answer_for_a_new_file() {
    let mut repo = TestRepo::init();
    for name in ["a.kt", "b.kt", "c.kt"] {
        repo.commit_file(&format!("src/{name}"), "fun x() {}\n", "code");
    }
    // CRLF elsewhere: a baseline that reached past the directory would
    // answer differently.
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
    assert_eq!(ruling(&repo, "src/new.md").await, Ruling::Open);
}

#[tokio::test]
async fn a_path_marked_as_not_text_is_left_alone() {
    // The only exclusion the app honours: generated output and test data
    // are `.gitattributes`' business.
    let mut repo = TestRepo::init();
    repo.commit_file(".gitattributes", "*.bin -text\n", "attributes");
    repo.commit_file("blob.bin", "content\n", "data");
    assert_eq!(ruling(&repo, "blob.bin").await, Ruling::NotText);
    assert_eq!(baseline(&repo, "blob.bin").await, None);
}

/// That git still prints the patch shapes the unit tests beside
/// [`platitude_core::eol`] read (e.g.
/// `read_tests::a_file_with_no_ending_at_all_reports_its_first`,
/// `read_tests::losing_the_final_newline_is_not_an_ending_change`). That
/// moves only with git, so the full gate (`-- --ignored ::periodic::`)
/// runs these, not every change.
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
        // git compares in index space, so a working tree turned to CRLF diffs
        // empty (status still calls it modified): the conversion is git's,
        // working as configured.
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
        // gone, so the reading names LF, the ending it will be stored with.
        // Whether to say so is the baseline's call (it declines here).
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
        // `autocrlf=true` leaves a blob that already stores CR alone, so the
        // CRs in the patch are real data; lines added in the file's own
        // ending are not a notice.
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
        // The pending-marks pass reads one multi-file diff, not one command
        // per path; its paths have to match the ones status reports.
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
