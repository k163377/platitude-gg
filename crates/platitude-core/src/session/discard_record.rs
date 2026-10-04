//! What a write leaves on Platitude GG's own record of what it took
//! (破棄記録仕様.md §2), and the write that brings a part of it back (§4).
//!
//! **The record follows the write's own step**: what has to be read before
//! the thing goes — a copy of the work, a branch's upstream, a stash's
//! message — is read first, and a failure there stops the write with
//! nothing taken; the line goes on once the step landed, and a failure
//! there is reported on its own: the write stays done, and goes unlisted.
//! A copy goes on as what the write took — what its paths held over what it
//! left — also after a write that failed having taken some
//! ([`discards::record_copy`]).

use std::collections::{BTreeSet, HashSet};

use super::*;
use crate::discards::{self, Copied, CopyOf, CopyOperation, RemoteTip, TagBefore};

impl RepoSession {
    /// Answers a line written after a write landed: the log is told, and a
    /// failure goes out on its own (the write it follows is not undone).
    pub(super) fn recorded(&self, written: Result<(), GitError>) {
        match written {
            Ok(()) => self.sink.event(SessionEvent::DiscardsChanged {
                restored: Vec::new(),
            }),
            Err(error) => self.fail("discard record", error),
        }
    }

    /// The same for a step that may have found nothing to write
    /// (`Ok(false)`): the log is told only of a line.
    pub(super) fn recorded_if(&self, written: Result<bool, GitError>) {
        match written {
            Ok(false) => {}
            other => self.recorded(other.map(drop)),
        }
    }

    /// Puts a copy made before the work went on the record once the write
    /// answered (`done`): what it took, landed or failed part-way — the
    /// copy is then all that is left of it — and nothing where it took
    /// nothing. The write's own answer stands either way.
    pub(super) async fn record_copied(
        &self,
        exec: &GitExecutor,
        repo: &RepoInfo,
        copied: Option<Copied>,
        done: Result<(), GitError>,
        cancel: &CancellationToken,
    ) -> Result<(), GitError> {
        let Some(copied) = copied else {
            return done;
        };
        let landed = done.is_ok();
        let written =
            discards::record_copy(exec, &repo.workdir, &repo.git_dir, &copied, landed, cancel)
                .await;
        self.recorded_if(written);
        done
    }

    /// The commits a write is about to move off, for [`Self::tell_if_left`]:
    /// what each of `revs` names now (`HEAD`, a branch moved from under
    /// another's HEAD) — none for a name that names no commit.
    pub(super) async fn moving_off(
        &self,
        exec: &GitExecutor,
        repo: &RepoInfo,
        revs: &[String],
        cancel: &CancellationToken,
    ) -> Result<Vec<Oid>, GitError> {
        let mut found = Vec::new();
        for rev in revs {
            found.extend(discards::commit_of(exec, &repo.workdir, rev, cancel).await?);
        }
        Ok(found)
    }

    /// Tells the discard log a write left `before` where only git's reflogs
    /// reach it now — neither HEAD, a branch nor a remote-tracking branch
    /// holds it — which is what the log lists of a move (§3) and what lights
    /// its seat. One walk, stopped at the first such commit. **Not against
    /// the tags**: every tag is an object to read before the walk starts,
    /// seconds at tens of thousands of them on every write that moves HEAD,
    /// so a commit only a tag holds lights the seat for an entry the log
    /// does not list. A failure here fails nothing: the write landed.
    pub(super) async fn tell_if_left(
        &self,
        exec: &GitExecutor,
        repo: &RepoInfo,
        before: &[Oid],
        cancel: &CancellationToken,
    ) {
        if before.is_empty() {
            return;
        }
        let cmd = GitCommand::new()
            .cwd(&repo.workdir)
            .args(["rev-list", "-1"])
            .args(before.iter().map(Oid::to_hex))
            .args(["--not", "HEAD", "--branches", "--remotes", "--"]);
        match exec.run(cmd, cancel).await {
            Ok(out) if !out.stdout_utf8().trim().is_empty() => {
                self.sink.event(SessionEvent::DiscardsChanged {
                    restored: Vec::new(),
                });
            }
            Ok(_) => {}
            Err(error) => tracing::warn!(%error, "whether a write left commits behind"),
        }
    }

