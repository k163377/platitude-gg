//! Presets where a write is turned down by something only git's run can
//! tell — a hook here, a signing key, another git's lock, the far side's
//! own settings — for the report each brings down (デザイン規約
//! §答えの要らない報せ).

use super::basic::{basic, dirty, plan, stashes};
use super::remote::{diverged, hooked, install_hook, unpublished};
use super::repo::{DemoRepo, file_url};
use super::signing::config_path;
use super::worktrees::carried;

/// Builds `preset` if it is one of this module's; `None` for the rest
/// (`super::presets::build`).
pub(super) fn build(preset: &str, repo: &mut DemoRepo) -> Option<Result<(), String>> {
    let made = match preset {
        "hooked-auto" => hooked_auto(repo),
        "unsigned" => unsigned(repo),
        "prepush" => prepush(repo),
        "nonff" => nonff(repo),
        "toolarge" => toolarge(repo),
        "checkedout" => checkedout(repo),
        "prerebase" => prerebase(repo),
        "plan-unsigned" => plan_unsigned(repo),
        "stash-lock" => stash_lock(repo),
        "stash-droplock" => stash_droplock(repo),
        "postcheckout" => postcheckout(repo),
        "signed-push" => signed_push(repo),
        "push-unsigned" => push_unsigned(repo),
        "prepush-config" => prepush_config(repo),
        "stage-lock" => stage_lock(repo),
        "plan-lock" => plan_lock(repo),
        "plan-untracked" => plan_untracked(repo),
        "squash-hooked" => squash_hooked(repo),
        "carried-lock" => carried_lock(repo),
        "reftable-lock" => reftable_lock(repo),
        "reftable-droplock" => reftable_droplock(repo),
        "remote-lock" => remote_lock(repo),
        "tag-hidden" => tag_hidden(repo),
        "publish-unreachable" => publish_unreachable(repo),
        _ => return None,
    };
    Some(made)
}

/// Writes an empty lock file at `rel` under the repository's `.git` — another
/// git holding it, or one that crashed and left it.
fn lock(repo: &DemoRepo, rel: &str) -> Result<(), String> {
    let lock = repo.work.join(".git").join(rel);
    std::fs::write(&lock, "").map_err(|e| format!("writing {}: {e}", lock.display()))
}

/// [`ahead_of_origin`] asking for a signed push (`push.gpgSign=true`) of a
/// remote that takes none — no `receive.certNonceSeed` there.
fn signed_push(repo: &mut DemoRepo) -> Result<(), String> {
    ahead_of_origin(repo)?;
    repo.git(&["config", "push.gpgSign", "true"])?;
    Ok(())
}

/// The same against a remote that takes signed pushes, with the ssh key to
/// sign with not there.
fn push_unsigned(repo: &mut DemoRepo) -> Result<(), String> {
    signed_push(repo)?;
    let origin = repo.root.join("origin.git");
    repo.git_at(
        &origin,
        &["config", "receive.certNonceSeed", "a seed of its own"],
    )?;
    let key = repo.root.join("no-such-key.pub");
    repo.git(&["config", "gpg.format", "ssh"])?;
    repo.git(&["config", "user.signingkey", &config_path(&key)])?;
    Ok(())
}

/// [`prepush`]'s no from a hook the config names (`hook.<name>.event`) —
/// git 2.54 and later; the minimum runs no such hook, and the push goes.
fn prepush_config(repo: &mut DemoRepo) -> Result<(), String> {
    ahead_of_origin(repo)?;
    repo.git(&["config", "hook.lint.event", "pre-push"])?;
    repo.git(&[
        "config",
        "hook.lint.command",
        "echo \"lint: 1 problem found, nothing pushed\" >&2; exit 1",
    ])?;
    Ok(())
}

/// [`dirty`] with another git holding the index's lock: nothing can be
/// staged, unstaged or discarded.
fn stage_lock(repo: &mut DemoRepo) -> Result<(), String> {
    dirty(repo)?;
    lock(repo, "index.lock")
}

/// [`plan`] with another git holding a lock a remade commit needs —
/// `CHERRY_PICK_HEAD`'s, which every replayed pick writes and the abort does
/// not: the replay stops on it and is put back.
fn plan_lock(repo: &mut DemoRepo) -> Result<(), String> {
    plan(repo)?;
    lock(repo, "CHERRY_PICK_HEAD.lock")
}

/// A straight run whose second-newest commit takes a file away that is
/// standing untracked now: dropping that commit would check out the one
/// under it, file and all, over the untracked copy (`drop-commit-go row:2`
/// — the untracked file stands the worktree's row above HEAD's).
fn plan_untracked(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("notes.txt", "v1\n", "docs: start the notes")?;
    repo.commit("src/old.txt", "old helper\n", "feat: an old helper")?;
    repo.git(&["rm", "-q", "--", "src/old.txt"])?;
    repo.commit(
        "src/new.txt",
        "new helper\n",
        "refactor: replace the old helper",
    )?;
    repo.commit("src/two.txt", "two\n", "feat: carry on")?;
    repo.write("src/old.txt", "a copy kept by hand\n")
}

