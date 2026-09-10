use super::*;

/// Where a section's rows come from.
///
/// Every section keeps what arrived, in the shape it arrived in, and
/// answers for a row as the view asks. Holding the refs sections as built
/// rows instead put megabytes on the process's Rust heap, all of it a
/// second copy of what the snapshot says
/// (ci/baseline/code-costs-windows-x64.md §メモリの形).
#[derive(Default)]
pub(super) enum Source {
    #[default]
    Waiting,
    Locals(Arc<platitude_core::session::RefsSnapshot>),
    Remotes(Arc<platitude_core::session::RefsSnapshot>),
    Tags(Arc<platitude_core::session::RefsSnapshot>),
    Stashes(Vec<platitude_core::stash::StashEntry>),
    /// The working copies, and which of them this window is showing —
    /// the one fact about a worktree row that is not in the entry.
    Worktrees {
        list: Vec<platitude_core::worktrees::WorktreeEntry>,
        current: String,
    },
    /// The pending changes, with the order the pane shows them in. **A
    /// row is a bucket and an entry**, not an entry: an entry with both
    /// halves changed (`MM`) is one row under the index and another under
    /// the working tree.
    Files {
        status: platitude_core::status::WorkTreeStatus,
        order: Vec<FileAt>,
    },
}

impl platitude_core::mem::Footprint for Source {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::Waiting => 0,
            // The snapshot belongs to the session, which is where the
            // memory report counts it.
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

    /// The branch row named `short`, through the snapshot's own index —
    /// the two ref sections are name-ordered, so a menu's question about
    /// a name is a binary search rather than a walk of every row
    /// (`RefsSnapshot::local_named`). `None` for every other section.
    pub(super) fn branch_named(&self, short: &str) -> Option<&platitude_core::session::BranchItem> {
        match self {
            Self::Locals(snapshot) => snapshot.local_named(short),
            Self::Remotes(snapshot) => snapshot.remote_named(short),
            _ => None,
        }
    }

    /// The tag row named `short`, the same way (`RefsSnapshot::tag_named`
    /// — the tags keep their own order, so the index is a separate one).
    pub(super) fn tag_named(&self, short: &str) -> Option<&platitude_core::session::TagItem> {
        match self {
            Self::Tags(snapshot) => snapshot.tag_named(short),
            _ => None,
        }
    }

    /// Where `remote` carries `name` when that is not where this
    /// repository has the tag; empty when the two agree, when that remote
    /// does not carry the name, or when nothing has read the remotes yet.
    ///
    /// Off the snapshot's own sorted run rather than the rows: a drift is
    /// listed once for the local tag, and no row of the sidebar is the
    /// remote's reading of a name that is here as well.
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

    /// Whether the entry at `at` is one of `names` — the names being
    /// what git is asked about rather than what a row shows, so a stash
    /// answers to its selector and every other row to the name it shows
    /// (the same split `Entry::full` makes).
    ///
    /// What the sidebar shows a row as gone by while its delete is out
    /// (`NavSectionModel::set_hidden`).
    pub(super) fn is_named(&self, at: usize, names: &[String]) -> bool {
        self.entry(at).is_some_and(|of| {
            let full = of.full();
            let key = if full.is_empty() { of.name() } else { full };
            names.iter().any(|named| named == key)
        })
    }

    /// How many of this source's entries `names` covers. Zero without a
    /// delete standing, which is what keeps the scan off every arrange of
    /// a section holding tens of thousands of tags.
    pub(super) fn named_count(&self, names: &[String]) -> usize {
        if names.is_empty() {
            return 0;
        }
        (0..self.len())
            .filter(|at| self.is_named(*at, names))
            .count()
    }