    /// Every branch's tip before a write that rebases, for
    /// [`Self::record_rebase_group`] — read whatever the screen asked: the
    /// user's own `rebase.updateRefs` moves branches together as
    /// `--update-refs` does, and a rebasing pull or a squash rebases too.
    pub(super) async fn tips_before(
        &self,
        exec: &GitExecutor,
        repo: &RepoInfo,
        cancel: &CancellationToken,
    ) -> Result<Option<HashMap<String, Oid>>, GitError> {
        branch_tips(exec, &repo.workdir, cancel).await.map(Some)
    }

    /// Notes the branches a rebase moved together, read off their tips
    /// before and after: one entry for them all (破棄記録仕様.md §2). One
    /// branch moved is git's own line alone, and a rebase that stopped has
    /// moved none yet — the continue that finishes it moves them
    /// ([`stopped_rebase_tips`]).
    pub(super) async fn record_rebase_group(
        &self,
        exec: &GitExecutor,
        repo: &RepoInfo,
        before: Option<HashMap<String, Oid>>,
        cancel: &CancellationToken,
    ) {
        let Some(before) = before else {
            return;
        };
        let after = match branch_tips(exec, &repo.workdir, cancel).await {
            Ok(after) => after,
            Err(error) => return self.fail("discard record", error),
        };
        let mut moved: Vec<(String, Oid, Oid)> = before
            .into_iter()
            .filter_map(|(name, old)| {
                let new = *after.get(&name)?;
                (new != old).then_some((name, old, new))
            })
            .collect();
        if moved.len() < 2 {
            return;
        }
        moved.sort();
        let written =
            discards::record_moves(exec, &repo.workdir, &repo.git_dir, &moved, cancel).await;
        self.recorded(written);
    }

    /// Brings back `parts` of one entry of the discard log, one after
    /// another — the whole entry and a single part alike (§4) — each part
    /// the record gave leaving a note that it is back. A part git refuses
    /// stops the rest, as the first step of a composite write does, and its
    /// refusal is the write's.
    ///
    /// Work goes back into the copy it was thrown out of, which may be
    /// another than this session's: the restore waits its turn there too
    /// ([`Self::write_into`]), so a commit accepted there first does not
    /// take in what comes back.
    pub fn restore_discard(self: &Arc<Self>, parts: Vec<discards::Part>) -> Option<OperationId> {
        let into: Vec<PathBuf> = parts
            .iter()
            .filter_map(|part| match &part.restore {
                discards::Restore::Changes { git_dir, .. } if !git_dir.is_empty() => {
                    Some(PathBuf::from(git_dir))
                }
                _ => None,
            })
            .collect();
        let s = Arc::clone(self);
        self.write_into(
            OperationKind::Restore,
            AfterWrite::Graph,
            &into,
            move |exec, repo, cancel| async move {
                let mut restored = Vec::new();
                let mut outcome = Ok(());
                for part in &parts {
                    match discards::restore(&exec, &repo.workdir, &part.restore, &cancel).await {
                        Ok(how) => restored.push(how),
                        Err(error) => {
                            outcome = Err(error);
                            break;
                        }
                    }
                    if let Some((record, at)) = part.record {
                        let noted = discards::record_restored(
                            &exec,
                            &repo.workdir,
                            &repo.git_dir,
                            &record,
                            at,
                            &cancel,
                        )
                        .await;
                        if let Err(error) = noted {
                            s.fail("discard record", error);
                        }
                    }
                }
                if !restored.is_empty() {
                    s.sink.event(SessionEvent::DiscardsChanged { restored });
                }
                outcome
            },
        )
    }
}

