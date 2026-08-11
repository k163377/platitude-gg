//! Free helpers of the session: the carry-across moves and rewrites, row
//! building, and the ref joins behind snapshots and label maps.

use super::*;

/// Everything one `git rebase --interactive` needs, held together so the
/// two attempts a carry makes are the same command twice over.
pub(super) struct Replay<'a> {
    upstream: &'a str,
    steps: &'a [sequencer::RebaseStep],
    options: integrate::RebaseOptions,
    /// The todo-editor binary shipped beside the application.
    helper: PathBuf,
}

impl Replay<'_> {
    /// Locates the helper, which packaging must keep beside the app.
    pub(super) fn of<'a>(
        upstream: &'a str,
        steps: &'a [sequencer::RebaseStep],
        options: integrate::RebaseOptions,
    ) -> Result<Replay<'a>, GitError> {
        let helper = sequencer::helper_path().map_err(|source| GitError::Io {
            command: "git rebase --interactive".to_string(),
            source,
        })?;
        Ok(Replay {
            upstream,
            steps,
            options,
            helper,
        })
    }

    async fn run(
        &self,
        executor: &GitExecutor,
        repo: &RepoInfo,
        cancel: &CancellationToken,
    ) -> Result<integrate::RebaseOutcome, GitError> {
        sequencer::rebase_interactive(
            executor,
            repo,
            self.upstream,
            self.steps,
            &self.options,
            &self.helper,
            cancel,
        )
        .await
    }
}

/// The two shapes of history rewrite this application runs, held together
/// because git refuses both over a dirty working tree in the very same
/// words — so both go round through the same stash, and the person who
/// staged half of their work gets it back staged either way
/// (デザイン規約 §未コミット変更がある状態で履歴を書き換える).
pub(super) enum Rewrite<'a> {
    /// `git rebase <upstream>`: a whole branch onto a new base.
    Onto {
        upstream: &'a str,
        options: &'a integrate::RebaseOptions,
    },
    /// `git rebase --interactive`: a plan assembled here, for the edits
    /// that touch one commit (`squash` / reword / drop).
    Replay(&'a Replay<'a>),
}

impl Rewrite<'_> {
    async fn run(
        &self,
        executor: &GitExecutor,
        repo: &RepoInfo,
        cancel: &CancellationToken,
    ) -> Result<integrate::RebaseOutcome, GitError> {
        match self {
            Rewrite::Onto { upstream, options } => {
                integrate::rebase(executor, &repo.workdir, upstream, options, cancel).await
            }
            Rewrite::Replay(replay) => replay.run(executor, repo, cancel).await,
        }
    }
}

/// Replays a one-commit edit plan through `git rebase --interactive`.
pub(super) async fn run_plan(
    executor: &GitExecutor,
    repo: &RepoInfo,
    plan: &sequencer::EditPlan,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let replay = Replay::of(&plan.upstream, &plan.steps, plan.options())?;
    rewrite_carrying(executor, repo, &Rewrite::Replay(&replay), cancel).await
}

/// Runs `rewrite`, going round through a stash when the working tree is
/// in the way (デザイン規約 §未コミット変更がある状態で履歴を書き換える).
pub(super) async fn rewrite_carrying(
    executor: &GitExecutor,
    repo: &RepoInfo,
    rewrite: &Rewrite<'_>,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    match rewrite.run(executor, repo, cancel).await? {
        integrate::RebaseOutcome::Done => Ok(()),
        integrate::RebaseOutcome::Blocked(refusal) => {
            tracing::info!(%refusal, "rebase refused: going round through a stash");
            carry_across_rewrite(executor, repo, rewrite, refusal, cancel).await
        }
    }
}

