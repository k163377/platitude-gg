//! Presets about where a branch stands against a remote.

use super::repo::{DemoRepo, file_url};

/// A branch that has never been sent anywhere, in a repository with two
/// remotes — so the first push's question has something to pick between.
///
/// A push meets each of the three names over there differently: `taken`
/// left the trunk with a commit of its own (refused), `carried` is behind
/// us on our own line (lands and moves somebody else's branch on), and
/// `outsider` was never fetched here (no answer from this end). The
/// checked-out branch has no upstream.
///
/// "Never fetched here" holds only because `verify::seed` turns the
/// opening's fetch off for this preset.
pub(super) fn unpublished(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;

    let fork = repo.root.join("fork.git");
    std::fs::create_dir_all(&fork).map_err(|e| e.to_string())?;
    repo.git_at(&fork.clone(), &["init", "--bare", "-b", "main"])?;
    let fork_url = file_url(&fork);
    repo.git(&["remote", "add", "fork", &fork_url])?;

    // `taken`: pushed without `-u`, so nothing here records that it went
    // anywhere.
    repo.git(&["switch", "--create", "taken"])?;
    repo.commit("src/app.txt", "app v2\n", "feat: theirs")?;
    repo.git(&["push", "origin", "taken"])?;

    // `outsider`, pushed from another clone.
    let seeder = repo.seeder("origin.git")?;
    repo.git_at(&seeder.clone(), &["switch", "--create", "outsider"])?;
    std::fs::write(seeder.join("elsewhere.txt"), "not here\n").map_err(|e| e.to_string())?;
    repo.git_at(&seeder.clone(), &["add", "--", "elsewhere.txt"])?;
    repo.git_at(&seeder.clone(), &["commit", "-m", "feat: somewhere else"])?;
    repo.git_at(&seeder.clone(), &["push", "origin", "outsider"])?;

    repo.git(&["switch", "main"])?;
    repo.git(&["switch", "--create", "feature/new-thing"])?;
    repo.commit("src/new.txt", "new\n", "feat: draft the new thing")?;
    // `carried`: sent from here under a second name, then left behind.
    repo.git(&["push", "origin", "HEAD:refs/heads/carried"])?;
    repo.commit("src/new.txt", "new v2\n", "feat: finish the new thing")?;
    Ok(())
}

/// A fork workflow: the branch fetches from `origin`, and its own
/// `branch.main.pushRemote` sends every push to `fork`.
///
/// git weighs the branch's mark before the repository's (git-config(5)),
/// so a destination worked out from the repository's alone names `origin`
/// while the push goes to the fork (`push-target`, デザイン規約
/// §リモートへ送る). The commit of our own makes the button live.
pub(super) fn forkmark(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;

    let fork = repo.root.join("fork.git");
    std::fs::create_dir_all(&fork).map_err(|e| e.to_string())?;
    repo.git_at(&fork.clone(), &["init", "--bare", "-b", "main"])?;
    let fork_url = file_url(&fork);
    repo.git(&["remote", "add", "fork", &fork_url])?;
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

    let seeder = repo.seeder("origin.git")?;
    std::fs::write(seeder.join("a.txt"), "v2\nremote work\n").map_err(|e| e.to_string())?;
    repo.git_at(&seeder.clone(), &["add", "--", "a.txt"])?;
    repo.git_at(
        &seeder.clone(),
        &["commit", "-m", "feat: pushed while you slept"],
    )?;
    repo.git_at(&seeder.clone(), &["push"])?;
    Ok(())
}

/// [`behind`] with work of our own on top and nothing fetched since: the
/// remote moved on and this end does not know, so the toolbar offers a
/// plain `push`.
///
/// A window over this must open with the fetch off (`verify::seed`), or
/// what is on screen is [`diverged`].
pub(super) fn outrun(repo: &mut DemoRepo) -> Result<(), String> {
    behind(repo)?;
    repo.commit("b.txt", "ours\n", "feat: work of our own")?;
    Ok(())
}

/// A branch that still names an upstream the far side no longer holds:
/// git answers `[gone]` for it (デザイン規約 §左メニューの所作).
///
/// The prune is what makes it gone: without it the remote-tracking ref
/// stays and the branch reads as tracking. The name carries no slash,
/// which would fold the row under a folder.
pub(super) fn gone(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;
    repo.git(&["switch", "--create", "release-1.2"])?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.git(&["push", "--set-upstream", "origin", "release-1.2"])?;
    repo.git(&["push", "origin", "--delete", "release-1.2"])?;
    repo.git(&["fetch", "--prune", "origin"])?;
    repo.git(&["switch", "main"])?;
    Ok(())
}

