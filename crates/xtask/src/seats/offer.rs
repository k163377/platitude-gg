//! The order the roster offers free letters in. The lock decides which
//! session gets a letter; the order only decides which one a session asks
//! for first. A landing hands its letter back while the session that
//! landed it often sits on in the tree and takes it again at its next edit
//! (`land::release_claim`). So the letter the asking session's own
//! conversation last stood in comes first, and the other trees go
//! quietest first: a tree somebody is still at, by its git or by a
//! conversation whose last event was there, comes after the other trees.
//! A letter with no tree comes after every tree, since making one is a
//! cold build. Newcomers read one order, so sessions asking at once meet
//! on one letter first; its claim's creation settles it
//! (`claim::lock_or_read`), and each loser asks for the next.

use std::time::SystemTime;

use super::{
    SEATS, SeatEntry, has_a_directory, same_tree, seat_in_repository, survey, transcripts,
};

/// Every roster letter of the repository at `primary`, in the order to
/// offer them; `first`, the letter the asking session stood in, leads.
pub(super) fn offer_order(
    primary: &str,
    entries: &[SeatEntry],
    first: Option<&str>,
) -> Vec<&'static str> {
    let mut trees: Vec<(&'static str, SystemTime)> = Vec::new();
    let mut treeless: Vec<&'static str> = Vec::new();
    for seat in SEATS {
        match entries.iter().find(|entry| entry.seat == seat) {
            Some(entry) if has_a_directory(&entry.tree.path) => trees.push((
                seat,
                survey::index_written(&entry.tree.path).unwrap_or(SystemTime::UNIX_EPOCH),
            )),
            _ => treeless.push(seat),
        }
    }
    // Only an event after a tree's own last write can move it.
    if let Some(oldest) = trees.iter().map(|(_, heard)| *heard).min() {
        for stand in transcripts::stands_since(oldest) {
            let Some((repository, letter)) = seat_in_repository(&stand.cwd.replace('\\', "/"))
            else {
                continue;
            };
            if !same_tree(&repository, primary) {
                continue;
            }
            if let Some((_, heard)) = trees.iter_mut().find(|(seat, _)| *seat == letter) {
                *heard = (*heard).max(stand.at);
            }
        }
    }
    ordered(trees, treeless, first)
}

/// `first`, then the trees by when they were last heard from, then the
/// treeless letters. Pure, for the tests.
fn ordered(
    mut trees: Vec<(&'static str, SystemTime)>,
    treeless: Vec<&'static str>,
    first: Option<&str>,
) -> Vec<&'static str> {
    trees.sort_by_key(|(_, heard)| *heard);
    let mut order: Vec<&'static str> = trees
        .into_iter()
        .map(|(seat, _)| seat)
        .chain(treeless)
        .collect();
    if let Some(at) = order.iter().position(|seat| Some(*seat) == first) {
        let seat = order.remove(at);
        order.insert(0, seat);
    }
    order
}

#[cfg(test)]
mod tests {
    use super::ordered;
    use std::time::{Duration, SystemTime};

    #[test]
    fn the_seat_stood_in_leads_then_trees_quietest_first_then_the_treeless() {
        let at = |secs| SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
        let trees = || vec![("b", at(30)), ("c", at(10)), ("d", at(20))];
        assert_eq!(
            ordered(trees(), vec!["e", "f"], None),
            ["c", "d", "b", "e", "f"]
        );
        assert_eq!(
            ordered(trees(), vec!["e", "f"], Some("b")),
            ["b", "c", "d", "e", "f"]
        );
        assert_eq!(ordered(Vec::new(), vec!["a"], Some("z")), ["a"]);
    }
}
