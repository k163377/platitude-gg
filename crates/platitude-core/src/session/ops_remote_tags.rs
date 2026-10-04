//! What each remote carries under `refs/tags/`, and the writes that move
//! it: reading it, and sending, replacing or taking away one tag.
//!
//! Apart from [`super::ops_remote`] because a tag's whereabouts on a
//! remote has to be asked for outright (every other sidebar row reads
//! `refs/remotes/`), so every write here reads again on its way out.

use super::discard_record::{held_before, record_token};
use super::*;
use crate::discards::{self, TagBefore};

/// What one pass over the remotes' tags established.
#[derive(Debug, Default)]
pub(super) struct RemoteTagsRead {
    /// Whether what the remotes carry moved — the caller's reason to
    /// republish.
    pub moved: bool,
    /// What was asked and did not answer, named with what git said.
    /// Empty when everything asked answered (or nothing was asked).
    pub unread: Vec<String>,
}

impl RepoSession {
    /// The fetch, and then where the remotes keep their tags: a fetched
    /// tag lands in `refs/tags/` beside the local ones, and afterwards
    /// nothing local tells them apart. The second round trip belongs here
    /// — the user has already agreed to reach the network.
    pub(super) async fn fetch_and_read_tags(
        self: &Arc<Self>,
        exec: &GitExecutor,
        workdir: &Path,
        remote_name: Option<&str>,
        timeout: std::time::Duration,
        cancel: &CancellationToken,
    ) -> Result<(), GitError> {
        remote::fetch(exec, workdir, remote_name, timeout, cancel).await?;
        self.read_remote_tags(exec, workdir, remote_name, timeout, cancel)
            .await;
        Ok(())
    }

    /// Records what each remote advertises under `refs/tags/`.
    ///
    /// **Fails nothing**: a badge is not worth failing a fetch that
    /// worked, and an unreachable remote keeps its last answer, as
    /// `refs/remotes/` does for branches. What could not be read is still
    /// named ([`RemoteTagsRead::unread`]) for the tracked catch-up
    /// ([`RemoteTagRefreshOutcome::Unanswered`]); the fire-and-forget path
    /// ignores it.
    pub(super) async fn read_remote_tags(
        &self,
        exec: &GitExecutor,
        workdir: &Path,
        only: Option<&str>,
        timeout: std::time::Duration,
        cancel: &CancellationToken,
    ) -> RemoteTagsRead {
        let remotes = match self.remotes(workdir, cancel).await {
            Ok(read) => read.list,
            Err(error) => {
                tracing::debug!(%error, "remote tags: the remotes could not be listed");
                return RemoteTagsRead {
                    moved: false,
                    unread: vec![format!("the remote list: {error}")],
                };
            }
        };
        let mut answered: Vec<crate::Name> = Vec::new();
        let mut unread: Vec<String> = Vec::new();
        let mut fresh: Vec<(crate::Name, Oid, bool, crate::Name)> = Vec::new();
        for r in &remotes {
            if only.is_some_and(|wanted| wanted != r.name) {
                continue;
            }
            match remote::list_tags(exec, workdir, &r.name, timeout, cancel).await {
                Ok(tags) => {
                    let remote = crate::Name::from(r.name.as_str());
                    answered.push(remote.clone());
                    fresh.extend(
                        tags.into_iter()
                            .map(|t| (t.name, t.commit, t.annotated, remote.clone())),
                    );
                }
                // The session is going away: nobody is left to be told.
                Err(error) if error.is_cancelled() => return RemoteTagsRead::default(),
                Err(error) => {
                    tracing::debug!(remote = %r.name, %error, "remote tags: unreadable");
                    unread.push(format!("{}: {error}", r.name));
                }
            }
        }
        RemoteTagsRead {
            moved: self.remerge_remote_tags(&remotes, &answered, fresh),
            unread,
        }
    }

    /// Rebuilds the index with `fresh` in place of what `answered` said
    /// last, and says whether that changed anything. The only place the
    /// index is built. What it keeps comes out of the index itself, so the
    /// readings are held once ([`RemoteTagIndex::readings`]).
    fn remerge_remote_tags(
        &self,
        configured: &[remote::Remote],
        answered: &[crate::Name],
        fresh: Vec<(crate::Name, Oid, bool, crate::Name)>,
    ) -> bool {
        let current = self.remote_tag_index();
        let kept = current.readings().filter(|(_, _, _, remote)| {
            configured.iter().any(|r| r.name == remote.as_str())
                && !answered.iter().any(|a| a == remote)
        });
        let index = RemoteTagIndex::build(kept.chain(fresh.iter().cloned()));
        let mut slot = relock(&self.remote_tag_index);
        if **slot == index || !self.keeps_what_it_reads() {
            return false;
        }
        *slot = Arc::new(index);
        self.remote_tag_gen.fetch_add(1, Ordering::SeqCst);
        true
    }