/// Copies what a discard of `rows` throws away: the files as the index and
/// the working tree hold them — a staged rename's old name among them
/// ([`stage::with_old_names`]) — and the untracked ones.
pub(super) async fn copy_of_choices(
    exec: &GitExecutor,
    repo: &RepoInfo,
    rows: &[(String, stage::DiscardSide)],
    cancel: &CancellationToken,
) -> Result<Option<Copied>, GitError> {
    let mut tracked = Vec::new();
    let mut untracked = Vec::new();
    // A staged row's discard takes its file back to HEAD in the index too.
    let mut touches_index = false;
    for (path, side) in rows {
        match side {
            // A repository of its own is no file to take: `clean -f -d`
            // leaves it where it is, so it is no part of the copy either.
            stage::DiscardSide::Untracked if repo.workdir.join(path).join(".git").exists() => {}
            stage::DiscardSide::Untracked => untracked.push(path.clone()),
            stage::DiscardSide::Unstaged => tracked.push(path.clone()),
            stage::DiscardSide::Staged => {
                tracked.push(path.clone());
                touches_index = true;
            }
        }
    }
    tracked.sort();
    tracked.dedup();
    let operation = if tracked.is_empty() {
        CopyOperation::Untracked
    } else {
        CopyOperation::Discard
    };
    copy(
        exec,
        repo,
        &tracked,
        &untracked,
        operation,
        touches_index,
        None,
        cancel,
    )
    .await
}

/// Copies the part of one file's unstaged diff a discard throws away — the
/// file as the working tree holds it.
pub(super) async fn copy_of_partial(
    exec: &GitExecutor,
    repo: &RepoInfo,
    target: &DiffTarget,
    selects: &[HunkSelect],
    cancel: &CancellationToken,
) -> Result<Option<Copied>, GitError> {
    let DiffTarget::Unstaged { path } = target else {
        return Ok(None);
    };
    let operation = if selects.iter().any(|select| select.lines.is_some()) {
        CopyOperation::Lines
    } else {
        CopyOperation::Hunk
    };
    let tracked = [path.clone()];
    copy(exec, repo, &tracked, &[], operation, false, None, cancel).await
}

/// Copies what `reset --hard <rev>` throws away with the commits: every
/// tracked change, staged or not, and the untracked files in the way of
/// what `rev` tracks ([`in_the_way`]) — the other untracked files stay
/// where they are — with the move it goes with, so the list tells the two
/// as one (§2).
pub(super) async fn copy_for_reset(
    exec: &GitExecutor,
    repo: &RepoInfo,
    rev: &str,
    cancel: &CancellationToken,
) -> Result<Option<Copied>, GitError> {
    let current = status::load_tracked(exec, &repo.workdir, cancel).await?;
    let mut tracked: Vec<String> = Vec::new();
    for item in &current.items {
        match item {
            status::StatusItem::Tracked {
                path, orig_path, ..
            } => {
                tracked.push(path.clone());
                tracked.extend(orig_path.iter().cloned());
            }
            status::StatusItem::Unmerged { path, .. } => tracked.push(path.clone()),
            status::StatusItem::Untracked { .. } => {}
        }
    }
    tracked.sort();
    tracked.dedup();
    // A file the index stopped tracking is in the way too, and already the
    // tracked side's: the working tree's version is what that side takes.
    let mut untracked = in_the_way(exec, &repo.workdir, rev, cancel).await?;
    untracked.retain(|path| tracked.binary_search(path).is_err());
    if tracked.is_empty() && untracked.is_empty() {
        return Ok(None);
    }
    let (Some(old), Some(new)) = (
        current.branch_oid,
        discards::commit_of(exec, &repo.workdir, rev, cancel).await?,
    ) else {
        return Ok(None);
    };
    let reference = current.branch_head.as_deref().unwrap_or("HEAD");
    let moved = Some((reference, old, new));
    copy(
        exec,
        repo,
        &tracked,
        &untracked,
        CopyOperation::ResetHard,
        true,
        moved,
        cancel,
    )
    .await
}

