use super::*;

/// Where a section's rows come from: what arrived, in the shape it
/// arrived in, answered row by row as the view asks. Built rows would be
/// a second copy of the snapshot on the Rust heap
/// (ci/baseline/code-costs-windows-x64.md §メモリの形).
#[derive(Default)]
pub(super) enum Source {
    #[default]
    Waiting,
    Locals(Arc<platitude_core::session::RefsSnapshot>),
    Remotes(Arc<platitude_core::session::RefsSnapshot>),
    Tags(Arc<platitude_core::session::RefsSnapshot>),
    Stashes(Vec<platitude_core::stash::StashEntry>),
    /// The working copies, and which one this window shows (no entry says).
    Worktrees {
        list: Vec<platitude_core::worktrees::WorktreeEntry>,
        current: String,
    },
    /// The pending changes and the pane's order. A row is a bucket and an
    /// entry: an `MM` entry is one row under each side.
    Files {
        status: platitude_core::status::WorkingTreeStatus,
        order: Vec<FileAt>,
    },
}

impl platitude_core::mem::Footprint for Source {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::Waiting => 0,
            // Counted under the session, which owns the snapshot.
            Self::Locals(_) | Self::Remotes(_) | Self::Tags(_) => 0,
            Self::Stashes(list) => list.heap_bytes(),
            Self::Worktrees { list, current } => list.heap_bytes() + current.heap_bytes(),
            Self::Files { status, order } => {
                status.heap_bytes() + order.capacity() * size_of::<FileAt>()
            }
        }
    }
}

impl Source {
    pub(super) fn len(&self) -> usize {
        match self {
            Self::Waiting => 0,
            Self::Locals(snapshot) => snapshot.locals.len(),
            Self::Remotes(snapshot) => snapshot.remotes.len(),
            Self::Tags(snapshot) => snapshot.tags.len(),
            Self::Stashes(list) => list.len(),
            Self::Worktrees { list, .. } => list.len(),
            Self::Files { order, .. } => order.len(),
        }
    }

