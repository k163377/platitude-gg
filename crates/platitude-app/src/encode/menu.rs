//! A ref's menu as core answers it for one stand (`GitFacts.refMenu`):
//! what each row and card offers, in fields QML reads by name.

use platitude_core::offers::{PullOffer, RefMenuOffers, UpstreamPlace, WorktreeOffers};
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::wire::{Fields, Record, field};

/// What the right-click menu on a ref offers (`offers::ref_menu`,
/// `offers::pull_row` and `offers::worktree_rows`), read whole as it opens
/// and as it stands again: the row's own rows, the BRANCH and TAG cards'
/// rows, the `pull` row and the WORKTREE card's two making rows. Seats and
/// reasons are words — `Seat::word`, `UpstreamPlace::word`,
/// `PullRuns::word`, `PullOut::word`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefMenu {
    pub switch_to: bool,
    pub switch_asks: bool,
    pub branch_here: bool,
    pub integrate: bool,
    pub standing: bool,
    pub delete: bool,
    pub delete_remote: bool,
    pub set_upstream: bool,
    pub unset_upstream: bool,
    pub current: bool,
    /// Where the branch's upstream stands — the colour its name wears.
    pub upstream_where: String,
    pub push_tag: bool,
    pub delete_remote_tag: String,
    pub delete_tag_everywhere: String,
    /// What the `pull` row runs; empty for no row.
    pub pull_runs: String,
    /// Why that row is greyed; empty while it is live.
    pub pull_out: String,
    pub making: WorktreeMaking,
}

impl RefMenu {
    pub fn new(
        offers: RefMenuOffers,
        pull: Option<PullOffer>,
        upstream: UpstreamPlace,
        making: WorktreeMaking,
    ) -> Self {
        Self {
            switch_to: offers.switch_to,
            switch_asks: offers.switch_asks,
            branch_here: offers.branch_here,
            integrate: offers.integrate_from,
            standing: offers.standing,
            delete: offers.delete,
            delete_remote: offers.delete_remote,
            set_upstream: offers.set_upstream,
            unset_upstream: offers.unset_upstream,
            current: offers.on_current_branch,
            upstream_where: upstream.word().to_string(),
            push_tag: offers.push_tag,
            delete_remote_tag: offers.delete_remote_tag.word().to_string(),
            delete_tag_everywhere: offers.delete_tag_everywhere.word().to_string(),
            pull_runs: pull.map_or("", |pull| pull.runs.word()).to_string(),
            pull_out: pull
                .and_then(|pull| pull.out)
                .map_or("", |out| out.word())
                .to_string(),
            making,
        }
    }
}

impl Record for RefMenu {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("switch", &self.switch_to)
            .put("switchAsks", &self.switch_asks)
            .put("branchHere", &self.branch_here)
            .put("integrate", &self.integrate)
            .put("standing", &self.standing)
            .put("delete", &self.delete)
            .put("deleteRemote", &self.delete_remote)
            .put("setUpstream", &self.set_upstream)
            .put("unsetUpstream", &self.unset_upstream)
            .put("current", &self.current)
            .put("upstreamWhere", &self.upstream_where)
            .put("pushTag", &self.push_tag)
            .put("deleteRemoteTag", &self.delete_remote_tag)
            .put("deleteTagEverywhere", &self.delete_tag_everywhere)
            .put("pullRuns", &self.pull_runs)
            .put("pullOut", &self.pull_out)
            .put("making", &self.making.to_map())
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        Some(Self {
            switch_to: field(map, "switch")?,
            switch_asks: field(map, "switchAsks")?,
            branch_here: field(map, "branchHere")?,
            integrate: field(map, "integrate")?,
            standing: field(map, "standing")?,
            delete: field(map, "delete")?,
            delete_remote: field(map, "deleteRemote")?,
            set_upstream: field(map, "setUpstream")?,
            unset_upstream: field(map, "unsetUpstream")?,
            current: field(map, "current")?,
            upstream_where: field(map, "upstreamWhere")?,
            push_tag: field(map, "pushTag")?,
            delete_remote_tag: field(map, "deleteRemoteTag")?,
            delete_tag_everywhere: field(map, "deleteTagEverywhere")?,
            pull_runs: field(map, "pullRuns")?,
            pull_out: field(map, "pullOut")?,
            making: WorktreeMaking::from_map(&field::<QVariantMap>(map, "making")?)?,
        })
    }
}