/// [`plan`] with a `prepare-commit-msg` hook that turns every message down:
/// a fold remakes the commit it folds into with it.
fn squash_hooked(repo: &mut DemoRepo) -> Result<(), String> {
    plan(repo)?;
    install_hook(
        &repo.work.join(".git").join("hooks"),
        "prepare-commit-msg",
        "#!/bin/sh\n\
         echo \"commit messages here come from the ticket template\" >&2\n\
         exit 1\n",
    )
}

/// [`carried`] with another git holding this worktree's index lock: no other
/// worktree's changes can be applied here.
fn carried_lock(repo: &mut DemoRepo) -> Result<(), String> {
    carried(repo)?;
    lock(repo, "index.lock")
}

/// [`basic`] in the reftable ref store (git 2.48+ migrates it in place),
/// with the store's one lock taken: git's rename of a branch then names no
/// lock at all.
fn reftable_lock(repo: &mut DemoRepo) -> Result<(), String> {
    basic(repo)?;
    repo.git(&["refs", "migrate", "--ref-format=reftable"])?;
    lock(repo, "reftable/tables.list.lock")
}

/// [`stashes`] in the reftable ref store, its one lock taken the moment a
/// new entry goes in — by a `reference-transaction` hook, as
/// [`stash_droplock`]'s: the renamed entry goes in, the old one's drop says
/// only `Could not drop stash entry`, naming no lock.
fn reftable_droplock(repo: &mut DemoRepo) -> Result<(), String> {
    stashes(repo)?;
    repo.git(&["refs", "migrate", "--ref-format=reftable"])?;
    install_hook(
        &repo.work.join(".git").join("hooks"),
        "reference-transaction",
        "#!/bin/sh\n\
         [ \"$1\" = committed ] || exit 0\n\
         while read -r old new ref; do\n\
         if [ \"$ref\" = refs/stash ]; then\n\
         : > \"$(git rev-parse --git-dir)/reftable/tables.list.lock\"\n\
         fi\n\
         done\n\
         exit 0\n",
    )
}

/// [`ahead_of_origin`] with another git holding `main`'s lock over there:
/// the far side turns the push down in its own words for the lock.
fn remote_lock(repo: &mut DemoRepo) -> Result<(), String> {
    ahead_of_origin(repo)?;
    let lock = repo.root.join("origin.git/refs/heads/main.lock");
    std::fs::write(&lock, "").map_err(|e| format!("writing {}: {e}", lock.display()))
}

/// `v1` on `origin` and, moved, here — with `origin` hiding it from reads
/// (`uploadpack.hideRefs`) but not from a push's own look: git turns the
/// plain push down as `already exists`, and a read of the remote's tags
/// right after finds no `v1` on any commit (`remote::push_tag`).
fn tag_hidden(repo: &mut DemoRepo) -> Result<(), String> {
    ahead_of_origin(repo)?;
    repo.git(&["tag", "v1", "HEAD~1"])?;
    repo.git(&["push", "origin", "v1"])?;
    repo.git(&["tag", "--force", "v1", "HEAD"])?;
    let origin = repo.root.join("origin.git");
    repo.git_at(&origin, &["config", "uploadpack.hideRefs", "refs/tags/v1"])?;
    Ok(())
}

/// [`unpublished`] with `origin` nowhere to be reached: asking it what it
/// holds for the branch fails (the publish bar's `Asking origin failed`).
fn publish_unreachable(repo: &mut DemoRepo) -> Result<(), String> {
    unpublished(repo)?;
    repo.git(&[
        "remote",
        "set-url",
        "origin",
        "file:///C:/pgg-nowhere/gone.git",
    ])?;
    Ok(())
}

/// A clone with a commit of its own to push, `origin` holding the rest.
fn ahead_of_origin(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;
    repo.commit("src/app.txt", "app v2\n", "feat: carry on with the app")?;
    Ok(())
}

/// The `pre-push` hook here says no — a linter's complaint on stdout, its
/// noise on stderr, as husky and lefthook wrap one.
fn prepush(repo: &mut DemoRepo) -> Result<(), String> {
    ahead_of_origin(repo)?;
    install_hook(
        &repo.work.join(".git").join("hooks"),
        "pre-push",
        "#!/bin/sh\n\
         echo \"src/app.txt:1: missing licence header\"\n\
         echo \"lint: 1 problem found, nothing pushed\" >&2\n\
         exit 1\n",
    )
}

