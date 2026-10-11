//! Line endings across a partial discard. Where git converts the file, the
//! diff is in index space, and git's apply reads the whole file back
//! through the conversion and writes it out again, so left alone it ends
//! every line the way the settings say — lines nobody asked about included.
//! A file that ends its lines one way keeps that way.

use crate::support::exec::env;
use crate::support::stage::{fp, indexed};
use crate::support::{TestRepo, info};
use platitude_core::details::DiffTarget;
use platitude_core::patch::HunkSelect;
use platitude_core::stage;

/// Twenty numbered lines, each ended by `eol`, with lines 2 and 18 edited
/// where `edited` says so — two hunks.
fn numbered(eol: &str, edited: [bool; 2]) -> String {
    (1..=20)
        .map(|n| {
            let mark = match n {
                2 if edited[0] => " EDITED",
                18 if edited[1] => " EDITED",
                _ => "",
            };
            format!("line {n}{mark}{eol}")
        })
        .collect()
}

/// Throws away the first hunk of `f.txt`'s unstaged diff.
async fn discard_the_first_hunk(repo: &TestRepo) {
    let (exec, cancel) = env();
    let repo_info = info(repo).await;
    let target = DiffTarget::Unstaged {
        path: "f.txt".into(),
    };
    stage::discard_partial(
        &exec,
        &repo_info,
        &target,
        &[HunkSelect::whole(0)],
        fp(&repo_info, &target).await,
        &cancel,
    )
    .await
    .expect("discard the first hunk");
}

fn on_disk(repo: &TestRepo) -> String {
    String::from_utf8(std::fs::read(repo.path.join("f.txt")).expect("read f.txt")).expect("utf-8")
}

/// `core.autocrlf=true` would write CRLF, but an editor (or a formatter)
/// saved the file with LF. Git compares in index space, so nothing shows
/// in the diff; a discard that rewrote it would turn all twenty lines.
#[tokio::test]
async fn an_lf_file_under_autocrlf_true_keeps_lf() {
    let mut repo = TestRepo::init_autocrlf();
    repo.commit_file("f.txt", &numbered("\n", [false; 2]), "root");
    repo.write_file("f.txt", &numbered("\n", [true; 2]));

    discard_the_first_hunk(&repo).await;

    assert_eq!(on_disk(&repo), numbered("\n", [false, true]));
}

/// The mirror: `core.autocrlf=input` writes LF, and the file on disk is
/// CRLF. The lines brought back end like the rest.
#[tokio::test]
async fn a_crlf_file_under_autocrlf_input_keeps_crlf() {
    let mut repo = TestRepo::init();
    repo.git(&["config", "core.autocrlf", "input"]);
    repo.commit_file("f.txt", &numbered("\n", [false; 2]), "root");
    repo.write_file("f.txt", &numbered("\r\n", [true; 2]));

    discard_the_first_hunk(&repo).await;

    assert_eq!(on_disk(&repo), numbered("\r\n", [false, true]));
    assert_eq!(
        indexed(&mut repo, "f.txt"),
        numbered("\n", [false; 2]).trim_end(),
        "the index is untouched"
    );
}

/// Where git converts nothing (`core.autocrlf=false`, no attributes) the
/// diff is of the bytes as they are: an editor that saved the file CRLF
/// changed every line, and throwing that hunk away brings the committed LF
/// back. A file this end reads as CRLF throughout is not one to write CRLF.
#[tokio::test]
async fn a_crlf_file_git_does_not_convert_comes_back_as_committed() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", &numbered("\n", [false; 2]), "root");
    repo.write_file("f.txt", &numbered("\r\n", [true, false]));

    discard_the_first_hunk(&repo).await;

    assert_eq!(on_disk(&repo), numbered("\n", [false; 2]));
}

/// A last line without a newline has no ending to tell by. Under
/// `diff.context=0` it can be the only line a hunk keeps, in a CRLF file git
/// compared byte for byte; read as converted, the discard would turn the
/// lines above it LF.
#[tokio::test]
async fn an_unterminated_last_line_says_nothing_about_the_endings() {
    let mut repo = TestRepo::init();
    repo.commit_file("f.txt", "a\r\nb\r\nc\r\n", "root");
    repo.git(&["config", "diff.context", "0"]);
    repo.write_file("f.txt", "a\r\nb\r\nZ");

    discard_the_first_hunk(&repo).await;

    assert_eq!(on_disk(&repo), "a\r\nb\r\nc\r\n");
}

/// The checkout's own shape under `core.autocrlf=true`: CRLF on disk, LF
/// in the index. Git's write already agrees with the file.
#[tokio::test]
async fn a_crlf_file_under_autocrlf_true_keeps_crlf() {
    let mut repo = TestRepo::init_autocrlf();
    repo.commit_file("f.txt", &numbered("\r\n", [false; 2]), "root");
    repo.write_file("f.txt", &numbered("\r\n", [true; 2]));

    discard_the_first_hunk(&repo).await;

    assert_eq!(on_disk(&repo), numbered("\r\n", [false, true]));
}
