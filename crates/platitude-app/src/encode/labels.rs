//! The ref chips on a graph row, as QML draws them: one record per name
//! the commit carries, and the words the rest of the app tells a chip's
//! kind by.

use platitude_core::session::{LabelKind, RefLabel};
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::wire::{Fields, Listed, Record, field};

/// Branch names wearing the PR badge. Real PR data joins in Phase 4; until
/// then the only thing that ever fills this is the harness, so the design
/// can be reviewed (`harness::Knobs::fake_pr`) — and a build without the
/// harness has an empty set here and no way to be handed a full one.
pub(crate) fn pr_set() -> &'static std::collections::HashSet<String> {
    &crate::harness::knobs().fake_pr
}

/// One chip: what the frame says (the kind), what the name says (where
/// the ref is), and the marks around it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chip {
    pub kind: LabelKind,
    pub name: String,
    /// HEAD is on this branch (bold name, and the colour "here").
    pub is_head: bool,
    /// On a remote as well — the cloud badge.
    pub has_remote: bool,
    /// A PR is open for it — the badge's other face; the two never stack.
    pub has_pr: bool,
    /// This repository holds the ref: the whereabouts the chip writes in
    /// the name's colour (デザイン規約 §ref の種別). False for a remote
    /// branch, and for a tag only a remote has.
    pub here: bool,
    /// Another working copy has this branch checked out — the green
    /// frame, and why git refuses a `switch` onto it (measured).
    pub held: bool,
    /// `git worktree lock` is on that copy — the padlock beside the name.
    pub locked: bool,
    /// Whose reading this is, when it is not this repository's: the
    /// remotes carrying a tag, comma-separated. Empty for everything else.
    pub remote: String,
}

/// The chips of one row, in the order core sorted them: HEAD's marker
/// or branch first, then locals, remotes, the working-copy markers and
/// tags. The row's one card shows the first of them.
pub type Chips = Listed<Chip>;

/// The word a chip's kind goes out under, and the word every rule on the
/// other side of the bridge branches on. **The list itself** — a kind
/// added here is a word added to `RefChip.kindKeyOf` and the menus.
pub fn kind_word(kind: LabelKind) -> &'static str {
    match kind {
        LabelKind::Head => "head",
        LabelKind::LocalBranch => "branch",
        LabelKind::RemoteBranch => "remote",
        LabelKind::Worktree => "worktree",
        LabelKind::Tag => "tag",
    }
}

pub fn kind_from_word(word: &str) -> Option<LabelKind> {
    match word {
        "head" => Some(LabelKind::Head),
        "branch" => Some(LabelKind::LocalBranch),
        "remote" => Some(LabelKind::RemoteBranch),
        "worktree" => Some(LabelKind::Worktree),
        "tag" => Some(LabelKind::Tag),
        _ => None,
    }
}

/// The kind a chip's word names *as a ref the menus can act on*: the
/// same word for a branch, a remote branch and a tag, and `""` for the
/// two markers — the detached HEAD and a working copy standing on the
/// commit with no branch out — which name no ref, so there is nothing
/// to switch to, rename or delete (a copy is opened from its own row).
pub fn ref_kind_word(word: &str) -> &'static str {
    match kind_from_word(word) {
        Some(LabelKind::LocalBranch) => "branch",
        Some(LabelKind::RemoteBranch) => "remote",
        Some(LabelKind::Tag) => "tag",
        Some(LabelKind::Head | LabelKind::Worktree) | None => "",
    }
}

impl Chip {
    /// What this chip answers to across the two places it is drawn: its
    /// kind and its name, and nothing of how it is drawn. A background
    /// pass that learns the branch now has a remote rewrites the chip
    /// and would lose a gesture keyed on the whole of it, and the row
    /// and the card its chip unfolds into have to agree on what "the
    /// same target" means (デザイン規約 §グラフ行のダブルクリック). A tag may
    /// share a name with a branch, so the kind stays part of it.
    pub fn key(&self) -> String {
        chip_key(kind_word(self.kind), &self.name)
    }
}

/// The key a chip of that kind and name answers to ([`Chip::key`]), for
/// the callers that hold the two halves and no chip — the gone set.
pub fn chip_key(kind: &str, name: &str) -> String {
    format!("{kind}:{name}")
}