/// The WORKTREE card's two making rows (`offers::worktree_rows`): whether
/// `Create worktree here…` stands — only where new worktrees have a place
/// to go —, how the row's own branch goes out (`branch` / `track`, empty
/// for no row), and why that row is greyed (`current` / `held` /
/// `stopped`, empty while live).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeMaking {
    pub here: bool,
    pub checkout: String,
    pub out: String,
}

impl WorktreeMaking {
    /// `placed`: the worktrees have a place for a new one — a box whose
    /// Enter could make nothing is not offered.
    pub fn new(offers: WorktreeOffers, placed: bool) -> Self {
        Self {
            here: offers.here && placed,
            checkout: offers
                .checkout
                .map_or("", |checkout| checkout.word())
                .to_string(),
            out: offers.checkout_out.map_or("", |out| out.word()).to_string(),
        }
    }
}

impl Record for WorktreeMaking {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("here", &self.here)
            .put("checkout", &self.checkout)
            .put("out", &self.out)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        Some(Self {
            here: field(map, "here")?,
            checkout: field(map, "checkout")?,
            out: field(map, "out")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platitude_core::offers::{PullOut, PullRuns, Seat, WorktreeAddOut, WorktreeCheckout};

    #[test]
    fn the_menus_answer_says_each_seat_and_reason_in_its_word() {
        let menu = RefMenu::new(
            RefMenuOffers {
                switch_to: true,
                delete_remote_tag: Seat::Greyed,
                delete_tag_everywhere: Seat::Live,
                ..RefMenuOffers::default()
            },
            Some(PullOffer {
                runs: PullRuns::Aside,
                out: Some(PullOut::OwnCommits),
            }),
            UpstreamPlace::Current,
            WorktreeMaking::default(),
        );
        assert!(menu.switch_to);
        assert_eq!(
            (
                menu.delete_remote_tag.as_str(),
                menu.delete_tag_everywhere.as_str()
            ),
            ("greyed", "live")
        );
        assert_eq!(
            (menu.pull_runs.as_str(), menu.pull_out.as_str()),
            ("aside", "own")
        );
        assert_eq!(menu.upstream_where, "current");
        let none = RefMenu::new(
            RefMenuOffers::default(),
            None,
            UpstreamPlace::None,
            WorktreeMaking::default(),
        );
        assert_eq!(
            (
                none.pull_runs.as_str(),
                none.pull_out.as_str(),
                none.delete_remote_tag.as_str()
            ),
            ("", "", ""),
            "no row is no word"
        );
        assert_eq!(RefMenu::from_map(&menu.to_map()), Some(menu));
    }

    #[test]
    fn a_making_row_with_nowhere_to_go_is_not_offered() {
        let offers = WorktreeOffers {
            here: true,
            checkout: Some(WorktreeCheckout::Branch),
            checkout_out: Some(WorktreeAddOut::Held),
        };
        let making = WorktreeMaking::new(offers, true);
        assert_eq!(
            making,
            WorktreeMaking {
                here: true,
                checkout: "branch".into(),
                out: "held".into(),
            }
        );
        assert!(!WorktreeMaking::new(offers, false).here);
        assert_eq!(
            WorktreeMaking::from_map(&making.to_map()),
            Some(making.clone())
        );
        let menu = RefMenu::new(RefMenuOffers::default(), None, UpstreamPlace::None, making);
        assert_eq!(
            RefMenu::from_map(&menu.to_map()),
            Some(menu),
            "nested whole"
        );
    }
}