    /// The per-remote answers merged into the index the join reads.
    pub(super) fn remote_tag_index(&self) -> Arc<RemoteTagIndex> {
        Arc::clone(&relock(&self.remote_tag_index))
    }

    /// Sends one tag to one remote. `expect` pins a leased overwrite to
    /// the commit that remote was last seen holding the tag on; empty
    /// sends it plain (see [`remote::push_tag`]).
    ///
    /// **The badge is re-read inside the write**: what a remote carries
    /// under `refs/tags/` has no local record, so the refs refresh after it
    /// would not notice (rules-refs/core.md「タグのリモート状態」). Inside,
    /// as in [`Self::fetch_and_read_tags`]: the press has already agreed to
    /// reach the network. A refused push moved nothing and reads nothing —
    /// except one refused for a lease the remote has left
    /// ([`crate::ReportKind::is_outdated`]), which reads again to show where
    /// the name went, as a branch's catch-up fetch does ([`tags_moved`]).
    ///
    /// **The refresh comes after the closure returns**: `AfterWrite::Graph`
    /// reads the refs then, and asking for one inside would be a second
    /// pass over every ref — the longest read this application makes.
    pub fn push_tag(
        self: &Arc<Self>,
        remote_name: String,
        tag: String,
        expect: String,
    ) -> Option<OperationId> {
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        self.write(
            OperationKind::Push,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let result = remote::push_tag(
                    &exec,
                    &repo.workdir,
                    &remote_name,
                    &tag,
                    &expect,
                    timeout,
                    &cancel,
                )
                .await;
                if tags_moved(&result) {
                    s.read_remote_tags(&exec, &repo.workdir, Some(&remote_name), timeout, &cancel)
                        .await;
                }
                // A leased push overwrote what the remote held, as the lease
                // read it: that goes on the discard record (破棄記録仕様.md §2).
                if let Some(replaced) = result? {
                    let keep = record_token();
                    s.recorded_if(
                        tag_pushed_over(&exec, &repo, &remote_name, &tag, &replaced, &keep).await,
                    );
                }
                Ok(())
            },
        )
    }

    /// Takes one tag off one remote, leaving whatever is here, leased to
    /// `expect`, the commit the screen showed it on
    /// ([`remote::delete_remote_tag`]). Reads that remote's tags
    /// afterwards, as [`Self::push_tag`] does.
    pub fn delete_remote_tag(
        self: &Arc<Self>,
        remote_name: String,
        tag: String,
        expect: String,
    ) -> Option<OperationId> {
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        self.write(
            OperationKind::Push,
            AfterWrite::Name { status: false },
            move |exec, repo, cancel| async move {
                let result = remote::delete_remote_tag(
                    &exec,
                    &repo.workdir,
                    &remote_name,
                    &tag,
                    &expect,
                    timeout,
                    &cancel,
                )
                .await;
                if tags_moved(&result) {
                    s.read_remote_tags(&exec, &repo.workdir, Some(&remote_name), timeout, &cancel)
                        .await;
                }
                let held = result?;
                let keep = record_token();
                s.recorded_if(
                    tags_gone(&exec, &repo, &remote_name, &tag, None, held.as_ref(), &keep).await,
                );
                Ok(())
            },
        )
    }

    /// Replaces a tag on a remote with one under a new name, which git
    /// does as a push and a delete (see [`remote::replace_remote_tag`]),
    /// the delete leased to `expect`. The UI asks first: the old name is
    /// destroyed. Reads that remote's tags afterwards, as
    /// [`Self::push_tag`] does.
    pub fn replace_remote_tag(
        self: &Arc<Self>,
        remote_name: String,
        from: String,
        to: String,
        expect: String,
    ) -> Option<OperationId> {
        let timeout = self.network_timeout();
        let s = Arc::clone(self);
        self.write(
            OperationKind::Push,
            AfterWrite::Graph,
            move |exec, repo, cancel| async move {
                let result = remote::replace_remote_tag(
                    &exec,
                    &repo.workdir,
                    &remote_name,
                    &from,
                    &to,
                    &expect,
                    timeout,
                    &cancel,
                )
                .await;
                if tags_moved(&result) {
                    s.read_remote_tags(&exec, &repo.workdir, Some(&remote_name), timeout, &cancel)
                        .await;
                }
                let held = result?;
                let keep = record_token();
                s.recorded_if(
                    tags_gone(
                        &exec,
                        &repo,
                        &remote_name,
                        &from,
                        None,
                        held.as_ref(),
                        &keep,
                    )
                    .await,
                );
                Ok(())
            },
        )
    }

    /// Deletes a tag here and on the remote as one queued write, the
    /// remote half leased to `expect` as in [`Self::delete_remote_tag`].
    ///
    /// **The remote half goes first**, unlike
    /// [`Self::delete_branch_everywhere`]: a refused lease then stops the
    /// pair with nothing touched, where the other order would already have
    /// dropped the name here — with no reflog behind it. The local half
    /// after it refuses only a name that is not here, which the menu does
    /// not offer. The commits only the tag held leave the graph at once
    /// (`session::leaving`).
    pub fn delete_tag_everywhere(
        self: &Arc<Self>,
        tag: String,
        remote_name: String,
        expect: String,
    ) -> Option<OperationId> {
        let timeout = self.network_timeout();
        let name = tag.clone();
        let s = Arc::clone(self);
        // A kind of its own: it answers as a tag write but runs on the
        // remote lane, since the far end paces the first half and the read.
        self.write_taking(
            OperationKind::DeleteTagEverywhere,
            AfterWrite::Name { status: false },
            &[LeavingRef::Tag(&name)],
            move |exec, repo, cancel| async move {
                let local = format!("refs/tags/{tag}");
                let here =
                    discards::tag_before(&exec, &repo.workdir, &tag, &local, &cancel).await?;
                let remote_half = remote::delete_remote_tag(
                    &exec,
                    &repo.workdir,
                    &remote_name,
                    &tag,
                    &expect,
                    timeout,
                    &cancel,
                )
                .await;
                // The remote's tags are read whatever the local half says:
                // the name is gone over there even where it stays here.
                let local_half = if remote_half.is_ok() {
                    tag::delete(&exec, &repo.workdir, &tag, &cancel).await
                } else {
                    Ok(())
                };
                if tags_moved(&remote_half) {
                    s.read_remote_tags(&exec, &repo.workdir, Some(&remote_name), timeout, &cancel)
                        .await;
                }
                // What went: the remote's name as the lease read it, and the
                // one here with it.
                if let Ok(held) = &remote_half {
                    let here = here.filter(|_| local_half.is_ok());
                    let keep = record_token();
                    let written = tags_gone(
                        &exec,
                        &repo,
                        &remote_name,
                        &tag,
                        here.as_ref(),
                        held.as_ref(),
                        &keep,
                    )
                    .await;
                    s.recorded_if(written);
                }
                remote_half.map(drop).and(local_half)
            },
        )
    }
}