/// Stash, rewrite, put back — the same three steps [`carry_across`] takes
/// around a move, with the rewrite in the middle. `--autostash` is not
/// what runs them, for two measured reasons: it restores with a plain
/// `stash apply`, so **everything that was staged comes back unstaged**,
/// and when the rebase stops part-way it parks the work in
/// `.git/rebase-merge/autostash`, where `stash list` cannot see it and
/// neither can the graph.
///
/// The restore is skipped when the rebase stopped part-way, and that is
/// the whole difference from a move: git will not write into an index
/// that already holds unmerged paths, so a `pop` there does nothing at
/// all while reporting the conflict it walked into (実測 — 規約 §`stash
/// pop` の非ゼロを conflict と読んでよいのは). The entry stays in the
/// stash list, drawn as its own row in the graph, and the person settling
/// the conflict puts it back when the operation is over — which is where
/// the same three commands typed by hand would leave it.
async fn carry_across_rewrite(
    executor: &GitExecutor,
    repo: &RepoInfo,
    rewrite: &Rewrite<'_>,
    refusal: GitError,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if !stash_everything(executor, repo, cancel).await? {
        // The tree was cleaned between the refusal and now, so there is
        // nothing of ours to carry and nothing of anybody else's to
        // touch: the rebase that was refused goes through as it stands.
        return match rewrite.run(executor, repo, cancel).await? {
            integrate::RebaseOutcome::Done => Ok(()),
            integrate::RebaseOutcome::Blocked(again) => Err(again),
        };
    }
    match rewrite.run(executor, repo, cancel).await {
        Ok(integrate::RebaseOutcome::Done) => {}
        Ok(integrate::RebaseOutcome::Blocked(_)) => {
            // Nothing should stand in the way of a tree that was just
            // emptied, so whatever is holding this one is not something a
            // stash gets past. Put the work back and let git's first
            // refusal say why — a failure to put it back is the more
            // urgent news and goes through instead.
            stash::pop(executor, &repo.workdir, STASH_TOP, cancel).await?;
            return Err(refusal);
        }
        Err(error) => {
            // A rebase that stopped part-way is holding the tree; the
            // work stays in the stash until the operation is over. One
            // that failed without starting leaves the emptied tree, and
            // then the stash was only the room it needed.
            if !opstate::detect(executor, &repo.workdir, cancel)
                .await?
                .any()
            {
                pop_back_after_failure(executor, repo, cancel).await;
            }
            return Err(error);
        }
    }

    pop_back_split_first(executor, repo, cancel).await
}

/// Builds the synthetic row for uncommitted changes: zero id, no author,
/// one dashed edge running down to HEAD. The UI recognizes the all-zero
/// id and renders the dashed empty node and the WIP subject.
pub(super) fn wip_row(head: &Oid, builder: &mut GraphBuilder) -> LogRow {
    let zero = Oid::zero_like(head);
    let g = builder.push_virtual(&zero, head);
    LogRow {
        row: g.row,
        oid_hex: zero.to_hex(),
        short_sha: zero.short_hex(8),
        author: String::new(),
        author_email: String::new(),
        co_authors: Vec::new(),
        time: 0,
        subject: String::new(),
        body: String::new(),
        node_lane: g.node_lane,
        node_color: g.node_color,
        width: g.width,
        segments: g.segments,
        labels: Vec::new(),
        stash_ref: String::new(),
    }
}

/// Row counts of one completed log pass. They differ in both directions:
/// the synthetic WIP row is shown but never walked, and a stash's
/// synthetic index/untracked parents are walked but never shown.
#[derive(Clone, Copy, Default)]
pub(super) struct LogTotals {
    /// Rows delivered to the UI.
    pub(super) shown: u32,
    /// Commits the walk emitted — what `--max-count` limits, so this is
    /// what decides `truncated`.
    pub(super) walked: u32,
}

/// The entry a [`stash_everything`] just made, for the moves that put it
/// back.
const STASH_TOP: &str = "stash@{0}";

/// Moves HEAD to `target`, going round through a stash when the working
/// tree is in the way (デザイン規約 §未コミット変更がある状態での移動).
pub(super) async fn move_carrying(
    executor: &GitExecutor,
    repo: &RepoInfo,
    target: &CheckoutTarget,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    match branch::checkout(executor, &repo.workdir, target, cancel).await? {
        branch::CheckoutOutcome::Moved => Ok(()),
        branch::CheckoutOutcome::Blocked(refusal) => {
            tracing::info!(%refusal, "move refused: going round through a stash");
            carry_across(executor, repo, target, refusal, cancel).await
        }
    }
}

