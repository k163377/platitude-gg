//! Laying a walked graph out again: the rows the readings put there
//! move, and nothing the walk found does.

use std::sync::Arc;

use super::relay::{Standing, lay, still_where_the_listing_says};
use super::{Carried, LogRow};
use crate::graph::GraphBuilder;
use crate::oid::Oid;
use crate::status::Kinds;

fn oid(n: u8) -> Oid {
    let hex = format!("{n:02x}").repeat(20);
    Oid::from_hex_str(&hex).unwrap()
}

/// One walked row, as a pass leaves it: what it says is its own, and
/// what the laying rewrites is only where it sits.
fn walked(id: u8, parents: &[u8], builder: &mut GraphBuilder) -> LogRow {
    let mine = oid(id);
    let parents: Box<[Oid]> = parents.iter().map(|p| oid(*p)).collect();
    let g = builder.push_ids(&mine, &parents, false);
    LogRow {
        row: g.row,
        oid_hex: mine.to_hex(),
        short_sha: mine.short_hex(8),
        author: "Test".to_string(),
        author_email: "test@example.com".to_string(),
        co_authors: Vec::new(),
        time: 1_700_000_000 + i64::from(id),
        subject: format!("commit {id:02x}"),
        body: String::new(),
        node_lane: g.node_lane,
        node_color: g.node_color,
        width: g.width,
        segments: g.segments,
        labels: Vec::new(),
        stash_ref: String::new(),
        published: false,
        carried: None,
        parents,
    }
}

/// A history the walk found, with nothing of any reading in it.
fn history() -> Vec<LogRow> {
    let mut b = GraphBuilder::new();
    vec![
        walked(3, &[2], &mut b),
        walked(2, &[1], &mut b),
        walked(1, &[], &mut b),
    ]
}

fn where_they_sit(rows: &[LogRow]) -> Vec<(String, u16, u8)> {
    rows.iter()
        .map(|r| (r.oid_hex.clone(), r.node_lane, r.node_color))
        .collect()
}

fn standing(pending: Option<Vec<Oid>>, carried: Vec<Carried>) -> Standing {
    Standing {
        pending,
        carried: Arc::new(carried),
    }
}

/// The row the status asks for is put there by the laying, and the
/// commits under it sit exactly where the walk left them — which is what
/// makes laying out again safe to do at the moment of
/// publishing.
#[test]
fn the_uncommitted_row_can_be_put_on_a_graph_that_was_walked_without_it() {
    let walked_rows = history();
    let before = where_they_sit(&walked_rows);

    let (laid, _) = lay(
        walked_rows,
        &standing(Some(Vec::new()), Vec::new()),
        Some(oid(3)),
    );

    assert_eq!(laid.len(), 4, "the row was not put on");
    assert!(
        Oid::hex_is_zero(&laid[0].oid_hex),
        "the row is not the one at the top: {}",
        laid[0].oid_hex
    );
    assert_eq!(
        where_they_sit(&laid[1..]),
        before,
        "the history moved under the row"
    );
}

/// And taken off again, with the same history under it. The pair is the
/// whole of what a tree turning dirty and clean again does to the graph.
#[test]
fn the_uncommitted_row_can_be_taken_off_a_graph_that_was_walked_with_it() {
    let walked_rows = history();
    let before = where_they_sit(&walked_rows);
    let (with_row, _) = lay(
        walked_rows,
        &standing(Some(Vec::new()), Vec::new()),
        Some(oid(3)),
    );

    let (without, _) = lay(with_row, &standing(None, Vec::new()), Some(oid(3)));

    assert_eq!(where_they_sit(&without), before, "the history moved");
}

/// Laying it again is the answer to the reading in hand: a copy that
/// has moved to another commit draws above the commit it is on
/// now.
#[test]
fn a_copy_is_laid_where_its_reading_puts_it_now() {
    let copy = |head: u8| Carried {
        name: "seat".into(),
        path: "/tmp/seat".to_string(),
        head: oid(head),
        kinds: Kinds::default(),
    };

    let (was, _) = lay(history(), &standing(None, vec![copy(1)]), Some(oid(3)));
    let at_one = was
        .iter()
        .position(|r| r.carried.is_some())
        .expect("the copy drew no row");
    assert_eq!(
        was[at_one + 1].oid_hex,
        oid(1).to_hex(),
        "the row is not above the commit its reading names"
    );

    let (now, _) = lay(was, &standing(None, vec![copy(3)]), Some(oid(3)));
    let at_three = now
        .iter()
        .position(|r| r.carried.is_some())
        .expect("the copy drew no row after it moved");
    assert_eq!(
        now[at_three + 1].oid_hex,
        oid(3).to_hex(),
        "the row stayed on the commit the copy left"
    );
    assert_eq!(
        now.iter().filter(|r| r.carried.is_some()).count(),
        1,
        "the row the last reading drew was kept as well"
    );
}

/// A branch with no commits yet: nothing to reach down to, so the row
/// stands where the first commit will and takes the whole graph with it.
#[test]
fn the_row_stands_alone_where_there_is_no_commit_to_leash_to() {
    let (laid, _) = lay(Vec::new(), &standing(Some(Vec::new()), Vec::new()), None);
    assert_eq!(laid.len(), 1);
    assert!(Oid::hex_is_zero(&laid[0].oid_hex));
    assert!(
        laid[0].segments.is_empty(),
        "a leash was drawn to nothing: {:?}",
        laid[0].segments
    );
}

/// The listing names every copy's HEAD for the price of one short
/// process, and the readings cost a `status` each on a slower tick — so
/// a copy that has committed is known to have moved long before what it
/// is carrying is. Its reading draws nothing until the next one
/// lands.
#[test]
fn a_reading_the_listing_has_moved_past_draws_nothing() {
    let reading = |path: &str, head: u8| Carried {
        name: "seat".into(),
        path: path.to_string(),
        head: oid(head),
        kinds: Kinds::default(),
    };
    let readings = vec![reading("/tmp/a", 1), reading("/tmp/b", 2)];

    let listed = std::collections::HashMap::from([
        (super::joins::same_path_key("/tmp/a"), oid(1)),
        // b committed since its reading was taken.
        (super::joins::same_path_key("/tmp/b"), oid(9)),
    ]);
    let current = still_where_the_listing_says(&readings, &listed);
    assert_eq!(current.len(), 1, "the reading left behind was kept");
    assert_eq!(current[0].path, "/tmp/a");

    // A copy taken since the last listing has a reading and no entry
    // yet; it is there, so its row is too.
    let unlisted =
        std::collections::HashMap::from([(super::joins::same_path_key("/tmp/a"), oid(1))]);
    assert_eq!(
        still_where_the_listing_says(&readings, &unlisted).len(),
        2,
        "a copy the listing has not named yet lost its row"
    );
}

/// What the walk found keeps everything it said: the laying rewrites
/// where a row sits and nothing else, so the words, the chips and the
/// published mark survive a graph being laid out again.
#[test]
fn laying_out_again_rewrites_the_lanes_and_nothing_a_row_says() {
    let mut walked_rows = history();
    walked_rows[1].published = true;
    walked_rows[1].subject = "the subject the walk read".to_string();

    let (laid, _) = lay(
        walked_rows,
        &standing(Some(Vec::new()), Vec::new()),
        Some(oid(3)),
    );

    let same = laid
        .iter()
        .find(|r| r.oid_hex == oid(2).to_hex())
        .expect("the commit was lost");
    assert!(same.published, "the published mark was dropped");
    assert_eq!(same.subject, "the subject the walk read");
}
