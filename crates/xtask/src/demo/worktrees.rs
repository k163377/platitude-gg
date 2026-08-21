//! The preset for the WORKTREES section: every annotation `git worktree
//! list` can put on an entry, in one repository.

use super::repo::DemoRepo;

/// One worktree in each state git can report: an everyday linked one, a
/// detached one, a lock with a reason and a lock without, and an entry
/// whose folder is gone (`prunable`). The checkout this window opens is
/// the main one, so it is the row that wears the current mark.
///
/// The branches matter as much as the folders: `feature/topic-a` is
/// checked out over in `topic`, and git refuses both `switch` and
/// `branch --delete` for a branch another worktree holds (実測), so the
/// BRANCHES row for it is the one the menu has to answer for.
pub(super) fn worktrees(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nA repository with several working copies.\n",
        "docs: start the readme",
    )?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.git(&["tag", "v0.1"])?;
    repo.commit("src/lib.txt", "lib v1\n", "feat: add the library")?;

    // A branch with history of its own, so the row leads somewhere.
    repo.git(&["switch", "--create", "feature/topic-a"])?;
    repo.commit("src/topic.txt", "topic draft\n", "feat: draft the topic")?;
    repo.git(&["switch", "main"])?;
    repo.commit("docs/guide.md", "guide v1\n", "docs: add a guide")?;

    // Ordinary: a second checkout of a branch this one is not on.
    repo.git(&["worktree", "add", "../topic", "feature/topic-a"])?;
    // Detached: no branch to name on the right of the row.
    repo.git(&["worktree", "add", "--detach", "../detached", "v0.1"])?;
    // Locked, with the words git was given for why.
    repo.git(&["worktree", "add", "-b", "hotfix/urgent", "../hotfix"])?;
    repo.git(&[
        "worktree",
        "lock",
        "--reason",
        "release run is using this checkout",
        "../hotfix",
    ])?;
    // Locked with no reason at all — the flag on its own.
    repo.git(&["worktree", "add", "-b", "spike/idea", "../spike"])?;
    repo.git(&["worktree", "lock", "../spike"])?;
    // Listed, but the folder it names is gone: `git worktree prune`
    // would drop it, and opening it can only fail.
    repo.git(&["worktree", "add", "-b", "gone/branch", "../gone"])?;
    let gone = repo.root.join("gone");
    std::fs::remove_dir_all(&gone).map_err(|e| format!("removing {}: {e}", gone.display()))?;
    // A folder and a branch that both run past the pane, on a row that
    // also carries a mark: the seat, the elided name and the branch on
    // the right have to share the narrowest sidebar there is (180) with
    // none of them walking over another. **Named to sort last** so the
    // rows the verbs address by number do not move.
    repo.git(&[
        "worktree",
        "add",
        "-b",
        "release/2026-08-candidate-with-a-very-long-name",
        "../very-long-folder-name-for-a-release-candidate",
    ])?;
    repo.git(&[
        "worktree",
        "lock",
        "--reason",
        "held by the release run on the build machine, which is a reason long enough to run past any pane",
        "../very-long-folder-name-for-a-release-candidate",
    ])?;

    Ok(())
}