/// Stash, move, put back — what a person would type when git will not
/// carry the work itself. `refusal` is what git said the first time, kept
/// for the dead ends that have nothing better to report.
///
/// The restore is a merge, so the changes land on top of what the target
/// has and only the parts git cannot combine need settling. Going through
/// a stash rather than `switch --merge` buys two things that flag cannot
/// give — **the staged/unstaged split survives** (`--index`), and a
/// conflict **keeps the stash entry**, so the work still exists somewhere
/// other than a marked-up file.
async fn carry_across(
    executor: &GitExecutor,
    repo: &RepoInfo,
    target: &CheckoutTarget,
    refusal: GitError,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    if !stash_everything(executor, repo, cancel).await? {
        // The tree was cleaned between the refusal and now, so there is
        // nothing of ours to carry and nothing of anybody else's to
        // touch: the move that was refused goes through as it stands.
        return match branch::checkout(executor, &repo.workdir, target, cancel).await? {
            branch::CheckoutOutcome::Moved => Ok(()),
            branch::CheckoutOutcome::Blocked(again) => Err(again),
        };
    }
    let outcome = match branch::checkout(executor, &repo.workdir, target, cancel).await {
        Ok(outcome) => outcome,
        Err(error) => {
            pop_back_after_failure(executor, repo, cancel).await;
            return Err(error);
        }
    };
    if let branch::CheckoutOutcome::Blocked(_) = outcome {
        // Nothing should stand in the way of a tree that was just
        // emptied, so whatever is holding this one is not something a
        // stash gets past (a `--skip-worktree` file, say). Put the work
        // back and let git's first refusal say why — a failure to put it
        // back is the more urgent news and goes through instead.
        stash::pop(executor, &repo.workdir, STASH_TOP, cancel).await?;
        return Err(refusal);
    }

    pop_back_split_first(executor, repo, cancel).await
}

/// Stashes the whole working tree out of a move's way, and answers whether
/// an entry of ours was really made.
///
/// A clean tree stashes nothing while exiting 0 — the refusal that raised
/// the question can go stale when the tree is cleaned from a terminal in
/// between. With no entry of ours, [`STASH_TOP`] names somebody else's
/// work and must not be touched.
async fn stash_everything(
    executor: &GitExecutor,
    repo: &RepoInfo,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let options = stash::PushOptions {
        include_untracked: true,
        keep_index: false,
        staged_only: false,
    };
    let before = stash::tip(executor, &repo.workdir, cancel).await?;
    stash::push(executor, &repo.workdir, "", options, &[], cancel).await?;
    Ok(stash::tip(executor, &repo.workdir, cancel).await? != before)
}

/// Puts the carried work back once the move or the rewrite has landed,
/// keeping the staged/unstaged split for as long as git will take it. The
/// last step of both carries, and the only one they share.
///
/// A conflicting restore exits non-zero while having done exactly what was
/// asked, so the exit code alone cannot judge it: the working tree decides.
/// Unmerged paths mean the merge landed and is waiting to be settled; a
/// clean tree means the restore did nothing, and then the split has to be
/// given up on (see below) or git's message goes through. Nothing was
/// unmerged when the carry began — the stash emptied the tree — so what is
/// found afterwards can only have come from the restore.
async fn pop_back_split_first(
    executor: &GitExecutor,
    repo: &RepoInfo,
    cancel: &CancellationToken,
) -> Result<(), GitError> {
    let kept_index = stash::pop_with_index(executor, &repo.workdir, STASH_TOP, cancel).await;
    if kept_index.is_ok() || conflicts_now(executor, repo, cancel).await? {
        return Ok(());
    }
    // git refuses `--index` outright when the staged half is what collides
    // ("conflicts in index. Try without --index.") and leaves everything
    // where it was. Its own advice is the fallback: restore without the
    // index, which brings the changes across merged and gives up only on
    // the staged/unstaged split.
    match stash::pop(executor, &repo.workdir, STASH_TOP, cancel).await {
        Ok(()) => Ok(()),
        Err(error) => {
            if conflicts_now(executor, repo, cancel).await? {
                Ok(())
            } else {
                Err(error)
            }
        }
    }
}