    pub(super) fn files(status: platitude_core::status::WorkTreeStatus) -> Self {
        let mut order = Vec::new();
        // The heading being filled, and where its rows began: buckets
        // shown under one of them are one list (`Bucket::run`), and a
        // list is put in order once it is whole.
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

/// Puts one heading's rows in name order.
///
/// git answers with its tracked entries sorted and its untracked ones
/// sorted after them, so the unstaged heading — which is both — arrives
/// as two sorted lists one after the other. Left that way, every
/// untracked file sits at the foot of the heading, and staging one moves
/// it up among the others: the same files, listed two different ways on
/// the two sides of the pane.
fn by_path(rows: &mut [FileAt], status: &platitude_core::status::WorkTreeStatus) {
    let path_of = |row: &FileAt| {
        status
            .items
            .get(row.at as usize)
            .map_or("", platitude_core::status::StatusItem::path)
    };
    rows.sort_by(|a, b| path_of(a).cmp(path_of(b)));
}

/// One file row: which of git's four answers it came out of, and the
/// entry in the status it came from.
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
}

impl Bucket {
    /// GitKraken display order, which is also the order the pane's group
    /// runs come in. Buckets sharing a heading stand next to each other,
    /// which is what lets `files` close one off the moment `run` changes.
    const SHOWN: [Self; 4] = [
        Self::Conflicts,
        Self::Unstaged,
        Self::Untracked,
        Self::Staged,
    ];

    /// Whether an entry belongs in this bucket — the same test
    /// `WorkTreeStatus` makes for its four iterators (a test at the foot
    /// of the file holds the two together).
    fn holds(self, item: &platitude_core::status::StatusItem) -> bool {
        use platitude_core::status::StatusItem;
        match (self, item) {
            (Self::Conflicts, StatusItem::Unmerged { .. })
            | (Self::Untracked, StatusItem::Untracked { .. }) => true,
            (Self::Unstaged, StatusItem::Tracked { unstaged, .. }) => *unstaged != '.',
            (Self::Staged, StatusItem::Tracked { staged, .. }) => *staged != '.',
            _ => false,
        }
    }

    /// What the row routes diffs and staging by.
    pub(super) fn routing(self) -> &'static str {
        match self {
            Self::Conflicts => "conflicts",
            Self::Unstaged => "unstaged",
            Self::Untracked => "untracked",
            Self::Staged => "staged",
        }
    }

    /// The run the pane shows it in — where an untracked file counts as
    /// unstaged, while `routing` keeps the real answer.
    pub(super) fn run(self) -> &'static str {
        match self {
            Self::Conflicts => "conflicts",
            Self::Unstaged | Self::Untracked => "unstaged",
            Self::Staged => "staged",
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

/// The last segment of a path — git prints worktree paths the platform's
/// way, so either separator splits (`urlpath::path_leaf` is the one rule).
fn leaf_of(path: &str) -> &str {
    crate::urlpath::path_leaf(path)
}
/// A remote branch is looked up without its remote prefix, so
/// `origin/main` matches a PR on `main`.
pub(super) fn pr_key(short: &str) -> &str {
    short.split_once('/').map_or(short, |(_, rest)| rest)
}

/// The letters git reports for one file row: what each side did to a
/// conflicted file, the half of an ordinary change this row is about, or
/// the mark for a file git has never been told about.
pub(super) fn letters_of(item: &platitude_core::status::StatusItem, bucket: Bucket) -> String {
    use platitude_core::status::StatusItem;
    match (item, bucket) {
        (StatusItem::Unmerged { ours, theirs, .. }, _) => format!("{ours}{theirs}"),
        (StatusItem::Tracked { unstaged, .. }, Bucket::Unstaged) => unstaged.to_string(),
        (StatusItem::Tracked { staged, .. }, Bucket::Staged) => staged.to_string(),
        (StatusItem::Untracked { .. }, _) => "?".to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::testkit::*;
    use super::*;

    /// Which entries a bucket holds is git's answer, and core is where it
    /// is written down — this list asks one entry at a time so a row can
    /// name the bucket it came out of, and the two must not drift.
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

    /// The unstaged heading is two of git's answers at once — the tracked
    /// entries and the untracked files — and git hands each of them over
    /// sorted, one list after the other. Left end to end, the same three
    /// files read one way while one of them was untracked and another way
    /// once all three were staged.
    #[test]
    fn a_heading_lists_its_files_by_name_whichever_bucket_they_came_from() {
        use platitude_core::status::{StatusItem, WorkTreeStatus};
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
        let edited = WorkTreeStatus {
            items: vec![
                tracked('.', 'M', ".idea/gradle.xml"),
                tracked('.', 'M', ".idea/misc.xml"),
                StatusItem::Untracked {
                    path: ".idea/kotlinc.xml".to_string(),
                },
            ],
            ..Default::default()
        };
        let all_staged = WorkTreeStatus {
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
