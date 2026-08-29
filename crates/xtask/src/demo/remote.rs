//! Presets about where a branch stands against a remote.

use super::repo::{DemoRepo, file_url};

/// A branch that has never been sent anywhere, in a repository with two
/// remotes — so the question the first push raises has something to pick
/// between, and two names on the far side to run into.
///
/// The three names over there are the whole point, because a push meets
/// each of them differently (実測): `taken` left the trunk with a commit of
/// its own, so a push there is **refused**; `carried` is behind us on our
/// own line, so a push there **lands and moves somebody else's branch on**;
/// `outsider` was pushed from another clone and never fetched here, so
/// **neither answer can be given from this end**. The checked-out branch
/// has no upstream at all.
pub(super) fn unpublished(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;

    // A second place to send things, so the chooser has a choice to make.
    let fork = repo.root.join("fork.git");
    std::fs::create_dir_all(&fork).map_err(|e| e.to_string())?;
    repo.git_at(&fork.clone(), &["init", "--bare", "-b", "main"])?;
    let fork_url = file_url(&fork);
    repo.git(&["remote", "add", "fork", &fork_url])?;

    // Somebody else's branch, off the trunk with a commit we never take.
    // Pushed without `-u`, so nothing here records that it went anywhere.
    repo.git(&["switch", "--create", "taken"])?;
    repo.commit("src/app.txt", "app v2\n", "feat: theirs")?;
    repo.git(&["push", "origin", "taken"])?;

    // A third name, put there by somebody else and never fetched here: the
    // commit it holds is not in this repository, so the two histories
    // cannot be compared from this end at all.
    let seeder = repo.root.join("seeder");
    let url = file_url(&repo.root.join("origin.git"));
    let root = repo.root.clone();
    repo.git_at(&root, &["clone", &url, "seeder"])?;
    for (key, value) in [
        ("user.name", "Away Colleague"),
        ("user.email", "away@example.com"),
    ] {
        repo.git_at(&seeder.clone(), &["config", key, value])?;
    }
    repo.git_at(&seeder.clone(), &["switch", "--create", "outsider"])?;
    std::fs::write(seeder.join("elsewhere.txt"), "not here\n").map_err(|e| e.to_string())?;
    repo.git_at(&seeder.clone(), &["add", "--", "elsewhere.txt"])?;
    repo.git_at(&seeder.clone(), &["commit", "-m", "feat: somewhere else"])?;
    repo.git_at(&seeder.clone(), &["push", "origin", "outsider"])?;

    repo.git(&["switch", "main"])?;
    repo.git(&["switch", "--create", "feature/new-thing"])?;
    repo.commit("src/new.txt", "new\n", "feat: draft the new thing")?;
    // Sent from here under a second name, then left behind: the name is
    // taken over there by a commit this branch still contains, which is
    // the half git carries rather than refuses.
    repo.git(&["push", "origin", "HEAD:refs/heads/carried"])?;
    repo.commit("src/new.txt", "new v2\n", "feat: finish the new thing")?;
    Ok(())
}

/// A fork workflow: the branch goes on fetching from `origin`, and its
/// own mark (`branch.main.pushRemote`) sends every push of it to `fork`.
///
/// **The arrangement `remote.pushDefault` cannot stand in for.** git
/// weighs the branch's mark first and the repository's second
/// (git-config(5); 実測 2.55), so a destination worked out from the
/// repository's alone names `origin` here while the push goes to the
/// fork — and the counts beside it, which are about origin, are then
/// about somewhere else entirely (`push-target`, デザイン規約 §リモートへ送る).
///
/// The commit of our own is what makes the button live and gives those
/// counts a number to be wrong with.
pub(super) fn forkmark(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;

    let fork = repo.root.join("fork.git");
    std::fs::create_dir_all(&fork).map_err(|e| e.to_string())?;
    repo.git_at(&fork.clone(), &["init", "--bare", "-b", "main"])?;
    let fork_url = file_url(&fork);
    repo.git(&["remote", "add", "fork", &fork_url])?;
    // Only where the pushes go. `branch.main.remote` goes on saying
    // `origin`, which is the whole shape of a fork checkout.
    repo.git(&["config", "branch.main.pushRemote", "fork"])?;

    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    Ok(())
}

/// The remote holds commits a fetch would bring in (made by a second
/// clone). This repo has not fetched yet.
pub(super) fn behind(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("a.txt", "v1\n", "feat: one")?;
    repo.commit("a.txt", "v2\n", "feat: two")?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;

    let seeder = repo.root.join("seeder");
    let url = file_url(&repo.root.join("origin.git"));
    let root = repo.root.clone();
    repo.git_at(&root, &["clone", &url, "seeder"])?;
    for (key, value) in [
        ("user.name", "Away Colleague"),
        ("user.email", "away@example.com"),
    ] {
        repo.git_at(&seeder.clone(), &["config", key, value])?;
    }
    std::fs::write(seeder.join("a.txt"), "v2\nremote work\n").map_err(|e| e.to_string())?;
    repo.git_at(&seeder.clone(), &["add", "--", "a.txt"])?;
    repo.git_at(
        &seeder.clone(),
        &["commit", "-m", "feat: pushed while you slept"],
    )?;
    repo.git_at(&seeder.clone(), &["push"])?;
    Ok(())
}

/// Both sides moved on, and this repository has already seen it happen:
/// the fetch is part of the preset, so the toolbar offers the overwrite
/// (`push -f`) from the moment the window opens rather than after a verb.
///
/// Break the remote's URL afterwards (`.git/config`) and the overwrite
/// fails without the tracking refs moving — which is how the refused shape
/// of that button gets photographed.
pub(super) fn diverged(repo: &mut DemoRepo) -> Result<(), String> {
    behind(repo)?;
    repo.git(&["fetch", "origin"])?;
    repo.commit("b.txt", "ours\n", "feat: work of our own")?;
    Ok(())
}

/// A remote that keeps what it holds: every push to it is turned away by a
/// `pre-receive` hook, in the words a forge writes over a protected branch.
///
/// **A hook is the only way to have that refusal offline.** A protected
/// branch, a repository rule and a hook all reach this end as the same
/// `[remote rejected]`, and the sentence underneath is whatever the far
/// side chose to say — which is exactly what the report shows, so the
/// hook's message is written the way GitHub writes its own.
pub(super) fn protected(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;
    repo.git(&["switch", "--create", "feature/topic-a"])?;
    repo.commit("src/app.txt", "app v2\n", "feat: carry on with the app")?;
    repo.git(&["push", "origin", "feature/topic-a"])?;
    repo.git(&["switch", "main"])?;

    let hook = repo.root.join("origin.git").join("hooks");
    std::fs::create_dir_all(&hook).map_err(|e| e.to_string())?;
    let path = hook.join("pre-receive");
    std::fs::write(
        &path,
        "#!/bin/sh\n\
         echo \"error: GH006: Protected branch update failed for refs/heads/main.\" >&2\n\
         echo \"error: Cannot delete a protected branch\" >&2\n\
         exit 1\n",
    )
    .map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
