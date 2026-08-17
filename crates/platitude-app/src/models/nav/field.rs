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
}
