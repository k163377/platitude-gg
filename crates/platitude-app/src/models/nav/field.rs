use super::*;

impl NavSectionModel {
    /// What one row answers for one role — the only place a row's fields
    /// are worked out. `data` and the slots both read it, so the delegate
    /// and automation cannot be told different things.
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
                    // A folder's rename slot holds its path, so the
                    // name slot is empty.
                    Role::OrigName => Value::Said(""),
                    Role::IsHead => Value::Flag(item.is_head),
                    Role::HasRemote => Value::Flag(item.has_remote),
                    Role::OnlyRemote => Value::Flag(item.only_remote),
                    Role::HasPr => Value::Flag(item.has_pr),
                    Role::EolMark => Value::Flag(item.eol_mark),
                    Role::Depth => Value::Number(item.depth),
                    Role::Folder => Value::Flag(item.folder),
                    // A folder is measured against no upstream.
                    Role::Ahead | Role::Behind => Value::Number(0),
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
                // Where that checkout stands — what a click on the row
                // jumps to. Empty on a bare entry.
                Entry::Worktree { entry, .. } => {
                    Value::Said(entry.head_hex.as_deref().unwrap_or(""))
                }
                Entry::File { .. } => Value::Said(""),
            },
            Role::IsHead => Value::Flag(match of {
                // By the record's `head_name`, not the snapshot's own
                // marker (why: `head_name`).
                Entry::Local(branch) => {
                    !self.head_name.is_empty() && branch.short.as_str() == self.head_name
                }
                Entry::Remote(_) => false,
                // The worktree this window shows.
                Entry::Worktree { entry, current } => {
                    entry.path.replace('\\', "/").to_lowercase() == current
                }
                Entry::Tag(_) | Entry::Stash(_) | Entry::File { .. } => false,
            }),
            Role::HasRemote => Value::Flag(match of {
                Entry::Local(branch) | Entry::Remote(branch) => branch.has_remote,
                // Same badge as a branch, off the fetch's
                // `ls-remote --tags`.
                Entry::Tag(tag) => tag.has_remote,
                Entry::Stash(_) | Entry::Worktree { .. } | Entry::File { .. } => false,
            }),
            Role::OnlyRemote => Value::Flag(matches!(of, Entry::Tag(tag) if !tag.here)),
            Role::HasPr => Value::Flag(match of {
                Entry::Local(branch) => crate::encode::pr_set().contains(branch.short.as_str()),
                Entry::Remote(branch) => crate::encode::pr_set().contains(pr_key(&branch.short)),
                Entry::Worktree { entry, .. } => entry
                    .branch
                    .as_deref()
                    .is_some_and(|branch| crate::encode::pr_set().contains(branch)),
                Entry::Tag(_) | Entry::Stash(_) | Entry::File { .. } => false,
            }),
            Role::Change => match of {
                Entry::File { item, bucket } => Value::Spelled(letters_of(item, bucket)),
                // Shares the worktree rows' slot and seat (`item::HELD`).
                Entry::Local(branch) if branch.held_elsewhere => Value::Said(HELD),
                // The checkout's state, in the shared slot
                // (`item::LOCKED`).
                Entry::Worktree { entry, .. } => Value::Said(if entry.locked {
                    LOCKED
                } else if entry.prunable {
                    PRUNABLE
                } else if entry.main {
                    MAIN
                } else {
                    ""
                }),
                _ => Value::Said(""),
            },
            Role::Bucket => Value::Said(match of {
                Entry::File { item, bucket } => bucket.routing_of(item),
                // A worktree row carries its branch here (empty =
                // detached), which is what the row shows on its right.
                Entry::Worktree { entry, .. } => entry.branch.as_deref().unwrap_or(""),
                // A local branch has no bucket, so the slot carries the
                // upstream it cannot reach — git's `[gone]`. A name, not a
                // flag: the line the row opens says it and the badge draws
                // its state from it, so the two cannot disagree. Empty
                // where the upstream is here or there is none.
                Entry::Local(branch) => branch.upstream_gone.as_str(),
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
                // A worktree row's words for its state: the lock's reason,
                // or git's sentence for why it would be pruned. Empty on
                // an ordinary checkout and on a lock given no reason.
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
            // The same source, cut the way this row cuts its name: the
            // folders above spell `from` bytes of the new path, and a
            // source sharing them drops them too (`encode::rename_source`).
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
            // A local branch carries its own pair; a remote-tracking ref
            // carries the one made against it (`BranchItem::tracked_by`),
            // drawn by the line naming that branch (`NavRowFacts`).
            Role::Ahead => Value::Number(match of {
                Entry::Local(branch) | Entry::Remote(branch) => counted(branch.ahead),
                _ => 0,
            }),
            Role::Behind => Value::Number(match of {
                Entry::Local(branch) | Entry::Remote(branch) => counted(branch.behind),
                _ => 0,
            }),
        }
    }
}

/// A count as the `i32` role table carries it, saturating.
pub(super) fn counted(commits: u32) -> i32 {
    i32::try_from(commits).unwrap_or(i32::MAX)
}
