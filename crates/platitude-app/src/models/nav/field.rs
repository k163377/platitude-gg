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
