//! Every case here drives `wanted`, so they sit together rather than
//! beside the model whose slots none of them names.

use std::sync::Arc;

use platitude_core::details::DiffTarget;

use crate::encode::diff_key;
use crate::hub::DiffMsg;

use super::diff::wanted;

fn target(path: &str) -> DiffTarget {
    DiffTarget::Unstaged {
        path: path.to_string(),
    }
}

fn rows(path: &str, fingerprint: u64) -> DiffMsg {
    DiffMsg::Loaded {
        target: target(path),
        patches: Arc::new(Vec::new()),
        preview: None,
        fingerprint,
        endings: None,
    }
}

fn colours(path: &str) -> DiffMsg {
    DiffMsg::Coloured {
        target: target(path),
        colors: platitude_core::highlight::DiffColors::default(),
        settled: true,
    }
}

fn key(path: &str) -> String {
    diff_key(&target(path))
}

fn paths(msgs: &[DiffMsg]) -> Vec<String> {
    msgs.iter().map(|m| diff_key(m.target())).collect()
}

#[test]
fn the_open_file_is_found_under_a_slower_earlier_read() {
    // What the pane sees when a coloured file asked for first lands
    // after the plain one asked for second: both are in hand at once,
    // and only one of them is the file on screen. Taking the last
    // arrival would leave the reader looking at the previous file.
    let picked = wanted(
        vec![rows("plain.txt", 0), rows("slow.rs", 0)],
        &key("plain.txt"),
    );
    assert_eq!(paths(&picked), vec![key("plain.txt")]);
}

#[test]
fn the_rows_and_the_colours_behind_them_are_both_taken() {
    // Both halves of one diff, waiting together because the slot was
    // woken once for each and ran once.
    let picked = wanted(vec![rows("a.rs", 0), colours("a.rs")], &key("a.rs"));
    assert_eq!(picked.len(), 2);
    assert!(matches!(picked[0], DiffMsg::Loaded { .. }));
    assert!(matches!(picked[1], DiffMsg::Coloured { .. }));
}

#[test]
fn the_latest_answer_for_the_same_file_comes_last() {
    // A partial write re-reads the file it just changed, so the same
    // key arrives twice and the one with the write in it is second.
    // Applied in order, that is the one left on screen.
    let picked = wanted(vec![rows("a.rs", 1), rows("a.rs", 7)], &key("a.rs"));
    let last = picked.last().expect("two were kept");
    assert!(matches!(last, DiffMsg::Loaded { fingerprint: 7, .. }));
}

#[test]
fn answers_for_files_nobody_is_on_are_dropped() {
    assert!(wanted(vec![rows("a.rs", 0), colours("b.rs")], &key("c.rs")).is_empty());
    assert!(wanted(Vec::new(), &key("a.rs")).is_empty());
}
