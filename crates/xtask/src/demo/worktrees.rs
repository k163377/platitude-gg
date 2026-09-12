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
/// `branch --delete` for a branch another worktree holds (measured), so the
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

/// A working copy standing where no ref reaches: detached, with a commit
/// made in it. **That commit is in the graph only because the walk is
/// told to name it** (`session::walk::walk_command`) — `git log` reads
/// the HEAD of the tree it runs in and no other, and there is no branch,
/// no tag and no remote pointing here.
///
/// Small on purpose: the subject is one row and the chip on it, and a
/// preset with branches in it would leave the reader hunting for which
/// row is the one with no ref.
pub(super) fn worktree_detached(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nOne copy is off on its own.\n",
        "docs: start the readme",
    )?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.commit("src/app.txt", "app v2\n", "feat: grow the app")?;

    // Detached at the tip, and then a commit of its own: from here the
    // only thing in the repository that names that commit is the
    // worktree listing.
    repo.git(&["worktree", "add", "--detach", "../spike"])?;
    let spike = repo.root.join("spike");
    std::fs::write(spike.join("idea.txt"), "an idea nobody named yet\n")
        .map_err(|e| format!("writing the spike's file: {e}"))?;
    repo.git_at(&spike, &["add", "idea.txt"])?;
    repo.git_at(
        &spike,
        &["commit", "-m", "spike: try the idea in a detached checkout"],
    )?;
    Ok(())
}
