//! The preset for the WORKTREES section: every annotation `git worktree
//! list` can put on an entry, in one repository.

use super::repo::{DemoRepo, file_url};

/// Other working copies with something uncommitted in them, so the rows
/// they draw on the graph have something to say. Its own preset because
/// `worktrees` keeps its copies clean: a carried row in the graph would
/// move every row number its verbs address.
///
/// A copy of each shape the row draws: standing where this window stands
/// (its row goes above the stash on the same commit), on a branch further
/// down the history, and clean (no row at all). The dirt differs so the
/// tallies do too.
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

    // Further down the history, holding one path staged and then written
    // again: the pane shows it as one row, since nothing here can move
    // that copy's index (`Kinds::folded`, `Bucket::Whole`).
    repo.git(&["worktree", "add", "../topic", "feature/topic-a"])?;
    let topic = repo.root.join("topic");
    std::fs::write(topic.join("src/topic.txt"), "topic redrafted\n")
        .map_err(|e| format!("writing topic.txt: {e}"))?;
    repo.git_at(&topic, &["add", "--", "src/topic.txt"])?;
    std::fs::write(topic.join("src/topic.txt"), "topic redrafted, and again\n")
        .map_err(|e| format!("writing topic.txt again: {e}"))?;

    // Clean: a copy with nothing to say draws no row at all.
    repo.git(&["worktree", "add", "-b", "side/quiet", "../quiet"])?;

    // A name past the pane's 300px floor in any font, so the band, the
    // block and the diff's own header are read cut on this one
    // (`carried-read` on this copy).
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

/// How many more untracked files `carried-many` leaves in `../here`.
const CARRIED_MANY: usize = 40;

/// `carried`, with `here` holding forty more untracked files, so the pane
/// reading it overflows (`middle-hand carried`). Its own preset because
/// `carried`'s verbs read its counts.
pub(super) fn carried_many(repo: &mut DemoRepo) -> Result<(), String> {
    carried(repo)?;
    let here = repo.root.join("here");
    for n in 0..CARRIED_MANY {
        let name = format!("scrap_{n:02}.txt");
        std::fs::write(here.join(&name), format!("scrap {n:02}\n"))
            .map_err(|e| format!("writing {name}: {e}"))?;
    }
    Ok(())
}

/// The same copies, over a clean tree and a branch whose one commit lands
/// on the line this one just moved (`wip-landing-stopped`). A replay is
/// refused over uncommitted work, so the tree starts clean and the stop's
/// conflicts dirty it.
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

    // A copy standing where the replay stops, holding work of its own: the
    // walk hands a carried row out at its copy's commit
    // (`session::rows::CarriedRows::take_for`) and the replay leaves this
    // window detached there, so this row leads the held-back pass. Without
    // it the pass leads with an ordinary commit, which reads correctly
    // anyway, and the run would pass without asking. Holding the branch
    // the replay names is fine: git rebases onto it without a checkout
    // here.
    repo.git(&["worktree", "add", "../clashing", "side/clash"])?;
    let clashing = repo.root.join("clashing");
    std::fs::write(clashing.join("scratch.txt"), "held over on the side\n")
        .map_err(|e| format!("writing the clashing copy's scratch: {e}"))?;
    Ok(())
}