/// Whether a remote-tag write's answer calls for reading that remote's
/// tags again: it landed, or the remote was found holding the name elsewhere
/// ([`crate::ReportKind::is_outdated`]) — the read is what shows where.
fn tags_moved<T>(result: &Result<T, GitError>) -> bool {
    result.as_ref().map_or_else(GitError::is_outdated, |_| true)
}

/// Puts a leased push's overwrite on the record: what the remote held, unless
/// it was the very object that went up. Whether a line went on.
async fn tag_pushed_over(
    exec: &GitExecutor,
    repo: &RepoInfo,
    remote_name: &str,
    tag: &str,
    replaced: &remote::HeldTag,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let here = format!("refs/tags/{tag}");
    let sent = discards::tag_before(exec, &repo.workdir, tag, &here, cancel).await?;
    if sent.is_some_and(|sent| sent.object == replaced.object) {
        return Ok(false);
    }
    let before = held_before(exec, &repo.workdir, tag, replaced, cancel).await?;
    discards::record_tag_force_push(
        exec,
        &repo.workdir,
        &repo.git_dir,
        remote_name,
        &before,
        cancel,
    )
    .await?;
    Ok(true)
}

/// Puts a tag's delete on the record: the name here as it was, and the
/// remote's as the lease read it. Whether a line went on.
async fn tags_gone(
    exec: &GitExecutor,
    repo: &RepoInfo,
    remote_name: &str,
    tag: &str,
    here: Option<&TagBefore>,
    held: Option<&remote::HeldTag>,
    cancel: &CancellationToken,
) -> Result<bool, GitError> {
    let there = match held {
        Some(held) => Some(held_before(exec, &repo.workdir, tag, held, cancel).await?),
        None => None,
    };
    if here.is_none() && there.is_none() {
        return Ok(false);
    }
    discards::record_tag_delete(
        exec,
        &repo.workdir,
        &repo.git_dir,
        here,
        there.as_ref().map(|there| (remote_name, there)),
        cancel,
    )
    .await?;
    Ok(true)
}
