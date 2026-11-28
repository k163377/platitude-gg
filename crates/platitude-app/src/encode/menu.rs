//! A menu as core answers it for one stand (`GitFacts.menuOn` and the
//! other doors — `carriedMenu`, `filesMenu`, `remoteMenu`,
//! `RebasePlanModel.stepMenu`): the rows its class has, why each greys and
//! the seat each stands in, which `MenuLines` draws the menu by, and what
//! a press on some of them sends, in fields QML reads by name.

use platitude_core::menu::{Can, Card, CardGroup, Line};
use platitude_core::offers::{PullRuns, UpstreamPlace, WorktreeCheckout};
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::wire::{Fields, Listed, Record, field};

/// One row of the answer: its kind, why it greys and its seat
/// (`menu::Row::place`), which `MenuLines` draws it by.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuLine {
    /// `menu::Row::word`.
    pub word: String,
    /// Why it is greyed (`menu::Why::word`), empty while it is live.
    pub why: String,
    /// The first-level group it stands in (`menu::Group::word`) — a card's
    /// row, the group its card's title stands in.
    pub group: String,
    /// The card it hangs behind (`menu::Card::word`) and its group in
    /// there (`menu::CardGroup::word`), both empty on the first level.
    pub card: String,
    pub card_group: String,
    /// It acts on the worktree the WORKTREE card stands on, whatever branch
    /// that has out (`menu::Row::on_worktree`).
    pub on_worktree: bool,
}

impl MenuLine {
    fn of(line: &Line) -> Self {
        let place = line.row.place();
        Self {
            word: line.row.word().to_string(),
            why: match line.can {
                Can::Yes => String::new(),
                Can::No(why) => why.word().to_string(),
            },
            group: place.order().0.word().to_string(),
            card: place.card().map_or("", Card::word).to_string(),
            card_group: place.order().1.map_or("", CardGroup::word).to_string(),
            on_worktree: line.row.on_worktree(),
        }
    }
}

impl Record for MenuLine {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("word", &self.word)
            .put("why", &self.why)
            .put("group", &self.group)
            .put("card", &self.card)
            .put("cardGroup", &self.card_group)
            .put("onWorktree", &self.on_worktree)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        Some(Self {
            word: field(map, "word")?,
            why: field(map, "why")?,
            group: field(map, "group")?,
            card: field(map, "card")?,
            card_group: field(map, "cardGroup")?,
            on_worktree: field(map, "onWorktree")?,
        })
    }
}

/// Core's answer for a right-click menu (`GitFacts.menuOn` /
/// `carriedMenu`), read whole as it opens and as it stands again: the rows
/// its class has — the menu's own and its cards' — and why each is greyed
/// (`menu::lines`), and what a press on some of them sends.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuAnswer {
    /// The rows in drawing order.
    pub lines: Listed<MenuLine>,
    /// The `switch` row's move asks first (`menu::Subject::switch_asks`).
    pub switch_asks: bool,
    /// Where the branch's upstream stands — the colour its name wears
    /// (`UpstreamPlace::word`).
    pub upstream_where: String,
    /// What the `pull` row runs (`menu::pull_runs`, `PullRuns::word`);
    /// empty for no row.
    pub pull_runs: String,
    /// How `worktree add` takes the row's own branch out (`branch` /
    /// `track`, `WorktreeCheckout::word`); empty for no row, and where it
    /// has no way out (a remote row whose local branch is there).
    pub worktree_add: String,
}

impl MenuAnswer {
    pub fn new(
        lines: &[Line],
        switch_asks: bool,
        upstream: UpstreamPlace,
        pull: Option<PullRuns>,
        worktree_add: Option<WorktreeCheckout>,
    ) -> Self {
        Self {
            lines: Listed::new(lines.iter().map(MenuLine::of).collect()),
            switch_asks,
            upstream_where: upstream.word().to_string(),
            pull_runs: pull.map_or("", PullRuns::word).to_string(),
            worktree_add: worktree_add.map_or("", WorktreeCheckout::word).to_string(),
        }
    }

    /// The rows alone, for a menu whose presses send nothing more than the
    /// row (the working tree's files, a plan's step, a remote's own row).
    pub fn of_lines(lines: &[Line]) -> Self {
        Self::new(lines, false, UpstreamPlace::None, None, None)
    }