/// One worktree in each state git can report: linked, detached, detached
/// and locked, locked with and without a reason, and one whose folder is
/// gone (`prunable`). This window opens the main checkout, so that row
/// wears the current mark.
///
/// The graph reads the same list — a green frame on a held branch, a
/// branchless copy's folder name in one, a padlock for a lock
/// (デザイン規約 §ref の種別) — so each is on a row here.
///
/// `feature/topic-a` is out in `topic`, and git refuses `switch` and
/// `branch --delete` for a branch another worktree holds, so its BRANCHES
/// row is the one the menu has to answer for.
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
    // A second, branchless copy where the first stands: two green records
    // on one row, the one shape that repeats the front card's colour
    // (デザイン規約 §重ね表示「先頭のカードとその 1 つ後ろが同色の時だけ」).
    // Named to sort after `topic`, so the rows the verbs address by number
    // stay put.
    repo.git(&[
        "worktree",
        "add",
        "--detach",
        "../topic-twin",
        "feature/topic-a",
    ])?;
    // Detached: no branch to name on the right of the row.
    repo.git(&["worktree", "add", "--detach", "../detached", "v0.1"])?;
    // Detached and locked: a name of its own with a padlock beside it, on
    // a commit nothing else names. Named to sort between `topic` and the
    // long one, so row numbers stay put.
    repo.git(&["worktree", "add", "--detach", "../vaulted", "main~1"])?;
    repo.git(&[
        "worktree",
        "lock",
        "--reason",
        "kept for the audit",
        "../vaulted",
    ])?;
    repo.git(&["worktree", "add", "-b", "hotfix/urgent", "../hotfix"])?;
    repo.git(&[
        "worktree",
        "lock",
        "--reason",
        "release run is using this checkout",
        "../hotfix",
    ])?;
    repo.git(&["worktree", "add", "-b", "spike/idea", "../spike"])?;
    repo.git(&["worktree", "lock", "../spike"])?;
    // Listed, but the folder it names is gone: `git worktree prune`
    // would drop it, and opening it can only fail.
    repo.git(&["worktree", "add", "-b", "gone/branch", "../gone"])?;
    let gone = repo.root.join("gone");
    std::fs::remove_dir_all(&gone).map_err(|e| format!("removing {}: {e}", gone.display()))?;
    // A folder and a branch both past the pane, on a row with a mark: the
    // seat, the elided name and the branch share the narrowest sidebar
    // (180) without overlapping. Named to sort last, so row numbers stay
    // put.
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
/// one shape in which a REMOTES row opens on all three of its lines — the
/// reading's own name, the branch measured against it, and the copy
/// holding that branch (デザイン規約 §左メニューの所作). No other preset
/// has both a remote and a second copy. The commit made in the copy gives
/// the line a count; a branch level with its reading draws none.
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
/// made in it, which is in the graph only because the walk names it
/// (`session::walk::walk_command`) — `git log` reads only its own tree's
/// HEAD. Small, so the one row with no ref is easy to find.
pub(super) fn worktree_detached(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nOne copy is off on its own.\n",
        "docs: start the readme",
    )?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.commit("src/app.txt", "app v2\n", "feat: grow the app")?;

    // From here only the worktree listing names that commit.
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

/// Where new working copies would go, with something already in the way
/// of two of them — what the rows that make a copy warn of before the
/// press (`worktrees::new_copy_path`: `repo.worktrees/<branch>`).
/// `feature/free` has its folder to itself; `feature/blocked`'s holds a
/// file somebody left there; and git still lists a copy at
/// `fix/listed`'s, whose folder was taken away by hand. One more branch
/// has a free folder too long for the menu row, which the hover gives back.
pub(super) fn copy_places(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nA repository with places for new working copies.\n",
        "docs: start the readme",
    )?;
    repo.git(&["branch", "feature/a-name-long-enough-to-cut-its-folder"])?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.git(&["branch", "feature/blocked"])?;
    repo.git(&["branch", "fix/listed"])?;
    repo.commit("src/lib.txt", "lib v1\n", "feat: add the library")?;
    repo.git(&["branch", "feature/free"])?;
    repo.commit("docs/guide.md", "guide v1\n", "docs: add a guide")?;

    let places = repo.root.join("repo.worktrees");
    let blocked = places.join("feature-blocked");
    std::fs::create_dir_all(&blocked).map_err(|e| format!("making {}: {e}", blocked.display()))?;
    std::fs::write(blocked.join("notes.txt"), "left here by hand\n")
        .map_err(|e| format!("writing the blocked folder's file: {e}"))?;
    repo.git(&[
        "worktree",
        "add",
        "--detach",
        "../repo.worktrees/fix-listed",
    ])?;
    let listed = places.join("fix-listed");
    std::fs::remove_dir_all(&listed).map_err(|e| format!("removing {}: {e}", listed.display()))?;
    Ok(())
}

/// Where [`nested_copy`] puts its linked working copy, under the preset's
/// root. Shared with `verify::repos`, which opens the run at it rather
/// than at the path `demo::create` answers with.
pub const NESTED_COPY: &str = "copies/nested";

/// One linked working copy a level below the root, the way gathered
/// copies are kept (`.claude/worktrees/<letter>` in this tree). The
/// nesting is the point: for a sibling copy the folder above it and the
/// folder above the repository are the same, so a picker run could not
/// say which it was pointed at (`open-picker copy`).
pub(super) fn nested_copy(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nA repository whose copy is kept below it.\n",
        "docs: start the readme",
    )?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.commit("src/lib.txt", "lib v1\n", "feat: add the library")?;
    repo.git(&[
        "worktree",
        "add",
        "-b",
        "side/nested",
        &format!("../{NESTED_COPY}"),
    ])?;
    Ok(())
}

