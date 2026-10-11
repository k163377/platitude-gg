//! Presets where a write is turned down by something only git's run can
//! tell — a hook here, a signing key, another git's lock, the far side's
//! own settings — for the report each brings down (デザイン規約
//! §答えの要らない報せ).

use super::basic::{basic, plan, stashes};
use super::remote::{diverged, hooked, install_hook};
use super::repo::{DemoRepo, file_url};
use super::signing::config_path;

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
        _ => return None,
    };
    Some(made)
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