/// What `reset --hard <rev>` deletes without a word to write what `rev`
/// tracks (git-reset(1)): whatever stands at a path `rev` has and the index
/// has not, and a file standing where `rev` has a folder. Read off the
/// index's difference from `rev` — the paths only `rev` has (`D`) and only
/// the index has (`A`, a file the index tracks is the copy's tracked side).
async fn in_the_way(
    exec: &GitExecutor,
    workdir: &Path,
    rev: &str,
    cancel: &CancellationToken,
) -> Result<Vec<String>, GitError> {
    let cmd = GitCommand::new().cwd(workdir).args([
        "diff",
        "--cached",
        "--name-status",
        "-z",
        "--no-renames",
        "--diff-filter=AD",
        "--end-of-options",
        rev,
        "--",
    ]);
    let out = exec.run(cmd, cancel).await?;
    let mut fields = out
        .stdout
        .split(|b| *b == 0)
        .filter(|field| !field.is_empty());
    let (mut theirs, mut tracked) = (Vec::new(), HashSet::new());
    while let (Some(status), Some(path)) = (fields.next(), fields.next()) {
        let path = String::from_utf8_lossy(path).into_owned();
        if status == b"D" {
            theirs.push(path);
        } else {
            tracked.insert(path);
        }
    }
    let stands = |path: &str| std::fs::symlink_metadata(workdir.join(path)).ok();
    let mut found = BTreeSet::new();
    for path in theirs {
        if stands(&path).is_some() {
            found.insert(path);
            continue;
        }
        let file_above = path
            .match_indices('/')
            .map(|(at, _)| &path[..at])
            .find(|folder| {
                !tracked.contains(*folder) && stands(folder).is_some_and(|meta| !meta.is_dir())
            });
        if let Some(folder) = file_above {
            found.insert(folder.to_string());
        }
    }
    Ok(found.into_iter().collect())
}

#[expect(clippy::too_many_arguments)]
async fn copy(
    exec: &GitExecutor,
    repo: &RepoInfo,
    tracked: &[String],
    untracked: &[String],
    operation: CopyOperation,
    touches_index: bool,
    moved: Option<(&str, Oid, Oid)>,
    cancel: &CancellationToken,
) -> Result<Option<Copied>, GitError> {
    if tracked.is_empty() && untracked.is_empty() {
        return Ok(None);
    }
    let of = CopyOf {
        workdir: &repo.workdir,
        git_dir: &repo.git_dir,
        tracked,
        untracked,
        operation,
        touches_index,
        moved,
    };
    discards::copy_work(exec, &of, cancel).await
}

/// A remote's branch as the lease names it: `expect`, the commit the screen
/// showed it on, else the remote-tracking ref; `None` where neither names a
/// commit here.
pub(super) async fn remote_tip(
    exec: &GitExecutor,
    workdir: &Path,
    remote: &str,
    branch: &str,
    expect: Option<&str>,
    cancel: &CancellationToken,
) -> Result<Option<RemoteTip>, GitError> {
    let rev = match expect.filter(|expect| !expect.is_empty()) {
        Some(expect) => expect.to_string(),
        None => format!("refs/remotes/{remote}/{branch}"),
    };
    Ok(discards::commit_of(exec, workdir, &rev, cancel)
        .await?
        .map(|tip| RemoteTip {
            remote: remote.to_string(),
            branch: branch.to_string(),
            tip,
        }))
}

/// The token a line goes on the record under, once the write it follows
/// landed: none a close cancels. The write has happened, and the line is a
/// local write of its own (core.md: a local write runs to its end, a close
/// included) — a remote lane's token would drop it with the session.
pub(super) fn record_token() -> CancellationToken {
    CancellationToken::new()
}

