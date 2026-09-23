//! The preset for the WORKTREES section: every annotation `git worktree
//! list` can put on an entry, in one repository.

use super::repo::{DemoRepo, file_url};

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

/// How many more untracked files `carried-many` leaves in `../here`.
const CARRIED_MANY: usize = 40;

/// `carried`, with the copy standing where this window stands holding
/// forty more untracked files: the one shape in which the pane reading
/// another copy has more rows than it has room for (`middle-hand
/// carried`). Its own preset, because the counts `carried` is read by
/// are the point of its verbs.
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

/// One worktree in each state git can report: an everyday linked one, a
/// detached one, a detached one that is also locked, a lock with a reason
/// and a lock without, and an entry whose folder is gone (`prunable`). The
/// checkout this window opens is the main one, so it is the row that wears
/// the current mark.
///
/// **The graph reads the same list**: a branch another copy holds wears
/// the green frame, a copy with no branch wears its own folder name in
/// one, and a lock on either puts the padlock beside the name
/// (デザイン規約 §ref の種別), so every one of those is on a row here.
/// **And two copies stand on one commit**, which is the row that draws a
/// green sheet behind a green card — the one colour a fan repeats
/// (§重ね表示).
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
    // **A second copy standing where the first one is**, on no branch of
    // its own: the row then carries two green records — the branch
    // `topic` holds and this marker — which is the one shape that draws
    // the fan's repeated sheet in the front card's own colour
    // (デザイン規約 §重ね表示「先頭のカードとその 1 つ後ろが同色の時だけ」).
    // **Named to sort after `topic`**, so the rows the verbs address by
    // number stay where they are.
    repo.git(&[
        "worktree",
        "add",
        "--detach",
        "../topic-twin",
        "feature/topic-a",
    ])?;
    // Detached: no branch to name on the right of the row.
    repo.git(&["worktree", "add", "--detach", "../detached", "v0.1"])?;
    // Detached **and** locked: the one shape the graph draws with a name
    // of its own and a padlock beside it (デザイン規約 §ref の種別). A
    // commit of its own, so the marker stands on a row nothing else
    // names. **Named to sort between `topic` and the long one**, so the
    // rows the verbs address by number stay where they are.
    repo.git(&["worktree", "add", "--detach", "../vaulted", "main~1"])?;
    repo.git(&[
        "worktree",
        "lock",
        "--reason",
        "kept for the audit",
        "../vaulted",
    ])?;
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

/// Where [`nested_copy`] puts its linked working copy, under the root
/// that preset was built in.
///
/// Named here because two places need it and neither owns it: the
/// preset that makes the copy, and the run that opens at it
/// (`verify::repos`) — every other fixture hands a run the repository's
/// own copy, which is the one path `demo::create` answers with.
pub const NESTED_COPY: &str = "copies/nested";

/// One linked working copy, kept in a folder of its own a level below
/// the root — the way copies are kept where they are gathered together
/// rather than scattered beside the repositories
/// (`.claude/worktrees/<letter>` in this tree).
///
/// **The nesting is the whole of the preset.** Every other copy in
/// these fixtures is the repository's sibling, and there the folder
/// above the copy and the folder above the repository are one and the
/// same — a picker pointed at either lands in the same place, so a run
/// on such a fixture cannot say which of the two it was pointed at
/// (`open-picker copy`).
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

/// Every name the operation panel writes, long enough to be cut — so the
/// order it gives them up in, and the shapes its actions take as they
/// give their words up, can be photographed at widths a hand can drag
/// the window to. **Short names never reach those shapes**: the panel
/// only runs short when the names are long, so a repository called
/// `repo` on a branch called `main` photographs the whole set as one
/// picture of a band with room to spare.
///
/// **The names are the subject**, so the history under them is the
/// shortest one that still gives a branch an upstream to be measured
/// against, a remote that has moved on without it, and a copy to be
/// standing in.
///
/// **The branch's own line is what the panel pays for** — the upstream
/// is written under it, not after it (`OpsPicker`) — so that line alone
/// has to run long enough for the actions to give their words up above
/// the window's floor ([`LONG_BRANCH`]).
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
    // The remote moves on under it, from a clone of its own, and this
    // end is never told: what the panel then says is a branch that has
    // one of each, and a push git will not send.
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

/// The operation panel's two cards with something in every column they
/// draw — the shape a repository being worked in gives them, where
/// [`worktrees`] gives the copies' states one at a time.
///
/// **Every kind of row the branch card has**: a folder of two
/// (`topic/`), a branch standing alone (`rig`), one measured against a
/// remote it has parted from (`feature/tracked`, one each way — the
/// counts and the cloud), and two another copy has out (`worktree-a`,
/// `worktree-b` — the tree mark). **The branch the window stands on is
/// ahead of what it reads by two**, with a name shorter than that
/// reading: the panel's second line is then the longer of the two, which
/// is where the counts are set against its end.
///
/// The copy the second branch is out in is locked, so the copies' card
/// holds a row with a mark in its seat beside one without.
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