/// The branch [`long_names`] stands on. Long enough, on its own line,
/// that the panel's actions give their words up at a width over the
/// window's floor on both platforms' fonts (`band-actions`,
/// `band-actions-fold`).
const LONG_BRANCH: &str = "release/2026-08-candidate-with-a-very-long-branch-name-for-the-panel";

/// Every name the operation panel writes, long enough to be cut, so the
/// order it gives them up in and the shapes its actions take can be
/// photographed at widths a window can be dragged to — short names never
/// run the panel short. The history is the shortest that still gives the
/// branch an upstream, a remote that has moved on, and a copy. The
/// branch's own line is what the panel pays for (the upstream goes under
/// it, `OpsPicker`), hence [`LONG_BRANCH`].
pub(super) fn long_names(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nEvery name in the panel, long.\n",
        "docs: start the readme",
    )?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.add_origin()?;
    repo.git(&["switch", "--create", LONG_BRANCH])?;
    repo.commit(
        "src/release.txt",
        "release v1\n",
        "feat: prepare the release",
    )?;
    repo.git(&["push", "--set-upstream", "origin", LONG_BRANCH])?;
    // The remote moves on from a clone, unfetched here: with the commit
    // below, one each way, and a push git will not send.
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
    repo.git_at(&seeder.clone(), &["switch", LONG_BRANCH])?;
    std::fs::write(seeder.join("src/release.txt"), "release v1\nremote work\n")
        .map_err(|e| e.to_string())?;
    repo.git_at(&seeder.clone(), &["add", "--", "src/release.txt"])?;
    repo.git_at(
        &seeder.clone(),
        &["commit", "-m", "feat: pushed while you slept"],
    )?;
    repo.git_at(&seeder.clone(), &["push"])?;
    // One commit the remote has not got, so the counts stand beside the
    // name and give way with it.
    repo.commit(
        "src/release.txt",
        "release v2\n",
        "feat: finish the release",
    )?;
    // …and a copy with a name of its own, which the panel writes after
    // the repository's as one run.
    repo.git(&[
        "worktree",
        "add",
        "-b",
        "side/long-lived-integration-branch",
        "../a-very-long-working-copy-folder-name",
    ])?;
    Ok(())
}

/// The operation panel's two cards with something in every column — the
/// shape a repository being worked in gives them, where [`worktrees`]
/// gives the copies' states one at a time.
///
/// Every kind of row the branch card has: a folder of two (`topic/`), a
/// lone branch (`rig`), one parted from its remote (`feature/tracked`,
/// one each way — the counts and the cloud), and two out in other copies
/// (`worktree-a`, `worktree-b` — the tree mark). The current branch is two
/// ahead of its reading, whose name is longer, so the counts are set
/// against the end of the panel's longer second line. `worktree-b`'s copy
/// is locked, so the copies' card has a row with a mark in its seat beside
/// one without.
pub(super) fn panel(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nA repository being worked in.\n",
        "docs: start the readme",
    )?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;
    repo.git(&["branch", "topic/competent-benz"])?;
    repo.git(&["branch", "topic/wizardly-ellis"])?;
    repo.git(&["branch", "rig"])?;
    // Pushed, then taken back a step and moved on from there: one commit
    // each side, with nothing fetched to arrange it.
    repo.git(&["switch", "--create", "feature/tracked"])?;
    repo.commit("src/tracked.txt", "tracked v1\n", "feat: track the topic")?;
    repo.git(&["push", "--set-upstream", "origin", "feature/tracked"])?;
    repo.git(&["reset", "--hard", "HEAD~1"])?;
    repo.commit(
        "src/tracked.txt",
        "tracked, the other way\n",
        "feat: track the topic another way",
    )?;
    repo.git(&["switch", "main"])?;
    repo.git(&["worktree", "add", "-b", "worktree-a", "../a"])?;
    repo.git(&["worktree", "add", "-b", "worktree-b", "../b"])?;
    repo.git(&["worktree", "lock", "../b"])?;
    repo.commit("src/app.txt", "app v2\n", "feat: grow the app")?;
    repo.commit("docs/guide.md", "guide v1\n", "docs: add a guide")?;
    Ok(())
}