    /// The rows' words, in drawing order.
    #[cfg(test)]
    pub(crate) fn words(&self) -> Vec<&str> {
        self.lines.iter().map(|line| line.word.as_str()).collect()
    }

    /// Beside each, why it greys — empty while it is live.
    #[cfg(test)]
    pub(crate) fn whys(&self) -> Vec<&str> {
        self.lines.iter().map(|line| line.why.as_str()).collect()
    }
}

impl Record for MenuAnswer {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("lines", &self.lines)
            .put("switchAsks", &self.switch_asks)
            .put("upstreamWhere", &self.upstream_where)
            .put("pullRuns", &self.pull_runs)
            .put("worktreeAdd", &self.worktree_add)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        Some(Self {
            lines: field(map, "lines")?,
            switch_asks: field(map, "switchAsks")?,
            upstream_where: field(map, "upstreamWhere")?,
            pull_runs: field(map, "pullRuns")?,
            worktree_add: field(map, "worktreeAdd")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platitude_core::menu::{Row, Why};

    #[test]
    fn the_menus_answer_says_each_row_reason_and_press_in_its_word() {
        let menu = MenuAnswer::new(
            &[
                Line {
                    row: Row::BranchHere,
                    can: Can::Yes,
                },
                Line {
                    row: Row::Push,
                    can: Can::No(Why::NothingToSend),
                },
                Line {
                    row: Row::RemoveWorktree,
                    can: Can::No(Why::Locked),
                },
                Line {
                    row: Row::TagHere,
                    can: Can::Yes,
                },
            ],
            true,
            UpstreamPlace::Current,
            Some(PullRuns::Aside),
            Some(WorktreeCheckout::Track),
        );
        let seats: Vec<_> = menu
            .lines
            .iter()
            .map(|line| {
                (
                    line.word.as_str(),
                    line.why.as_str(),
                    line.group.as_str(),
                    line.card.as_str(),
                    line.card_group.as_str(),
                    line.on_worktree,
                )
            })
            .collect();
        assert_eq!(
            seats,
            [
                ("branch-here", "", "start", "", "", false),
                ("push", "clean", "sync", "", "", false),
                (
                    "remove-worktree",
                    "locked",
                    "worktree-card",
                    "worktree",
                    "delete",
                    true
                ),
                ("tag-here", "", "tag-card", "tag", "make", false),
            ]
        );
        assert_eq!(
            (
                menu.switch_asks,
                menu.upstream_where.as_str(),
                menu.pull_runs.as_str(),
                menu.worktree_add.as_str()
            ),
            (true, "current", "aside", "track")
        );
        assert_eq!(MenuAnswer::from_map(&menu.to_map()), Some(menu));
        let none = MenuAnswer::new(&[], false, UpstreamPlace::None, None, None);
        assert_eq!(
            (none.pull_runs.as_str(), none.worktree_add.as_str()),
            ("", ""),
            "no row is no word"
        );
    }

    /// The QML tests hand-build answers through the test double's table (`stubs/GitFacts.qml`'s `seats`), which
    /// `MenuLines` draws the menus by: it says each row's seat, and whether it acts on the worktree itself, as this
    /// record does.
    #[test]
    fn the_test_double_seats_every_row() {
        let double = include_str!("../../tests/qml/stubs/GitFacts.qml");
        let seats: Vec<(String, String)> = double
            .lines()
            .filter_map(|line| {
                let (word, seat) = line.trim().strip_prefix('"')?.split_once("\": [")?;
                Some((
                    word.to_string(),
                    seat.trim_end_matches(',').trim_end_matches(']').to_string(),
                ))
            })
            .collect();
        assert_eq!(seats.len(), Row::ALL.len(), "one seat for every row");
        for (row, (word, said)) in Row::ALL.iter().zip(&seats) {
            let place = row.place();
            let seat = format!(
                "\"{}\", \"{}\", \"{}\", {}",
                place.order().0.word(),
                place.card().map_or("", Card::word),
                place.order().1.map_or("", CardGroup::word),
                row.on_worktree()
            );
            assert_eq!(
                (word.as_str(), said),
                (row.word(), &seat),
                "in drawing order"
            );
        }
    }
}