/// Labels → chips. `pr` names the branches wearing the PR badge (the
/// callers pass [`pr_set`], the preview until Phase 4); only a local
/// branch wears it.
pub fn chips_of(labels: &[RefLabel], pr: &std::collections::HashSet<String>) -> Chips {
    Listed::new(
        labels
            .iter()
            .map(|l| Chip {
                kind: l.kind,
                name: l.text.to_string(),
                is_head: l.is_head,
                has_remote: l.has_remote,
                has_pr: matches!(l.kind, LabelKind::LocalBranch) && pr.contains(l.text.as_str()),
                here: l.here,
                held: l.held_elsewhere,
                locked: l.locked,
                remote: l.remote.clone(),
            })
            .collect(),
    )
}

/// The gone set: the keys of the chips the window is already showing as
/// gone while git is still being asked to delete the refs they name —
/// at most one per kind, since a delete touches at most one of each,
/// empty halves left out.
pub fn gone_keys(branch: &str, remote: &str, tag: &str) -> Vec<String> {
    [("branch", branch), ("remote", remote), ("tag", tag)]
        .into_iter()
        .filter(|(_, name)| !name.is_empty())
        .map(|(kind, name)| chip_key(kind, name))
        .collect()
}

/// The chips still standing once the gone set has spoken. The window
/// says a deleted ref's chip is gone before the walk that follows the
/// delete lands (デザイン規約 §消す操作は先に画面から消す), and this is
/// where that word is applied.
pub fn chips_shown(chips: &Chips, gone: &[String]) -> Chips {
    if gone.is_empty() {
        return chips.clone();
    }
    Listed::new(
        chips
            .iter()
            .filter(|chip| !gone.contains(&chip.key()))
            .cloned()
            .collect(),
    )
}

impl Record for Chip {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("kind", kind_word(self.kind))
            .put("name", &self.name)
            .put("isHead", &self.is_head)
            .put("hasRemote", &self.has_remote)
            .put("hasPr", &self.has_pr)
            .put("here", &self.here)
            .put("held", &self.held)
            .put("locked", &self.locked)
            .put("remote", &self.remote)
            .put("key", &self.key())
            .done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            kind: kind_from_word(&field::<String>(map, "kind")?).ok_or(())?,
            name: field(map, "name")?,
            is_head: field(map, "isHead")?,
            has_remote: field(map, "hasRemote")?,
            has_pr: field(map, "hasPr")?,
            here: field(map, "here")?,
            held: field(map, "held")?,
            locked: field(map, "locked")?,
            remote: field(map, "remote")?,
        })
    }
}

