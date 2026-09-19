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
/// Other working copies with something uncommitted in them, so the rows
/// they draw on the graph have something to say.
///
/// **Its own preset.** Those copies
/// are clean on purpose — the verbs that use them are about the rows in
/// the sidebar, and a carried row appearing in the graph would move every
/// row number they address by.
///
/// Three copies, one of each shape the row has to draw: one standing
/// where this window stands (its row goes above the stash on the same
/// commit), one on a branch of its own further down the history, and one
/// clean (no row at all). The dirt differs so the tallies cannot all be
/// the same number: untracked only, and one staged edit.
pub(super) fn carried(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nA repository whose other copies are holding work.\n",
        "docs: start the readme",
    )?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.git(&["switch", "--create", "feature/topic-a"])?;
    repo.commit("src/topic.txt", "topic draft\n", "feat: draft the topic")?;
    repo.git(&["switch", "main"])?;
    repo.commit("docs/guide.md", "guide v1\n", "docs: add a guide")?;

    // A stash on HEAD: the rows of the copies standing here go above it.
    repo.write("scratch.txt", "experiment\n")?;
    repo.git(&["stash", "push", "--include-untracked", "-m", "experiment"])?;

    // Standing where this window stands, holding two untracked files —
    // two so the pane's arrows have somewhere to step (`carried-read`).
    repo.git(&["worktree", "add", "-b", "side/here", "../here"])?;
    let here = repo.root.join("here");
    std::fs::write(here.join("jotted.txt"), "another note over here\n")
        .map_err(|e| format!("writing jotted.txt: {e}"))?;
    std::fs::write(here.join("notes.txt"), "jotted over here\n")
        .map_err(|e| format!("writing notes.txt: {e}"))?;

    // Further down the history, holding **one path on both sides**: staged
    // and then written again. The pane reading it shows one row — the
    // split is that copy's index, which nothing here can move — so this is
    // the copy the fold is read on (`Kinds::folded`, `Bucket::Whole`).
    repo.git(&["worktree", "add", "../topic", "feature/topic-a"])?;
    let topic = repo.root.join("topic");
    std::fs::write(topic.join("src/topic.txt"), "topic redrafted\n")
        .map_err(|e| format!("writing topic.txt: {e}"))?;
    repo.git_at(&topic, &["add", "--", "src/topic.txt"])?;
    std::fs::write(topic.join("src/topic.txt"), "topic redrafted, and again\n")
        .map_err(|e| format!("writing topic.txt again: {e}"))?;

    // Clean: a copy with nothing to say draws no row at all.
    repo.git(&["worktree", "add", "-b", "side/quiet", "../quiet"])?;

    // A name with nowhere to go: the pane's floor is 300px and this is
    // past it whatever the font, so the band, the block and the diff's own
    // header are all read on this one (`carried-long`).
    repo.git(&[
        "worktree",
        "add",
        "-b",
        "side/an-extremely-long-branch-name-for-the-edge-case",
        "../an-extremely-long-working-copy-name-for-the-edge-case",
    ])?;
    let long = repo
        .root
        .join("an-extremely-long-working-copy-name-for-the-edge-case");
    std::fs::write(long.join("docs/guide.md"), "guide, rewritten over there\n")
        .map_err(|e| format!("writing the long copy's guide: {e}"))?;

    // This window's own tree, so its row stands too and the two can be
    // told apart by the name only one of them wears.
    repo.write("docs/guide.md", "guide v2\n")?;
    Ok(())
}

/// The same copies, over a tree with nothing uncommitted in it and a
/// branch whose one commit lands on the line this one just moved.
///
/// **Both halves are needed and they pull against each other.** A replay
/// is refused over uncommitted work, so the press that stops has to
/// start from a clean tree — and the row a stopped operation's landing
/// goes to only exists because the stop itself leaves conflicts behind.
/// So the tree is committed here and dirtied by git, in a graph where
/// every other copy already has a row of its own — and one of them
/// stands where the replay stops, so the pass the run holds this
/// window's row back from leads with somebody else's
/// (`PGG_AUTO_ACT=wip-landing-stopped`).
pub(super) fn carried_clashing(repo: &mut DemoRepo) -> Result<(), String> {
    carried(repo)?;
    repo.commit(
        "docs/guide.md",
        "guide v2\n",
        "docs: take the guide further",
    )?;
    repo.git(&["switch", "--create", "side/clash", "HEAD~1"])?;
    repo.commit(
        "docs/guide.md",
        "guide, the side's way\n",
        "docs: the side's guide",
    )?;
    repo.git(&["switch", "main"])?;

    // A copy standing where the replay stops, holding work of its own.
    // **Its row is what the held-back pass leads with**: a carried row is
    // handed out by the walk at the commit its copy stands on
    // (`session::rows::CarriedRows::take_for`), the replay leaves this
    // window detached at that same commit, and the rows of the copies
    // `carried` already put further down the history draw nowhere near
    // the top. Without it the pass this run arranges leads with an
    // ordinary commit, which the reading this verb is about answers
    // correctly anyway — and the run would pass without ever asking.
    //
    // **Holding the branch the replay names is fine**: git rebases onto
    // that commit without checking the branch out here, and the press
    // answers in a second and a half either way (measured both forms).
    repo.git(&["worktree", "add", "../clashing", "side/clash"])?;
    let clashing = repo.root.join("clashing");
    std::fs::write(clashing.join("scratch.txt"), "held over on the side\n")
        .map_err(|e| format!("writing the clashing copy's scratch: {e}"))?;
    Ok(())
}

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
    // the right have to share the narrowest sidebar there is (180),
    // each keeping clear of the others. **Named to sort last** so the
    // rows the verbs address by number stay put.
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

/// A branch that reads a remote and is checked out somewhere else: the
/// one shape in which a REMOTES row opens on all three of its lines —
/// the reading's own name, the branch measured against it with the
/// counts, and the working copy holding that branch
/// (デザイン規約 §左メニューの所作).
///
/// **All three have to be true at once**, and no other preset has them:
/// `worktrees` has copies and no remote, and the remote presets have
/// readings and no second copy. The commit made over in the copy is what
/// gives the line a count to draw — a branch level with its reading
/// draws none.
pub(super) fn tracked_elsewhere(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nA branch read from a remote and held by another copy.\n",
        "docs: start the readme",
    )?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;

    repo.git(&["switch", "--create", "feature/topic-a"])?;
    repo.commit("src/topic.txt", "topic draft\n", "feat: draft the topic")?;
    repo.git(&["push", "--set-upstream", "origin", "feature/topic-a"])?;
    repo.git(&["switch", "main"])?;

    // Out in a copy of its own, and one commit past what the remote
    // holds.
    repo.git(&["worktree", "add", "../topic", "feature/topic-a"])?;
    let topic = repo.root.join("topic");
    std::fs::write(topic.join("src/topic.txt"), "topic ready\n")
        .map_err(|e| format!("writing topic.txt: {e}"))?;
    repo.git_at(&topic, &["add", "--", "src/topic.txt"])?;
    repo.git_at(&topic, &["commit", "-m", "feat: finish the topic"])?;
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