    pub(super) fn entry(&self, at: usize) -> Option<Entry<'_>> {
        match self {
            Self::Waiting => None,
            Self::Locals(snapshot) => snapshot.locals.get(at).map(Entry::Local),
            Self::Remotes(snapshot) => snapshot.remotes.get(at).map(Entry::Remote),
            Self::Tags(snapshot) => snapshot.tags.get(at).map(Entry::Tag),
            Self::Stashes(list) => list.get(at).map(Entry::Stash),
            Self::Worktrees { list, current } => {
                list.get(at).map(|entry| Entry::Worktree { entry, current })
            }
            Self::Files { status, order } => order.get(at).and_then(|row| {
                status.items.get(row.at as usize).map(|item| Entry::File {
                    item,
                    bucket: row.bucket,
                })
            }),
        }
    }

    /// The branch row named `short`, by binary search over the
    /// name-ordered section (`RefsSnapshot::local_named`).
    pub(super) fn branch_named(&self, short: &str) -> Option<&platitude_core::session::BranchItem> {
        match self {
            Self::Locals(snapshot) => snapshot.local_named(short),
            Self::Remotes(snapshot) => snapshot.remote_named(short),
            _ => None,
        }
    }

    /// The branches `remote` carries here, by the name half alone
    /// (`main`, `feature/x`), in name order — what the upstream question
    /// offers (デザイン規約 §ブランチが測られる相手を決める).
    ///
    /// The cut is by configured name, not by the first slash: the `my/`
    /// prefix run also holds remote `my/fork`'s `my/fork/main`
    /// ([`platitude_core::refs::split_remote_ref`]).
    pub(super) fn branches_on<'a>(&'a self, remote: &str, remotes: &[&'a str]) -> Vec<&'a str> {
        let Self::Remotes(snapshot) = self else {
            return Vec::new();
        };
        let prefix = format!("{remote}/");
        let from = snapshot
            .remotes
            .partition_point(|branch| branch.short.as_str() < prefix.as_str());
        snapshot.remotes[from..]
            .iter()
            .take_while(|branch| branch.short.as_str().starts_with(&prefix))
            .filter_map(|branch| {
                platitude_core::refs::split_remote_ref(
                    branch.short.as_str(),
                    remotes.iter().copied(),
                )
                .filter(|(on, _)| *on == remote)
                .map(|(_, name)| name)
            })
            .collect()
    }

    /// The tag row named `short`, through a separate name index — tags
    /// are not name-ordered (`RefsSnapshot::tag_named`).
    pub(super) fn tag_named(&self, short: &str) -> Option<&platitude_core::session::TagItem> {
        match self {
            Self::Tags(snapshot) => snapshot.tag_named(short),
            _ => None,
        }
    }

    /// The remotes carrying tag `name`, once each, with whether it stands
    /// apart from the right reading (`RefsSnapshot::tag_remotes`).
    /// Empty before the remotes are read.
    pub(super) fn tag_carriers(&self, name: &str, against: &str) -> Vec<(&str, bool)> {
        match self {
            Self::Tags(snapshot) => snapshot.tag_remotes(name, against),
            _ => Vec::new(),
        }
    }

    /// Whether the reading of tag `name` on `commit` stands apart from the
    /// right one (`RefsSnapshot::tag_apart_at`).
    pub(super) fn tag_apart_at(
        &self,
        name: &str,
        commit: platitude_core::Oid,
        against: &str,
    ) -> bool {
        match self {
            Self::Tags(snapshot) => snapshot.tag_apart_at(name, commit, against),
            _ => false,
        }
    }

    /// Who the right reading of tag `name` belongs to, as a sentence lists
    /// them (`RefsSnapshot::tag_weighed_against`); empty where nobody.
    pub(super) fn tag_weighed_against(&self, name: &str, against: &str) -> String {
        match self {
            Self::Tags(snapshot) => snapshot.tag_weighed_against(name, against).join(", "),
            _ => String::new(),
        }
    }

    /// What a menu on tag `name` stands on (`RefsSnapshot::tag_menu`).
    pub(super) fn tag_menu(&self, name: &str, against: &str, aim: &str) -> TagMenu {
        match self {
            Self::Tags(snapshot) => snapshot.tag_menu(name, against, aim).into(),
            _ => TagMenu::default(),
        }
    }

    /// The one remote a chip of tag `name` at `commit` draws the reading
    /// of (`RefsSnapshot::tag_aim_at`); empty where none.
    pub(super) fn tag_aim_at(&self, name: &str, commit: platitude_core::Oid) -> String {
        match self {
            Self::Tags(snapshot) => snapshot
                .tag_aim_at(name, commit)
                .unwrap_or_default()
                .to_string(),
            _ => String::new(),
        }
    }

    /// Where `remote` carries `name` when that is not where this
    /// repository has the tag; empty when the two agree, when that remote
    /// does not carry the name, or before the remotes are read. Read off
    /// `tag_drifts`: no sidebar row is the remote's reading of a name that
    /// is here too.
    pub(super) fn tag_drift(&self, name: &str, remote: &str) -> String {
        let Self::Tags(snapshot) = self else {
            return String::new();
        };
        snapshot
            .tag_drifts
            .binary_search_by(|d| {
                d.name
                    .as_str()
                    .cmp(name)
                    .then(d.remote.as_str().cmp(remote))
            })
            .map_or_else(
                |_| String::new(),
                |at| snapshot.tag_drifts[at].commit.to_hex(),
            )
    }

    /// Whether the entry at `at` is one of `names`, read as git is asked
    /// about it: `Entry::full` where there is one (a stash's selector),
    /// the name otherwise. How a row shows as gone while its delete is
    /// out (`NavSectionModel::set_hidden`).
    pub(super) fn is_named(&self, at: usize, names: &[String]) -> bool {
        self.entry(at).is_some_and(|of| {
            let full = of.full();
            let key = if full.is_empty() { of.name() } else { full };
            names.iter().any(|named| named == key)
        })
    }

    /// How many entries `names` covers. The early return keeps the scan
    /// off every arrange of a tag section while no delete stands.
    pub(super) fn named_count(&self, names: &[String]) -> usize {
        if names.is_empty() {
            return 0;
        }
        (0..self.len())
            .filter(|at| self.is_named(*at, names))
            .count()
    }

    /// The same status as one list of paths ([`Bucket::Whole`]).
    pub(super) fn whole_files(status: platitude_core::status::WorkingTreeStatus) -> Self {
        let mut order: Vec<FileAt> = (0..status.items.len())
            .map(|at| FileAt {
                bucket: Bucket::Whole,
                at: at as u32,
            })
            .collect();
        by_path(&mut order, &status);
        Self::Files { status, order }
    }

    pub(super) fn files(status: platitude_core::status::WorkingTreeStatus) -> Self {
        let mut order = Vec::new();
        // The heading being filled and where its rows began: buckets
        // sharing a `Bucket::run` are sorted as one list once it is whole.
        let (mut run, mut from) = ("", 0);
        for bucket in Bucket::SHOWN {
            if bucket.run() != run {
                by_path(&mut order[from..], &status);
                (run, from) = (bucket.run(), order.len());
            }
            for (at, item) in status.items.iter().enumerate() {
                if bucket.holds(item) {
                    order.push(FileAt {
                        bucket,
                        at: at as u32,
                    });
                }
            }
        }
        by_path(&mut order[from..], &status);
        Self::Files { status, order }
    }
}