impl platitude_core::mem::Footprint for Chip {
    fn heap_bytes(&self) -> usize {
        self.name.heap_bytes() + self.remote.heap_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qtbridge::qtbridge_type_lib::QVariant;

    fn label(text: &str, kind: LabelKind) -> RefLabel {
        RefLabel {
            text: text.into(),
            kind,
            has_remote: false,
            is_head: false,
            here: true,
            remote: String::new(),
            held_elsewhere: false,
            locked: false,
        }
    }

    /// [`chips_of`] with no branch wearing the PR badge — what every
    /// test here means when the set is not its subject.
    fn chips_no_pr(labels: &[RefLabel]) -> Chips {
        chips_of(labels, &Default::default())
    }

    #[test]
    fn a_chip_carries_every_mark_the_label_had_and_round_trips() {
        let chips = chips_no_pr(&[
            RefLabel {
                has_remote: true,
                is_head: true,
                ..label("main", LabelKind::LocalBranch)
            },
            RefLabel {
                held_elsewhere: true,
                locked: true,
                ..label("feature/topic-a", LabelKind::LocalBranch)
            },
            RefLabel {
                has_remote: true,
                here: false,
                remote: "origin, fork".into(),
                ..label("v9.9", LabelKind::Tag)
            },
            RefLabel {
                locked: true,
                ..label("spike", LabelKind::Worktree)
            },
            label("HEAD", LabelKind::Head),
        ]);
        assert_eq!(
            chips[0],
            Chip {
                kind: LabelKind::LocalBranch,
                name: "main".into(),
                is_head: true,
                has_remote: true,
                has_pr: false,
                here: true,
                held: false,
                locked: false,
                remote: String::new(),
            }
        );
        assert!(chips[1].held && chips[1].locked);
        assert!(!chips[2].here);
        assert_eq!(chips[2].remote, "origin, fork");
        // The marker for a copy with no branch out carries the padlock on
        // its own: it names no branch, so `held` stays down and the kind
        // is what puts the frame on it.
        assert!(chips[3].locked && !chips[3].held);
        assert_eq!(Chips::try_from(&QVariant::from(&chips)), Ok(chips));
    }

    /// The words the other side branches on, spelled out whole: a kind
    /// that moves is a test that fails.
    #[test]
    fn a_chip_goes_out_under_the_words_the_other_side_reads() {
        let chip = chips_no_pr(&[RefLabel {
            is_head: true,
            has_remote: true,
            ..label("main", LabelKind::LocalBranch)
        }]);
        let map = chip[0].to_map();
        assert_eq!(field::<String>(&map, "kind"), Ok("branch".into()));
        assert_eq!(field::<String>(&map, "name"), Ok("main".into()));
        assert_eq!(field::<bool>(&map, "isHead"), Ok(true));
        assert_eq!(field::<bool>(&map, "hasRemote"), Ok(true));
        assert_eq!(field::<bool>(&map, "hasPr"), Ok(false));
        assert_eq!(field::<bool>(&map, "here"), Ok(true));
        assert_eq!(field::<bool>(&map, "held"), Ok(false));
        assert_eq!(field::<bool>(&map, "locked"), Ok(false));
        assert_eq!(field::<String>(&map, "remote"), Ok(String::new()));
        assert_eq!(field::<String>(&map, "key"), Ok("branch:main".into()));
        for (kind, word) in [
            (LabelKind::Head, "head"),
            (LabelKind::LocalBranch, "branch"),
            (LabelKind::RemoteBranch, "remote"),
            (LabelKind::Worktree, "worktree"),
            (LabelKind::Tag, "tag"),
        ] {
            assert_eq!(kind_word(kind), word);
            assert_eq!(kind_from_word(word), Some(kind));
        }
        assert_eq!(kind_from_word(""), None);
    }

    /// The PR badge answers the set the caller passed — only a local
    /// branch wears it, and only when named. The set is an argument, so
    /// this test is a fact about its inputs alone.
    #[test]
    fn the_pr_badge_answers_the_named_branches() {
        let labels = [
            label("topic", LabelKind::LocalBranch),
            label("topic", LabelKind::Tag),
        ];
        let pr = std::collections::HashSet::from(["topic".to_string()]);
        let chips = chips_of(&labels, &pr);
        assert!(chips[0].has_pr, "the branch wears the badge");
        assert!(!chips[1].has_pr, "the tag sharing its name does not");
    }

    #[test]
    fn a_ref_kind_is_the_word_the_menus_branch_on_and_a_marker_is_none() {
        assert_eq!(ref_kind_word("branch"), "branch");
        assert_eq!(ref_kind_word("remote"), "remote");
        assert_eq!(ref_kind_word("tag"), "tag");
        assert_eq!(ref_kind_word("head"), "", "nothing to act on");
        assert_eq!(ref_kind_word("worktree"), "", "nor is a working copy");
        assert_eq!(ref_kind_word(""), "");
    }

    #[test]
    fn the_gone_set_takes_chips_out_by_kind_and_name() {
        let chips = chips_no_pr(&[
            label("main", LabelKind::LocalBranch),
            label("main", LabelKind::Tag),
            label("origin/main", LabelKind::RemoteBranch),
        ]);
        // A tag sharing the branch's name stays: the kind is part of the
        // identity.
        let shown = chips_shown(&chips, &gone_keys("main", "", ""));
        assert_eq!(shown.len(), 2);
        assert_eq!(shown[0].kind, LabelKind::Tag);
        assert_eq!(shown[1].name, "origin/main");
        assert_eq!(
            chips_shown(&chips, &gone_keys("main", "origin/main", "main")),
            Chips::default()
        );
        assert_eq!(chips_shown(&chips, &[]), chips);
        assert_eq!(
            chips_shown(&Chips::default(), &gone_keys("main", "", "")),
            Chips::default()
        );
        assert!(gone_keys("", "", "").is_empty());
        assert_eq!(gone_keys("main", "", "v1"), ["branch:main", "tag:v1"]);
    }
}