/// Puts one hook into `hooks` and makes it runnable (the exec bit matters
/// off Windows). `script` is the whole file, shebang included.
fn install_hook(hooks: &std::path::Path, name: &str, script: &str) -> Result<(), String> {
    std::fs::create_dir_all(hooks).map_err(|e| e.to_string())?;
    let path = hooks.join(name);
    std::fs::write(&path, script).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// A repository whose own `pre-commit` hook says no, with an unstaged
/// change for it to say it about.
///
/// Like a wrapped linter, the hook writes its complaint to stdout and its
/// noise to stderr — which says whether both streams reach the report
/// (`commit::refused`).
pub(super) fn hooked(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.write("src/app.txt", "app v2\t\n")?;

    install_hook(
        &repo.work.join(".git").join("hooks"),
        "pre-commit",
        "#!/bin/sh\n\
         echo \"src/app.txt:1: trailing whitespace\"\n\
         echo \"style: 1 problem found, nothing committed\" >&2\n\
         exit 1\n",
    )
}

/// A repository whose own `pre-commit` hook takes its time, with
/// something already staged — so a run's one write is the commit itself,
/// still in flight when the quit verbs close the window across it.
///
/// `sleep` is the blocking command all three OSes agree on
/// (`verify::repos`); two seconds holds the write across a sampler beat
/// and the close behind it. A hook that ends early fails the run out loud.
pub(super) fn slowhook(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.write("held.txt", "held by the hook\n")?;
    repo.git(&["add", "--all"])?;
    install_hook(
        &repo.work.join(".git").join("hooks"),
        "pre-commit",
        "#!/bin/sh\nsleep 2\n",
    )
}

/// Both sides moved on and the fetch is part of the preset, so the toolbar
/// offers the overwrite (`push -f`) from the moment the window opens.
/// Breaking the remote's URL afterwards photographs the refused overwrite.
pub(super) fn diverged(repo: &mut DemoRepo) -> Result<(), String> {
    behind(repo)?;
    repo.git(&["fetch", "origin"])?;
    repo.commit("b.txt", "ours\n", "feat: work of our own")?;
    Ok(())
}

/// A remote nothing can reach: `origin` is pushed to and tracked, then its
/// URL names a directory that was never made — the only shape a fetch
/// fails in without a network (`fetch-fail`, `fetch-resume`; the runs
/// carry `--allow-write-failure`).
///
/// The URL is absolute so a template copy rebinds it to its own root with
/// the rest of git's metadata (`template::rebind`).
pub(super) fn unreachable(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;
    let gone = file_url(&repo.root.join("gone.git"));
    repo.git(&["config", "remote.origin.url", &gone])?;
    Ok(())
}

/// A remote that keeps what it holds: every push to it is turned away by a
/// `pre-receive` hook, in GitHub's words for a protected ref. A forge's
/// protection and a hook reach this end as the same `[remote rejected]`.
pub(super) fn protected(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;
    repo.git(&["switch", "--create", "feature/topic-a"])?;
    repo.commit("src/app.txt", "app v2\n", "feat: carry on with the app")?;
    repo.git(&["push", "origin", "feature/topic-a"])?;
    repo.git(&["switch", "main"])?;
    // A tag on both sides, pushed before the hook goes in: the delete row
    // is offered only where the remote holds the name (`tag-refused`).
    repo.git(&["tag", "v1.0"])?;
    repo.git(&["push", "origin", "refs/tags/v1.0"])?;

    // The words follow the ref, as GitHub's do (`Protected tag` /
    // `Protected branch`): the report quotes them as they came. The refs
    // arrive on stdin as `<old> <new> <ref>`.
    install_hook(
        &repo.root.join("origin.git").join("hooks"),
        "pre-receive",
        "#!/bin/sh\n\
         while read -r old new ref; do\n\
         case \"$ref\" in\n\
         refs/tags/*) kind=tag ;;\n\
         *) kind=branch ;;\n\
         esac\n\
         echo \"error: GH006: Protected $kind update failed for $ref.\" >&2\n\
         echo \"error: Cannot delete a protected $kind\" >&2\n\
         done\n\
         exit 1\n",
    )
}