/// Best-effort restore after a move or a replay that failed outright (an
/// `Err`, not a `Blocked` refusal — a refusal git words in a way the
/// classifiers do not know arrives here). It did nothing, so the stash
/// was only the room it needed: put the work back before the caller
/// surfaces git's own error. If even the pop fails, that is logged and
/// the entry stays in the stash list, where the work is still
/// recoverable — the original error is the one worth showing.
async fn pop_back_after_failure(
    executor: &GitExecutor,
    repo: &RepoInfo,
    cancel: &CancellationToken,
) {
    if let Err(error) = stash::pop(executor, &repo.workdir, STASH_TOP, cancel).await {
        tracing::warn!(
            %error,
            "the switch failed and the stashed work could not be popped back; \
             it remains in the stash list"
        );
    }
}

/// Whether the working tree has unmerged paths right now.
///
/// What a restore leaves behind is the only honest answer to "did that
/// non-zero exit do anything": `git stash pop` reports a conflict and a
/// refusal the same way.
pub(super) async fn conflicts_now(
    executor: &GitExecutor,
    repo: &RepoInfo,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    Ok(status::load(executor, &repo.workdir, cancel)
        .await?
        .has_conflicts())
}

/// One sifted stream entry (stash rows carry their reflog selector).
pub(super) struct StreamItem {
    pub(super) meta: CommitMeta,
    pub(super) stash_ref: Option<String>,
}

/// Filters a parsed batch for display: stash commits keep only their
/// first-parent edge (the base commit), and their synthetic index /
/// untracked parent commits are recorded and dropped when they arrive
/// later (the walk shows no parent before all of its children, so the
/// stash row always streams first).
pub(super) fn sift_batch(
    batch: Vec<CommitMeta>,
    stash_refs: &HashMap<Oid, String>,
    skip: &mut std::collections::HashSet<Oid>,
    out: &mut Vec<StreamItem>,
) {
    for mut meta in batch {
        if skip.contains(&meta.oid) {
            continue;
        }
        let stash_ref = stash_refs.get(&meta.oid).cloned();
        if stash_ref.is_some() && meta.parents.len() > 1 {
            for extra in &meta.parents[1..] {
                skip.insert(*extra);
            }
            meta.parents = Box::from(&meta.parents[..1]);
        }
        out.push(StreamItem { meta, stash_ref });
    }
}

/// Builds one display row from a commit (labels attached by the caller).
/// `dashed_edge` draws the first-parent edge dashed (stash rows).
pub(super) fn make_row(
    commit: &CommitMeta,
    pool: &crate::model::StrPool,
    builder: &mut GraphBuilder,
    dashed_edge: bool,
) -> LogRow {
    let g = builder.push_with_edge_style(commit, dashed_edge);
    LogRow {
        row: g.row,
        oid_hex: commit.oid.to_hex(),
        short_sha: commit.oid.short_hex(8),
        author: pool.get(commit.author).to_string(),
        author_email: pool.get(commit.author_email).to_string(),
        co_authors: commit
            .co_authors
            .iter()
            .map(|(name, email)| crate::details::CoAuthor {
                name: pool.get(*name).to_string(),
                email: pool.get(*email).to_string(),
            })
            .collect(),
        time: commit.time,
        subject: commit.subject.to_string(),
        body: commit.body.to_string(),
        node_lane: g.node_lane,
        node_color: g.node_color,
        width: g.width,
        segments: g.segments,
        labels: Vec::new(),
        stash_ref: String::new(),
    }
}

