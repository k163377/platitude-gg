use super::*;

impl NavSectionModel {
    /// What one row answers for one role.
    ///
    /// **The only place a row's fields are worked out.** The view reads
    /// it through `data`, and the slots below read it directly, so no row
    /// can show the delegate one thing and tell automation another.
    #[expect(clippy::too_many_lines)]
    pub(super) fn field<'a>(&self, row: Row<'a>, role: Role) -> Value<'a> {
        let (of, depth, from) = match row {
            Row::Made(item) => {
                return match role {
                    Role::Name => Value::Said(&item.name),
                    Role::Full => Value::Said(&item.full),
                    Role::OidHex => Value::Said(&item.oid_hex),
                    Role::Change => Value::Said(&item.change),
                    Role::Bucket => Value::Said(&item.bucket),
                    Role::Group => Value::Said(&item.group),
                    Role::OrigPath => Value::Said(&item.orig_path),
                    // A folder's rename slot carries its own path, never
                    // a rename source.
                    Role::OrigName => Value::Said(""),
                    Role::IsHead => Value::Flag(item.is_head),
                    Role::HasRemote => Value::Flag(item.has_remote),
                    Role::OnlyRemote => Value::Flag(item.only_remote),
                    Role::Upstream => Value::Said(""),
                    Role::HasPr => Value::Flag(item.has_pr),
                    Role::EolMark => Value::Flag(item.eol_mark),
                    Role::Depth => Value::Number(item.depth),
                    Role::Folder => Value::Flag(item.folder),
                };
            }
            Row::Shown { of, depth, from } => (of, depth, from),
        };
        match role {
            Role::Name => Value::Said(Self::shown_name(of, from)),
            Role::Full => Value::Said(if self.tree_named && of.full().is_empty() {
                of.name()
            } else {
                of.full()
            }),
            Role::Depth => Value::Number(depth),
            Role::Folder => Value::Flag(false),
            Role::OidHex => match of {
                Entry::Local(branch) | Entry::Remote(branch) => Value::Spelled(branch.oid.to_hex()),
                Entry::Tag(tag) => Value::Spelled(tag.oid.to_hex()),
                // The commit makes the row clickable: the details pane
                // then shows the stashed changes.
                Entry::Stash(stash) => Value::Spelled(stash.oid.to_hex()),
                Entry::Worktree { .. } | Entry::File { .. } => Value::Said(""),
            },
            Role::IsHead => Value::Flag(match of {
                Entry::Local(branch) | Entry::Remote(branch) => branch.is_head,
                // The working copy this window shows is marked the way the
                // current branch is.
                Entry::Worktree { entry, current } => {
                    entry.path.replace('\\', "/").to_lowercase() == current
                }
                Entry::Tag(_) | Entry::Stash(_) | Entry::File { .. } => false,
            }),
            Role::HasRemote => Value::Flag(match of {
                Entry::Local(branch) | Entry::Remote(branch) => branch.has_remote,
                // Same badge as a branch: nothing means this tag is only
                // here. The bit comes off `ls-remote --tags`, which the
                // fetch carries.
                Entry::Tag(tag) => tag.has_remote,
                Entry::Stash(_) | Entry::Worktree { .. } | Entry::File { .. } => false,
            }),
            Role::OnlyRemote => Value::Flag(matches!(of, Entry::Tag(tag) if !tag.here)),
            Role::Upstream => Value::Said(match of {
                Entry::Local(branch) | Entry::Remote(branch) => &branch.upstream,
                _ => "",
            }),
            Role::HasPr => Value::Flag(match of {
                Entry::Local(branch) => {
                    crate::encode::fake_pr_set().contains(branch.short.as_str())
                }
                Entry::Remote(branch) => {
                    crate::encode::fake_pr_set().contains(pr_key(&branch.short))
                }
                Entry::Worktree { entry, .. } => entry
                    .branch
                    .as_deref()
                    .is_some_and(|branch| crate::encode::fake_pr_set().contains(branch)),
                Entry::Tag(_) | Entry::Stash(_) | Entry::File { .. } => false,
            }),
            Role::Change => match of {
                Entry::File { item, bucket } => Value::Spelled(letters_of(item, bucket)),
                // A branch another working copy has out keeps the state
                // in the same shared slot a worktree row does, and the
                // mark it draws comes out of the same seat — one question
                // (`can a move go here`), one answer, wherever it is met.
                Entry::Local(branch) if branch.held_elsewhere => Value::Said(HELD),
                // A worktree row has no change code, so it keeps the
                // state of the checkout in the same shared slot — the
                // mark the row opens with (`item::LOCKED`).
                Entry::Worktree { entry, .. } => Value::Said(if entry.locked {
                    LOCKED
                } else if entry.prunable {
                    PRUNABLE
                } else {
                    ""
                }),
                _ => Value::Said(""),
            },
            Role::Bucket => Value::Said(match of {
                Entry::File { bucket, .. } => bucket.routing(),
                // A worktree row carries its branch here (empty =
                // detached), which is what the row shows on its right.
                Entry::Worktree { entry, .. } => entry.branch.as_deref().unwrap_or(""),
                _ => "",
            }),
            Role::Group => Value::Said(match of {
                Entry::File { bucket, .. } => bucket.run(),
                _ => "",
            }),
            Role::OrigPath => Value::Said(match of {
                // Where a rename came from, which only the staged side of
                // one knows.
                Entry::File {
                    item: platitude_core::status::StatusItem::Tracked { orig_path, .. },
                    bucket: Bucket::Staged,
                } => orig_path.as_deref().unwrap_or(""),
                // A worktree row's words for the state in the slot
                // beside it: what the lock was given as its reason, or
                // git's own sentence for why it would be pruned. Empty
                // on an ordinary checkout — **and on a lock taken
                // without a reason**, which is a state of its own (the
                // row then says `Locked` and nothing after it).
                Entry::Worktree { entry, .. } => {
                    if entry.locked {
                        &entry.lock_reason
                    } else if entry.prunable {
                        &entry.prune_reason
                    } else {
                        ""
                    }
                }
                _ => "",
            }),
            // The same source, written the way this row writes names: the
            // tree has already spelled `from` bytes of the new path in the
            // folders above, and a source that shared them gives them up
            // too (`encode::rename_source`). The flat view cuts nothing.
            Role::OrigName => Value::Said(match of {
                Entry::File {
                    item: platitude_core::status::StatusItem::Tracked { orig_path, .. },
                    bucket: Bucket::Staged,
                } => crate::encode::rename_source(
                    orig_path.as_deref().unwrap_or(""),
                    of.name(),
                    from,
                ),
                _ => "",
            }),
            Role::EolMark => Value::Flag(matches!(of, Entry::File { item, .. }
                if self.eol_marks.iter().any(|mark| mark.path == item.path()))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::testkit::*;
    use super::*;

    #[test]
    fn a_tag_row_reads_out_of_the_snapshot() {
        let mut model = section(
            "tags",
            Source::Tags(snapshot(
                Vec::new(),
                vec![tag("v1.0", true, true), tag("v2.0-theirs", true, false)],
            )),
        );
        model.arrange();

        assert_eq!(model.shown_rows(), 2);
        assert_eq!(says(&model, 0, Role::Name), "v1.0");
        // A tag never went through the tree, so it has no full name and
        // the sidebar keys it by what it shows.
        assert_eq!(says(&model, 0, Role::Full), "");
        assert_eq!(says(&model, 0, Role::OidHex), oid("a").to_hex());
        assert!(flags(&model, 0, Role::HasRemote));
        assert!(!flags(&model, 0, Role::OnlyRemote));
        assert!(flags(&model, 1, Role::OnlyRemote));
        assert!(!flags(&model, 1, Role::Folder));
        assert_eq!(depth_of(&model, 1), 0);
    }
    #[test]
    fn a_file_row_reads_out_of_the_status() {
        let mut model = section("worktree", Source::files(pending()));
        model.tree_view = false;
        model.arrange();

        assert_eq!(model.shown_rows(), 5, "the MM entry is a row on each side");
        let row = |at| {
            (
                says(&model, at, Role::Name),
                says(&model, at, Role::Full),
                says(&model, at, Role::Change),
                says(&model, at, Role::Bucket),
                says(&model, at, Role::Group),
            )
        };
        assert_eq!(
            row(0),
            (
                "a.txt".into(),
                "a.txt".into(),
                "UU".into(),
                "conflicts".into(),
                "conflicts".into()
            )
        );
        // Untracked routes as itself and shows in the unstaged run —
        // among the unstaged files by name, not after them.
        assert_eq!(
            row(1),
            (
                "c.txt".into(),
                "c.txt".into(),
                "?".into(),
                "untracked".into(),
                "unstaged".into()
            )
        );
        assert_eq!(
            row(2),
            (
                "src/b.txt".into(),
                "src/b.txt".into(),
                "M".into(),
                "unstaged".into(),
                "unstaged".into()
            )
        );
        assert_eq!(
            row(4),
            (
                "src/b.txt".into(),
                "src/b.txt".into(),
                "M".into(),
                "staged".into(),
                "staged".into()
            )
        );
        assert_eq!(says(&model, 3, Role::OrigPath), "old.txt");
        // The unstaged half is what a path asked for by name answers with,
        // as it did when the rows were built.
        assert_eq!(model.change_of("src/b.txt".to_string()), "M");
        assert_eq!(model.change_of("a.txt".to_string()), "UU");
    }
    #[test]
    fn the_short_sections_read_out_of_what_arrived() {
        let mut model = section(
            "stashes",
            Source::Stashes(vec![platitude_core::stash::StashEntry {
                name: "stash@{0}".to_string(),
                oid: oid("c"),
                time: 0,
                message: "On main: a thing".to_string(),
            }]),
        );
        model.arrange();
        assert_eq!(says(&model, 0, Role::Name), "On main: a thing");
        assert_eq!(says(&model, 0, Role::Full), "stash@{0}");
        assert_eq!(says(&model, 0, Role::OidHex), oid("c").to_hex());

        let entry = |path: &str, branch: &str| platitude_core::worktrees::WorktreeEntry {
            path: path.to_string(),
            branch: Some(branch.to_string()),
            head_hex: None,
            bare: false,
            detached: false,
            locked: false,
            lock_reason: String::new(),
            prunable: false,
            prune_reason: String::new(),
        };
        let mut model = section(
            "worktrees",
            Source::Worktrees {
                list: vec![
                    entry("C:\\work\\repo", "main"),
                    entry("C:\\work\\other", "topic"),
                ],
                current: "c:/work/other".to_string(),
            },
        );
        model.arrange();
        assert_eq!(says(&model, 0, Role::Name), "repo");
        assert_eq!(says(&model, 0, Role::Full), "C:\\work\\repo");
        assert_eq!(says(&model, 0, Role::Bucket), "main");
        assert!(!flags(&model, 0, Role::IsHead));
        // The one this window is showing is marked, however git spelled it.
        assert!(flags(&model, 1, Role::IsHead));
        assert_eq!(model.head_row, 1);
    }

    /// The seat a worktree row opens with, and the words behind it — both
    /// out of slots the row shares with the other kinds, so a change to
    /// either would draw the wrong mark rather than fail (`item::LOCKED`).
    #[test]
    fn a_worktree_row_wears_the_state_of_its_checkout() {
        let entry = |path: &str, locked: bool, reason: &str, prunable: bool| {
            platitude_core::worktrees::WorktreeEntry {
                path: path.to_string(),
                branch: Some("topic".to_string()),
                head_hex: None,
                bare: false,
                detached: false,
                locked,
                lock_reason: reason.to_string(),
                prunable,
                prune_reason: if prunable {
                    "gitdir file points to non-existent location".to_string()
                } else {
                    String::new()
                },
            }
        };
        let mut model = section(
            "worktrees",
            Source::Worktrees {
                list: vec![
                    entry("C:\\work\\plain", false, "", false),
                    entry("C:\\work\\held", true, "release run", false),
                    entry("C:\\work\\quiet", true, "", false),
                    entry("C:\\work\\gone", false, "", true),
                    // git reports both on one entry; the lock is the one
                    // somebody chose, so it is the one the seat shows.
                    entry("C:\\work\\both", true, "release run", true),
                ],
                current: String::new(),
            },
        );
        model.arrange();
        assert_eq!(says(&model, 0, Role::Change), "");
        assert_eq!(says(&model, 0, Role::OrigPath), "");
        assert_eq!(says(&model, 1, Role::Change), "LOCKED");
        assert_eq!(says(&model, 1, Role::OrigPath), "release run");
        // A lock taken without a reason is still a lock: the mark comes
        // out and there is nothing to say beside it.
        assert_eq!(says(&model, 2, Role::Change), "LOCKED");
        assert_eq!(says(&model, 2, Role::OrigPath), "");
        assert_eq!(says(&model, 3, Role::Change), "PRUNABLE");
        assert_eq!(
            says(&model, 3, Role::OrigPath),
            "gitdir file points to non-existent location"
        );
        assert_eq!(says(&model, 4, Role::Change), "LOCKED");
        assert_eq!(says(&model, 4, Role::OrigPath), "release run");
    }

    /// A branch row wears the state in the same slot the worktree rows
    /// use, so the seat draws one mark from one field whichever section
    /// the row is in (`item::HELD`).
    #[test]
    fn a_branch_another_copy_holds_wears_the_state_in_the_shared_slot() {
        let local = |short: &str, held: bool| platitude_core::session::BranchItem {
            short: short.into(),
            full: format!("refs/heads/{short}").into(),
            oid: oid("a"),
            has_remote: false,
            is_head: false,
            upstream: "".into(),
            held_elsewhere: held,
        };
        let mut snap = platitude_core::session::RefsSnapshot {
            locals: vec![local("main", false), local("feature/topic-a", true)],
            remotes: Vec::new(),
            tags: Vec::new(),
            head: None,
            remote_names: Vec::new(),
        };
        snap.locals.sort_by(|a, b| a.short.cmp(&b.short));
        let mut model = section("branches", Source::Locals(std::sync::Arc::new(snap)));
        model.arrange();
        assert_eq!(model.oid_of_name("main".to_string()), oid("a").to_hex());
        assert_eq!(
            model.told(Role::Name, "feature/topic-a", Role::Change),
            "HELD"
        );
        assert_eq!(model.told(Role::Name, "main", Role::Change), "");
    }

    /// What the rows that would run `switch` or `branch --delete` ask
    /// before offering: git refuses both for a branch another worktree
    /// holds, and answers nothing about the copy this window is in.
    #[test]
    fn the_worktree_holding_a_branch_answers_for_every_other_copy() {
        let entry = |path: &str, branch: Option<&str>| platitude_core::worktrees::WorktreeEntry {
            path: path.to_string(),
            branch: branch.map(str::to_string),
            head_hex: None,
            bare: false,
            detached: false,
            locked: false,
            lock_reason: String::new(),
            prunable: false,
            prune_reason: String::new(),
        };
        let mut model = section(
            "worktrees",
            Source::Worktrees {
                list: vec![
                    entry("C:\\work\\repo", Some("main")),
                    entry("C:\\work\\other", Some("topic")),
                    entry("C:\\work\\loose", None),
                ],
                current: "c:/work/repo".to_string(),
            },
        );
        model.arrange();
        assert_eq!(
            model.worktree_holding("topic".to_string()),
            "C:\\work\\other"
        );
        // The copy this window is in refuses nothing — moving onto the
        // branch it already has out is a no-op, not a refusal.
        assert_eq!(model.worktree_holding("main".to_string()), "");
        assert_eq!(model.worktree_holding("nobody".to_string()), "");
        // A detached row names no branch, and the empty string must not
        // find it.
        assert_eq!(model.worktree_holding(String::new()), "");
    }
}