/// A remote's tag as the lease read it (`remote::HeldTag`), for the record:
/// its own object — an annotated tag's, which this repository may never
/// have had — and its commit where this repository has it, which the note
/// stands on.
pub(super) async fn held_before(
    exec: &GitExecutor,
    workdir: &Path,
    tag: &str,
    held: &remote::HeldTag,
    cancel: &CancellationToken,
) -> Result<TagBefore, GitError> {
    let commit = discards::commit_of(exec, workdir, &held.commit.to_hex(), cancel).await?;
    Ok(TagBefore {
        name: tag.to_string(),
        object: held.object,
        commit,
    })
}

/// The branches a stopped rebase moves together once it finishes, as they
/// stood when it began: git's own list of them (`rebase-merge/update-refs`,
/// three lines a branch — its name, its tip before, its tip after) and the
/// branch being rebased (`head-name`, `orig-head`). `None` where no rebase
/// that moves others stands. Read off the files under the copy's own git
/// directory, where its rebase keeps them.
pub(super) fn stopped_rebase_tips(git_dir: &Path) -> Option<HashMap<String, Oid>> {
    let state = git_dir.join("rebase-merge");
    let read = |name: &str| std::fs::read_to_string(state.join(name)).ok();
    let listed = read("update-refs")?;
    let lines: Vec<&str> = listed.lines().collect();
    let mut tips = HashMap::new();
    for entry in lines.chunks(3) {
        if let [name, before, _] = entry
            && let Some(name) = name.strip_prefix("refs/heads/")
            && let Ok(before) = Oid::from_hex_str(before)
        {
            tips.insert(name.to_string(), before);
        }
    }
    if let (Some(head), Some(orig)) = (read("head-name"), read("orig-head"))
        && let Some(name) = head.trim().strip_prefix("refs/heads/")
        && let Ok(orig) = Oid::from_hex_str(orig.trim())
    {
        tips.insert(name.to_string(), orig);
    }
    Some(tips)
}

/// The tip the branch a standing rebase rewrites had when it began
/// (`orig-head`, either backend's): what its finish moves the branch off.
pub(super) fn rebase_began_on(git_dir: &Path) -> Option<Oid> {
    ["rebase-merge", "rebase-apply"].iter().find_map(|state| {
        let text = std::fs::read_to_string(git_dir.join(state).join("orig-head")).ok()?;
        Oid::from_hex_str(text.trim()).ok()
    })
}

/// Every branch here and its tip.
async fn branch_tips(
    exec: &GitExecutor,
    workdir: &Path,
    cancel: &CancellationToken,
) -> Result<HashMap<String, Oid>, GitError> {
    let cmd = GitCommand::new().cwd(workdir).args([
        "for-each-ref",
        "--format=%(objectname) %(refname)",
        "refs/heads",
    ]);
    let out = exec.run(cmd, cancel).await?;
    Ok(out
        .stdout_utf8()
        .lines()
        .filter_map(|line| {
            let (oid, name) = line.split_once(' ')?;
            let name = name.strip_prefix("refs/heads/")?;
            Some((name.to_string(), Oid::from_hex_str(oid).ok()?))
        })
        .collect())
}

/// Puts a forced push's overwrite on the record: the tip it replaced, unless
/// the push only added to it (`replaced` is in the history of `local`, what
/// went up). Whether a line went on.
pub(super) async fn record_pushed_over(
    exec: &GitExecutor,
    repo: &RepoInfo,
    local: &str,
    replaced: &RemoteTip,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let Some(pushed) = discards::commit_of(exec, &repo.workdir, local, cancel).await? else {
        return Ok(false);
    };
    let old = replaced.tip.to_hex();
    if branch::is_merged_into(exec, &repo.workdir, &old, &pushed.to_hex(), cancel).await? {
        return Ok(false);
    }
    discards::record_force_push(exec, &repo.workdir, &repo.git_dir, replaced, cancel).await?;
    Ok(true)
}