/// Puts one heading's rows in path order. git lists untracked entries
/// after the sorted tracked ones, so unsorted, new files sit at the foot
/// of the unstaged heading and staging one moves it.
fn by_path(rows: &mut [FileAt], status: &platitude_core::status::WorkingTreeStatus) {
    let path_of = |row: &FileAt| {
        status
            .items
            .get(row.at as usize)
            .map_or("", platitude_core::status::StatusItem::path)
    };
    rows.sort_by(|a, b| path_of(a).cmp(path_of(b)));
}

/// One file row: its bucket and its index into the status.
pub(super) struct FileAt {
    bucket: Bucket,
    at: u32,
}

/// Which of git's four answers a file row came out of.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Bucket {
    Conflicts,
    Unstaged,
    Untracked,
    Staged,
    /// Every changed path once, whichever sides it changed on — how
    /// another working copy's changes are shown, since the stage split
    /// belongs to that copy's index (デザイン規約 §別の作業コピーを読む).
    Whole,
}

impl Bucket {
    /// GitKraken display order. Buckets sharing a heading must stand next
    /// to each other: `files` closes a heading the moment `run` changes.
    const SHOWN: [Self; 4] = [
        Self::Conflicts,
        Self::Unstaged,
        Self::Untracked,
        Self::Staged,
    ];

    /// Whether an entry belongs in this bucket — the same test as
    /// `WorkingTreeStatus`'s four iterators (held together by the test at
    /// the foot).
    fn holds(self, item: &platitude_core::status::StatusItem) -> bool {
        use platitude_core::status::StatusItem;
        match (self, item) {
            (Self::Whole, _)
            | (Self::Conflicts, StatusItem::Unmerged { .. })
            | (Self::Untracked, StatusItem::Untracked { .. }) => true,
            (Self::Unstaged, StatusItem::Tracked { unstaged, .. }) => *unstaged != '.',
            (Self::Staged, StatusItem::Tracked { staged, .. }) => *staged != '.',
            _ => false,
        }
    }