/// [`diverged`] onto a remote that takes no rewrite
/// (`receive.denyNonFastForwards`, on in every `git init --shared`): the
/// overwrite the toolbar offers is turned down over there.
fn nonff(repo: &mut DemoRepo) -> Result<(), String> {
    diverged(repo)?;
    let origin = repo.root.join("origin.git");
    repo.git_at(&origin, &["config", "receive.denyNonFastForwards", "true"])?;
    Ok(())
}

/// A remote that unpacks no more than ten bytes (`receive.maxInputSize`).
fn toolarge(repo: &mut DemoRepo) -> Result<(), String> {
    ahead_of_origin(repo)?;
    let origin = repo.root.join("origin.git");
    repo.git_at(&origin, &["config", "receive.maxInputSize", "10"])?;
    Ok(())
}

/// `origin` is a repository with a working tree of its own, `main`
/// checked out in it — a second clone on the same disk.
fn checkedout(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    let root = repo.root.clone();
    let url = file_url(&repo.work);
    repo.git_at(&root, &["clone", &url, "origin"])?;
    repo.git(&["remote", "add", "origin", &file_url(&root.join("origin"))])?;
    repo.git(&["fetch", "origin"])?;
    repo.git(&["branch", "--set-upstream-to=origin/main", "main"])?;
    repo.commit("src/app.txt", "app v2\n", "feat: carry on with the app")?;
    Ok(())
}

/// [`hooked`] under git 2.55's warning for `core.commentChar=auto`, which
/// it writes — ten lines of `hint:` with it — ahead of every refusal.
fn hooked_auto(repo: &mut DemoRepo) -> Result<(), String> {
    hooked(repo)?;
    repo.git(&["config", "core.commentChar", "auto"])?;
    Ok(())
}

/// Signing on, with a key that is not there.
fn sign_with_a_missing_key(repo: &mut DemoRepo) -> Result<(), String> {
    let key = repo.root.join("no-such-key.pub");
    for (key, value) in [
        ("gpg.format", "ssh".to_string()),
        ("user.signingkey", config_path(&key)),
        ("commit.gpgsign", "true".to_string()),
    ] {
        repo.git(&["config", key, &value])?;
    }
    Ok(())
}

/// Something staged, and signing on with a key that is not there.
fn unsigned(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    sign_with_a_missing_key(repo)?;
    repo.write("src/app.txt", "app v2\n")?;
    repo.git(&["add", "--", "src/app.txt"])?;
    Ok(())
}

/// [`plan`] with a `pre-rebase` hook that turns every rewrite down.
fn prerebase(repo: &mut DemoRepo) -> Result<(), String> {
    plan(repo)?;
    install_hook(
        &repo.work.join(".git").join("hooks"),
        "pre-rebase",
        "#!/bin/sh\n\
         echo \"main is shared: rewrite it on a branch of your own\" >&2\n\
         exit 1\n",
    )
}

/// [`plan`] with signing on and a key that is not there: a drop below the
/// newest commit remakes it, and that commit cannot be signed.
fn plan_unsigned(repo: &mut DemoRepo) -> Result<(), String> {
    plan(repo)?;
    sign_with_a_missing_key(repo)
}

/// [`stashes`] with another git holding the stash's lock: the renamed
/// entry cannot go in.
fn stash_lock(repo: &mut DemoRepo) -> Result<(), String> {
    stashes(repo)?;
    let lock = repo.work.join(".git/refs/stash.lock");
    std::fs::write(&lock, "").map_err(|e| format!("writing {}: {e}", lock.display()))
}

/// [`stashes`] where the stash's log is locked the moment a new entry goes
/// in — by a `reference-transaction` hook, standing in for another git
/// taking it between the two halves of a rename: the renamed entry goes
/// in, the old one cannot be dropped.
fn stash_droplock(repo: &mut DemoRepo) -> Result<(), String> {
    stashes(repo)?;
    install_hook(
        &repo.work.join(".git").join("hooks"),
        "reference-transaction",
        "#!/bin/sh\n\
         [ \"$1\" = committed ] || exit 0\n\
         while read -r old new ref; do\n\
         if [ \"$ref\" = refs/stash ]; then\n\
         : > \"$(git rev-parse --git-dir)/logs/refs/stash.lock\"\n\
         fi\n\
         done\n\
         exit 0\n",
    )
}

/// [`basic`] with a `post-checkout` hook that fails — git-lfs's own when
/// git-lfs is not installed: a worktree git makes is kept, with the hook's
/// failure as git's answer.
fn postcheckout(repo: &mut DemoRepo) -> Result<(), String> {
    basic(repo)?;
    install_hook(
        &repo.work.join(".git").join("hooks"),
        "post-checkout",
        "#!/bin/sh\n\
         echo \"This repository is configured for Git LFS but 'git-lfs' was not found on your path.\" >&2\n\
         exit 2\n",
    )
}
