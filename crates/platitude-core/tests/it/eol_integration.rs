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
use platitude_core::eol::{self, Baseline, Eol, Reading, Ruling, Scope};
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

async fn baseline(repo: &TestRepo, path: &str) -> Option<Baseline> {
    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();
    eol::baseline(&executor, &repo.path, path, &cancel)
        .await
        .expect("resolve baseline")
}

async fn ruling(repo: &TestRepo, path: &str) -> Ruling {
    let executor = GitExecutor::new();
    let cancel = CancellationToken::new();
    eol::ruling(&executor, &repo.path, path, &cancel)
        .await
        .expect("resolve ruling")
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

// --------------------------------------------------------------- baselines

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
async fn the_same_extension_elsewhere_widens_what_the_notice_may_claim() {
    let mut repo = TestRepo::init();
    for dir in ["one", "two", "three"] {
        repo.commit_file(&format!("{dir}/f.kt"), "fun x() {}\r\n", "code");
    }
    // Nothing sits beside the new file, so the sample had to leave the
    // directory and the notice may not say "here".
    assert_eq!(
        baseline(&repo, "fresh/new.kt").await,
        Some(Baseline {
            eol: Eol::Crlf,
            scope: Scope::Ext("kt".to_string())
        })
    );
}

#[tokio::test]
async fn a_file_with_no_extension_is_measured_against_the_repository() {
    let mut repo = TestRepo::init();
    for name in ["a.kt", "b.md", "c.txt"] {
        repo.commit_file(name, "line\n", "content");
    }
    assert_eq!(
        baseline(&repo, "Makefile").await,
        Some(Baseline {
            eol: Eol::Lf,
            scope: Scope::Repo
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
async fn too_few_files_to_read_means_no_answer_at_all() {
    // Two neighbours are not a house style, and answering from them would
    // be the app inventing one.
    let mut repo = TestRepo::init();
    repo.commit_file("only.kt", "fun x() {}\n", "code");
    repo.commit_file("second.kt", "fun y() {}\n", "code");
    assert_eq!(baseline(&repo, "third.kt").await, None);
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