    /// What the row routes diffs and staging by. `Whole` has none of its
    /// own — the side is the entry's ([`Self::routing_of`]).
    pub(super) fn routing(self) -> &'static str {
        match self {
            Self::Conflicts => "conflicts",
            Self::Unstaged | Self::Whole => "unstaged",
            Self::Untracked => "untracked",
            Self::Staged => "staged",
        }
    }

    /// The same for a `Whole` row: the index where anything is staged,
    /// the working tree otherwise — the nearer answer to what the copy
    /// has.
    pub(super) fn routing_of(self, item: &platitude_core::status::StatusItem) -> &'static str {
        use platitude_core::status::StatusItem;
        match (self, item) {
            (Self::Whole, StatusItem::Unmerged { .. }) => "conflicts",
            (Self::Whole, StatusItem::Untracked { .. }) => "untracked",
            (Self::Whole, StatusItem::Tracked { staged, .. }) if *staged != '.' => "staged",
            (Self::Whole, StatusItem::Tracked { .. }) => "unstaged",
            _ => self.routing(),
        }
    }

    /// The run the pane shows it in — where an untracked file counts as
    /// unstaged, while `routing` keeps the real answer.
    pub(super) fn run(self) -> &'static str {
        match self {
            Self::Conflicts => "conflicts",
            Self::Unstaged | Self::Untracked => "unstaged",
            Self::Staged => "staged",
            Self::Whole => "whole",
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Entry<'a> {
    Local(&'a platitude_core::session::BranchItem),
    Remote(&'a platitude_core::session::BranchItem),
    Tag(&'a platitude_core::session::TagItem),
    Stash(&'a platitude_core::stash::StashEntry),
    Worktree {
        entry: &'a platitude_core::worktrees::WorktreeEntry,
        /// The working copy this window is showing, spelled the way
        /// `path` is compared (forward slashes, lower case).
        current: &'a str,
    },
    File {
        item: &'a platitude_core::status::StatusItem,
        bucket: Bucket,
    },
}

impl<'a> Entry<'a> {
    /// What git calls it, before any indenting takes it apart.
    pub(super) fn name(self) -> &'a str {
        match self {
            Self::Local(branch) | Self::Remote(branch) => &branch.short,
            Self::Tag(tag) => &tag.short,
            // A stash shows its message; the selector that names it to
            // git rides in `full`.
            Self::Stash(stash) => &stash.message,
            Self::Worktree { entry, .. } => leaf_of(&entry.path),
            Self::File { item, .. } => item.path(),
        }
    }

    /// What git knows the row by, where that is not what it shows. A ref
    /// has none until the tree writes one (`field`).
    pub(super) fn full(self) -> &'a str {
        match self {
            Self::Local(_) | Self::Remote(_) | Self::Tag(_) => "",
            Self::Stash(stash) => &stash.name,
            Self::Worktree { entry, .. } => &entry.path,
            Self::File { item, .. } => item.path(),
        }
    }
}

/// The last segment of a worktree path, cut where the platform ends a
/// folder (`urlpath::path_leaf`).
fn leaf_of(path: &str) -> &str {
    crate::urlpath::path_leaf(path)
}
/// A remote branch is looked up without its remote prefix, so
/// `origin/main` matches a PR on `main`.
pub(super) fn pr_key(short: &str) -> &str {
    short.split_once('/').map_or(short, |(_, rest)| rest)
}

/// The letters git reports for one file row.
pub(super) fn letters_of(item: &platitude_core::status::StatusItem, bucket: Bucket) -> String {
    use platitude_core::status::StatusItem;
    match (item, bucket) {
        (StatusItem::Unmerged { ours, theirs, .. }, _) => format!("{ours}{theirs}"),
        (StatusItem::Tracked { unstaged, .. }, Bucket::Unstaged) => unstaged.to_string(),
        (StatusItem::Tracked { staged, .. }, Bucket::Staged) => staged.to_string(),
        (StatusItem::Untracked { .. }, _) => "?".to_string(),
        // The index's letter where it has one (デザイン規約 §別の作業コピーを読む).
        (
            StatusItem::Tracked {
                staged, unstaged, ..
            },
            Bucket::Whole,
        ) => match *staged == '.' {
            true => unstaged,
            false => staged,
        }
        .to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::testkit::*;
    use super::*;

    /// `holds` re-asks core's four iterators one entry at a time (so a row
    /// can name its bucket); this keeps the two in step.
    #[test]
    fn a_bucket_holds_what_core_says_it_holds() {
        let status = pending();
        for (bucket, theirs) in [
            (Bucket::Conflicts, status.conflicted().collect::<Vec<_>>()),
            (Bucket::Unstaged, status.unstaged().collect()),
            (Bucket::Untracked, status.untracked().collect()),
            (Bucket::Staged, status.staged().collect()),
        ] {
            let ours: Vec<_> = status.items.iter().filter(|i| bucket.holds(i)).collect();
            assert_eq!(
                ours,
                theirs,
                "{} does not hold what core puts in it",
                bucket.routing(),
            );
        }
    }

    /// Another copy's list: every path once, with the index's letter where
    /// it has one. The fixture's `MM` `src/b.txt` is the case that matters.
    #[test]
    fn a_copys_list_holds_every_path_once() {
        let status = pending();
        let Source::Files { order, .. } = Source::whole_files(status.clone()) else {
            panic!("whole_files builds a file source");
        };
        assert_eq!(
            order.len(),
            status.items.len(),
            "one row per path, whatever sides it changed on"
        );
        let letters: Vec<String> = order
            .iter()
            .map(|at| {
                let item = &status.items[at.at as usize];
                letters_of(item, at.bucket)
            })
            .collect();
        // Path order: a.txt (conflicted), c.txt (untracked), d.txt
        // (staged rename), src/b.txt (both sides — the index's letter).
        assert_eq!(letters, vec!["UU", "?", "R", "M"]);
        let sides: Vec<&str> = order
            .iter()
            .map(|at| at.bucket.routing_of(&status.items[at.at as usize]))
            .collect();
        assert_eq!(
            sides,
            vec!["conflicts", "untracked", "staged", "staged"],
            "a path on both sides opens the index's, which holds the newer bytes"
        );
    }

    /// The unstaged heading joins git's tracked and untracked lists, each
    /// sorted on its own (`by_path`).
    #[test]
    fn a_heading_lists_its_files_by_name_whichever_bucket_they_came_from() {
        use platitude_core::status::{StatusItem, WorkingTreeStatus};
        let under = |source: &Source, run: &str| -> Vec<String> {
            (0..source.len())
                .filter_map(|at| source.entry(at))
                .filter_map(|of| match of {
                    Entry::File { item, bucket } if bucket.run() == run => {
                        Some(item.path().to_string())
                    }
                    _ => None,
                })
                .collect()
        };
        let edited = WorkingTreeStatus {
            items: vec![
                tracked('.', 'M', ".idea/gradle.xml"),
                tracked('.', 'M', ".idea/misc.xml"),
                StatusItem::Untracked {
                    path: ".idea/kotlinc.xml".to_string(),
                },
            ],
            ..Default::default()
        };
        let all_staged = WorkingTreeStatus {
            items: vec![
                tracked('M', '.', ".idea/gradle.xml"),
                tracked('A', '.', ".idea/kotlinc.xml"),
                tracked('M', '.', ".idea/misc.xml"),
            ],
            ..Default::default()
        };

        let listed = under(&Source::files(edited), "unstaged");
        assert_eq!(
            listed,
            [".idea/gradle.xml", ".idea/kotlinc.xml", ".idea/misc.xml"]
        );
        assert_eq!(
            listed,
            under(&Source::files(all_staged), "staged"),
            "staging the untracked file must not move it in the list",
        );
    }
}