/// The per-listing lookups the two ref joins share, built once so neither
/// of them scans the listing from inside a loop.
///
/// Both joins run on every refs read — which is every poll tick — and both
/// used to answer "is there a remote for this branch", "is this remote
/// already spoken for" and "does this repository hold this tag" by walking
/// the whole listing again. On `JetBrains/kotlin` (53,614 refs, 45,782 of
/// them tags) that made one read cost seconds; through these it is
/// milliseconds.
pub(super) struct RefJoins<'a> {
    remotes: refs::RemoteBranches<'a>,
    /// Remote branches whose chip a local branch already carries.
    folded: std::collections::HashSet<&'a str>,
    /// Where each tag this repository holds points, by short name. Answers
    /// both "is this name here" and "is it on the same commit as there".
    tag_commit: HashMap<&'a str, Oid>,
}

impl<'a> RefJoins<'a> {
    pub(super) fn new(refs: &'a [RefEntry]) -> Self {
        let remotes = refs::RemoteBranches::index(refs);
        let folded = remotes.folded_into_local(refs);
        let tag_commit = refs
            .iter()
            .filter(|r| r.kind == RefKind::Tag)
            .map(|r| (r.short.as_str(), r.commit_oid()))
            .collect();
        Self {
            remotes,
            folded,
            tag_commit,
        }
    }
}

/// Builds the per-commit label chips from a refs listing and what the
/// remotes carry under `refs/tags/`.
///
/// A branch and the remote it is about get one chip between them: the
/// cloud badge already says the remote is here, so the remote's own label
/// is dropped (see [`refs::remotes_folded_into_local`]).
///
/// Tags fold on the same terms, but only when both sides point at the same
/// commit. One that points elsewhere over there gets a label of its own on
/// the row it is really on, so the same name stands on two rows — the whole
/// of that signal, since a fetch never resolves the disagreement (measured:
/// `--prune` leaves the local tag silently) and it has to keep showing.
pub(super) fn build_label_map(
    refs: &[RefEntry],
    head: &HeadState,
    remote_tags: &RemoteTagIndex,
    joins: &RefJoins<'_>,
) -> HashMap<Oid, Vec<RefLabel>> {
    let mut map: HashMap<Oid, Vec<RefLabel>> = HashMap::new();
    for r in refs {
        if r.kind == RefKind::RemoteBranch && joins.folded.contains(r.name.as_str()) {
            continue;
        }
        let kind = match r.kind {
            RefKind::LocalBranch => LabelKind::LocalBranch,
            RefKind::RemoteBranch => LabelKind::RemoteBranch,
            RefKind::Tag => LabelKind::Tag,
        };
        let has_remote = match r.kind {
            RefKind::LocalBranch => joins.remotes.has_counterpart(r),
            RefKind::Tag => remote_tags.contains_key(&r.short),
            RefKind::RemoteBranch => false,
        };
        map.entry(r.commit_oid()).or_default().push(RefLabel {
            text: r.short.clone(),
            kind,
            has_remote,
            is_head: r.is_head,
            here: r.kind != RefKind::RemoteBranch,
            remote: String::new(),
        });
    }
    for (name, commits) in remote_tags {
        let local = joins.tag_commit.get(name.as_str()).copied();
        for (oid, place) in commits {
            // Where the two agree there is one tag to speak of, and the
            // local label is already carrying its cloud.
            if local == Some(*oid) {
                continue;
            }
            map.entry(*oid).or_default().push(RefLabel {
                text: name.clone(),
                kind: LabelKind::Tag,
                has_remote: true,
                is_head: false,
                here: false,
                remote: place.remotes.join(", "),
            });
        }
    }
    if head.detached
        && let Some(oid) = head.oid
    {
        map.entry(oid).or_default().push(RefLabel {
            text: "HEAD".to_string(),
            kind: LabelKind::Head,
            has_remote: false,
            is_head: true,
            here: true,
            remote: String::new(),
        });
    }
    for labels in map.values_mut() {
        labels.sort_by(|a, b| {
            (!a.is_head, a.kind, a.text.as_str()).cmp(&(!b.is_head, b.kind, b.text.as_str()))
        });
    }
    map
}

