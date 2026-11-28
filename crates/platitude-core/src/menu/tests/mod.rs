//! Every class's rows (`table`), and what each row says it can do
//! (`lines`; a branch's upstream and its pull: `upstream`; the subjects no
//! shape makes: `apart`).

mod apart;
mod graph;
mod kept;
mod lines;
mod table;
mod testkit;
mod upstream;

use std::collections::BTreeMap;

use super::{Card, Place, Row};

/// A menu drawn on one line, as its rows and whether each is live: groups
/// apart by ` | `, a card's rows inside its title's brackets with its groups
/// apart by ` / `, a greyed row behind `~`.
fn render(rows: &BTreeMap<Row, bool>) -> String {
    let mut out = String::new();
    let mut at = None;
    for (&row, &live) in rows {
        let (group, inner) = row.place().order();
        match at {
            None => open_card(&mut out, row.place()),
            Some((was, was_inner)) if was != group => {
                if Option::is_some(&was_inner) {
                    out.push(']');
                }
                out.push_str(" | ");
                open_card(&mut out, row.place());
            }
            // A card whose title stands in a group of the first level.
            Some((_, None)) if inner.is_some() => {
                out.push(' ');
                open_card(&mut out, row.place());
            }
            Some((_, was_inner)) if was_inner != inner => out.push_str(" / "),
            Some(_) => out.push(' '),
        }
        if !live {
            out.push('~');
        }
        out.push_str(&format!("{row:?}"));
        at = Some((group, inner));
    }
    if at.is_some_and(|(_, inner)| inner.is_some()) {
        out.push(']');
    }
    out
}

fn open_card(out: &mut String, place: Place) {
    if let Place::In(card, _) = place {
        out.push_str(match card {
            Card::Reset => "RESET[",
            Card::Branch => "BRANCH[",
            Card::Worktree => "WORKTREE[",
            Card::Tag => "TAG[",
        });
    }
}

#[test]
fn every_row_has_a_word_of_its_own() {
    let mut words: Vec<&str> = Row::ALL.iter().map(|row| row.word()).collect();
    words.sort_unstable();
    words.dedup();
    assert_eq!(words.len(), Row::ALL.len());
}

/// A drawn menu parts its rows by these words, so two seats share a word
/// exactly where they are one seat.
#[test]
fn every_seat_has_a_word_of_its_own() {
    for a in Row::ALL {
        for b in Row::ALL {
            let (at, bt) = (a.place(), b.place());
            let (a_group, a_inner) = at.order();
            let (b_group, b_inner) = bt.order();
            assert_eq!(a_group == b_group, a_group.word() == b_group.word());
            assert_eq!(
                at.card() == bt.card(),
                at.card().map(Card::word) == bt.card().map(Card::word)
            );
            assert_eq!(
                a_inner == b_inner,
                a_inner.map(|inner| inner.word()) == b_inner.map(|inner| inner.word())
            );
        }
    }
}

#[test]
fn rows_stand_in_the_order_of_their_places() {
    let orders: Vec<_> = Row::ALL.iter().map(|row| row.place().order()).collect();
    assert!(
        orders.is_sorted(),
        "a row declared out of its place's order: {orders:?}"
    );
}
