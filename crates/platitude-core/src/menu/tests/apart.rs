//! The menus whose subject no [`crate::menu::Shape`] makes: the working
//! tree's chosen files, a plan's step, a remote's own row.

use crate::menu::{Below, Can, Chosen, On, Row, Why, lines};
use crate::stash::StashStanding;

/// What every row of the menu opened on `on` says, in drawing order, in an
/// open tab.
fn said(on: &On<'_>) -> Vec<(Row, Can)> {
    lines(true, on)
        .expect("a menu")
        .iter()
        .map(|line| (line.row, line.can))
        .collect()
}

fn files(chosen: Chosen, stash: StashStanding) -> On<'static> {
    On::Files { chosen, stash }
}

/// Each section's rows stand where the choice reaches it, and act on its
/// own files: a conflicted file has no `Discard` (git restores no unmerged
/// path), and a choice of both has both sections' rows.
#[test]
fn a_choice_of_files_has_the_rows_of_every_section_it_reaches() {
    let rows = |chosen| -> Vec<Row> {
        said(&files(chosen, StashStanding::Ready))
            .into_iter()
            .map(|(row, _)| row)
            .collect()
    };
    let resolve = [Row::KeepOurs, Row::TakeTheirs, Row::MergeTool];
    assert_eq!(
        rows(Chosen::Conflicts),
        [resolve.as_slice(), &[Row::StashFiles]].concat()
    );
    assert_eq!(rows(Chosen::Changes), [Row::StashFiles, Row::Discard]);
    assert_eq!(
        rows(Chosen::Both),
        [resolve.as_slice(), &[Row::StashFiles, Row::Discard]].concat()
    );
    assert_eq!(Chosen::of(false, false), None, "no file, no menu");
}

/// `stash` greys for the tree git refuses one of — no commit yet, or an
/// unmerged path anywhere —, the rest stand on nothing but the files.
#[test]
fn a_files_stash_greys_for_the_tree_git_refuses() {
    let stash = |standing| {
        said(&files(Chosen::Both, standing))
            .into_iter()
            .find(|(row, _)| *row == Row::StashFiles)
            .map(|(_, can)| can)
    };
    assert_eq!(
        stash(StashStanding::Unborn),
        Some(Can::No(Why::NoCommitsYet))
    );
    assert_eq!(
        stash(StashStanding::Conflicts),
        Some(Can::No(Why::ConflictsHere))
    );
    assert_eq!(stash(StashStanding::Ready), Some(Can::Yes));
    for (row, can) in said(&files(Chosen::Both, StashStanding::Conflicts)) {
        if row != Row::StashFiles {
            assert_eq!(can, Can::Yes, "{row:?}");
        }
    }
}

/// A tab still opening greys every row of every subject under the one
/// reason.
#[test]
fn every_subject_greys_while_the_tab_opens() {
    for on in [
        files(Chosen::Both, StashStanding::Ready),
        On::Step(Below::Landing),
        On::Remote { marked: false },
    ] {
        for line in lines(false, &on).expect("a menu") {
            assert_eq!(line.can, Can::No(Why::Opening), "{on:?} {:?}", line.row);
        }
    }
}

/// A fold lands in the nearest kept step below: with every step below
/// dropped it greys, and the oldest step, with none below, has no fold.
#[test]
fn a_fold_greys_with_every_step_below_dropped() {
    let keep = [
        (Row::StepPick, Can::Yes),
        (Row::StepReword, Can::Yes),
        (Row::StepEdit, Can::Yes),
    ];
    let drop = (Row::StepDrop, Can::Yes);
    let fold = |can| [(Row::StepSquash, can), (Row::StepFixup, can)];
    assert_eq!(
        said(&On::Step(Below::Landing)),
        [keep.as_slice(), &fold(Can::Yes), &[drop]].concat()
    );
    assert_eq!(
        said(&On::Step(Below::Dropped)),
        [
            keep.as_slice(),
            &fold(Can::No(Why::NothingToFoldInto)),
            &[drop]
        ]
        .concat()
    );
    assert_eq!(
        said(&On::Step(Below::Nothing)),
        [keep.as_slice(), &[drop]].concat()
    );
}

/// The mark stands greyed on the remote both keys already name, and live
/// on any other — one key's included, to finish the mark.
#[test]
fn the_origin_mark_greys_on_the_remote_both_keys_name() {
    assert_eq!(
        said(&On::Remote { marked: false }),
        [(Row::RemoteUrl, Can::Yes), (Row::MarkOrigin, Can::Yes)]
    );
    assert_eq!(
        said(&On::Remote { marked: true }),
        [
            (Row::RemoteUrl, Can::Yes),
            (Row::MarkOrigin, Can::No(Why::AlreadyOrigin))
        ]
    );
}