/// Fingerprint of what status reported: which paths, in which state.
///
/// Deliberately not the branch headers — ahead/behind move when a fetch
/// lands and say nothing about anyone's line endings.
pub(super) fn status_key(status: &WorkTreeStatus) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for item in &status.items {
        item.hash(&mut hasher);
    }
    hasher.finish()
}

/// Fingerprints where every ref points, so two reads can be compared
/// without keeping the listing around.
///
/// Only what moves the walk counts: a renamed upstream or a changed sort
/// date redraws chips through the label diff, and rebuilding for those
/// would repaint the graph over nothing. `git for-each-ref` lists in
/// refname order, so equal layouts hash equal.
pub(super) fn refs_key(refs: &[RefEntry], head: &HeadState) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for entry in refs {
        entry.name.hash(&mut hasher);
        entry.target.hash(&mut hasher);
        entry.peeled.hash(&mut hasher);
    }
    head.branch.hash(&mut hasher);
    head.oid.hash(&mut hasher);
    head.detached.hash(&mut hasher);
    hasher.finish()
}

/// Builds the sorted sidebar snapshot.
///
/// Tags a remote has and this repository does not are listed too: no local
/// ref puts them on a graph row, so the sidebar is the only place they can
/// be read at all. Where a name exists on both sides it is listed once —
/// the sidebar is a list of names to act on, and which commits the two
/// sides point at is what the graph rows are for.
pub(super) fn build_snapshot(
    refs: &[RefEntry],
    head: &HeadState,
    remote_tags: &RemoteTagIndex,
    joins: &RefJoins<'_>,
) -> RefsSnapshot {
    let mut snapshot = RefsSnapshot {
        head: Some(head.clone()),
        ..Default::default()
    };
    for r in refs {
        match r.kind {
            RefKind::LocalBranch => snapshot.locals.push(BranchItem {
                short: r.short.clone(),
                full: r.name.clone(),
                oid_hex: r.commit_oid().to_hex(),
                has_remote: joins.remotes.has_counterpart(r),
                is_head: r.is_head,
                upstream: joins
                    .remotes
                    .spoken_for(r)
                    .map(|u| u.short.clone())
                    .unwrap_or_default(),
            }),
            RefKind::RemoteBranch => snapshot.remotes.push(BranchItem {
                short: r.short.clone(),
                full: r.name.clone(),
                oid_hex: r.commit_oid().to_hex(),
                has_remote: true,
                is_head: false,
                upstream: String::new(),
            }),
            RefKind::Tag => snapshot.tags.push(TagItem {
                short: r.short.clone(),
                oid_hex: r.commit_oid().to_hex(),
                annotated: r.peeled.is_some(),
                created_unix: r.created_unix,
                has_remote: remote_tags.contains_key(&r.short),
                here: true,
            }),
        }
    }
    for (name, commits) in remote_tags {
        if joins.tag_commit.contains_key(name.as_str()) {
            continue;
        }
        // Remotes that disagree about a name still name one tag, and the
        // sidebar answers "does this name exist" rather than "where".
        let Some((oid, place)) = commits.iter().next() else {
            continue;
        };
        snapshot.tags.push(TagItem {
            short: name.clone(),
            oid_hex: oid.to_hex(),
            annotated: place.annotated,
            // An advertisement carries no date; these sort last, after
            // every tag whose creation this repository can see.
            created_unix: 0,
            has_remote: true,
            here: false,
        });
    }
    snapshot.locals.sort_by(|a, b| a.short.cmp(&b.short));
    snapshot.remotes.sort_by(|a, b| a.short.cmp(&b.short));
    // Tags newest-first (product decision), name as the tie-breaker.
    snapshot.tags.sort_by(|a, b| {
        b.created_unix
            .cmp(&a.created_unix)
            .then(a.short.cmp(&b.short))
    });
    snapshot
}
