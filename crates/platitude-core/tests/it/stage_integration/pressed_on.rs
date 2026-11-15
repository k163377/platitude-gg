//! Parts pressed one after another on the same diff, the later ones before
//! the earlier ones' writes have run (`stage::PressedOn`), and the rule that
//! places them (`parts::after`) held against the diff git prints after each.

use std::sync::Arc;

use crate::support::exec::env;
use crate::support::stage::indexed;
use crate::support::{TestRepo, info};
use platitude_core::details::{self, DiffTarget};
use platitude_core::parse::diff::{FilePatch, parse_patch};
use platitude_core::parts::{self, Moves, Part};
use platitude_core::patch::HunkSelect;
use platitude_core::repo::RepoInfo;
use platitude_core::stage::{self, PressedOn};

fn unstaged() -> DiffTarget {
    DiffTarget::Unstaged {
        path: "f.txt".into(),
    }
}

/// The diff of `target` as read now: its fingerprint and its parse.
async fn read(info: &RepoInfo, target: &DiffTarget) -> (u64, Arc<Vec<FilePatch>>) {
    let (exec, cancel) = env();
    let raw = details::file_diff_raw(&exec, &info.workdir, target, &cancel)
        .await
        .expect("diff");
    (details::fingerprint(&raw), Arc::new(parse_patch(&raw)))
}

/// A file of twenty lines with two well-separated edits: two hunks.
fn two_hunks() -> TestRepo {
    let mut repo = TestRepo::init();
    let base: String = (1..=20).map(|n| format!("line {n}\n")).collect();
    repo.commit_file("f.txt", &base, "root");
    let edited = base
        .replace("line 2\n", "line 2 EDITED\n")
        .replace("line 18\n", "line 18 EDITED\n");
    repo.write_file("f.txt", &edited);
    repo
}

/// Every shape the rule lays out is the one git prints after the write: a
/// hunk staged out of two (the old side below it moves), a line staged into
/// context with its hunk's end cut to three lines, a removal unstaged back
/// into context, and a hunk discarded.
#[tokio::test]
async fn what_the_rule_lays_out_is_what_git_prints_after_the_part() {
    let (exec, cancel) = env();
    // (file in HEAD, file on disk, the part, staged first?)
    let base: String = (1..=20).map(|n| format!("line {n}\n")).collect();
    let grown = base.replace("line 5\n", "line 5\nX\nline 5b\nY\n");
    let cases = [
        (
            base.clone(),
            base.replace("line 2\n", "two\nmore\n")
                .replace("line 18\n", "eighteen\n"),
            Part {
                hunk: 0,
                line: None,
                moves: Moves::Old,
            },
        ),
        (
            base.clone(),
            grown.clone(),
            Part {
                hunk: 0,
                line: Some(4),
                moves: Moves::Old,
            },
        ),
        (
            base.clone(),
            base.replace("line 2\n", "two\nmore\n")
                .replace("line 18\n", "eighteen\n"),
            Part {
                hunk: 0,
                line: None,
                moves: Moves::New,
            },
        ),
    ];
    for (head, disk, part) in cases {
        let mut repo = TestRepo::init();
        repo.commit_file("f.txt", &head, "root");
        repo.write_file("f.txt", &disk);
        let info = info(&repo).await;
        let target = unstaged();
        let (seen, before) = read(&info, &target).await;
        let select = match part.line {
            Some(line) => HunkSelect::lines(part.hunk, [line]),
            None => HunkSelect::whole(part.hunk),
        };
        let wrote = match part.moves {
            Moves::Old => {
                stage::apply_partial(&exec, &info, &target, &[select], seen, &cancel).await
            }
            Moves::New => {
                stage::discard_partial(&exec, &info, &target, &[select], seen, &cancel).await
            }
        };
        wrote.expect("the part lands");
        let (_, now) = read(&info, &target).await;
        let after = parts::after(&before, &[part]);
        assert!(after.exact, "{part:?}: laid out");
        assert!(
            parts::same_lines(&after.patches, &now),
            "{part:?}: laid out {:#?}\ngit prints {:#?}",
            after.patches,
            now
        );
    }
}

/// The second hunk, pressed on the diff with the first already taken out —
/// its place in that drawing, not in the bytes read — lands behind the
/// first: both are staged.
#[tokio::test]
async fn a_hunk_pressed_behind_another_lands_where_the_drawing_showed_it() {
    let mut repo = two_hunks();
    let (exec, cancel) = env();
    let info = info(&repo).await;
    let target = unstaged();
    let (seen, read_at_press) = read(&info, &target).await;
    let first = Part {
        hunk: 0,
        line: None,
        moves: Moves::Old,
    };
    stage::apply_partial(
        &exec,
        &info,
        &target,
        &[HunkSelect::whole(0)],
        seen,
        &cancel,
    )
    .await
    .expect("the first hunk");
    // The drawing had one hunk left: the second, now hunk 0.
    let on = PressedOn {
        seen,
        read: Arc::clone(&read_at_press),
        ahead: vec![first],
    };
    stage::apply_partial_on(&exec, &info, &target, &[HunkSelect::whole(0)], &on, &cancel)
        .await
        .expect("the second hunk, placed behind the first");
    let staged = indexed(&mut repo, "f.txt");
    assert!(staged.contains("line 2 EDITED") && staged.contains("line 18 EDITED"));
}

/// The part in front was refused (the file moved under it): the drawing the
/// second was pressed on is not what git prints, and nothing is written.
#[tokio::test]
async fn a_part_behind_one_git_refused_is_refused_and_writes_nothing() {
    let mut repo = two_hunks();
    let (exec, cancel) = env();
    let info = info(&repo).await;
    let target = unstaged();
    let (seen, read_at_press) = read(&info, &target).await;
    let on = PressedOn {
        seen,
        read: read_at_press,
        ahead: vec![Part {
            hunk: 0,
            line: None,
            moves: Moves::Old,
        }],
    };
    // Nothing ran in front: the bytes are still the ones read.
    let refused =
        stage::apply_partial_on(&exec, &info, &target, &[HunkSelect::whole(0)], &on, &cancel).await;
    assert!(refused.is_err(), "placed in a drawing git does not print");
    assert!(
        !indexed(&mut repo, "f.txt").contains("EDITED"),
        "the index is HEAD's"
    );
}

/// A hunk discarded behind a hunk staged, both pressed on the same bytes:
/// the first goes into the index, the second off the disk.
#[tokio::test]
async fn a_discard_pressed_behind_a_stage_takes_its_own_hunk() {
    let mut repo = two_hunks();
    let (exec, cancel) = env();
    let info = info(&repo).await;
    let target = unstaged();
    let (seen, read_at_press) = read(&info, &target).await;
    stage::apply_partial(
        &exec,
        &info,
        &target,
        &[HunkSelect::whole(0)],
        seen,
        &cancel,
    )
    .await
    .expect("the first hunk staged");
    let on = PressedOn {
        seen,
        read: read_at_press,
        ahead: vec![Part {
            hunk: 0,
            line: None,
            moves: Moves::Old,
        }],
    };
    stage::discard_partial_on(&exec, &info, &target, &[HunkSelect::whole(0)], &on, &cancel)
        .await
        .expect("the second hunk discarded");
    let disk = std::fs::read_to_string(repo.path.join("f.txt")).expect("read");
    assert!(disk.contains("line 2 EDITED") && !disk.contains("line 18 EDITED"));
    assert!(indexed(&mut repo, "f.txt").contains("line 2 EDITED"));
}
