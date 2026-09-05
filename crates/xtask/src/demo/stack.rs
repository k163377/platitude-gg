//! The presets for the chip stack: one commit per shape the sheets behind
//! the front card can take, and one repository for the deepest row there
//! is.

use super::repo::DemoRepo;

/// Every colour a chip column can put on a row, every way two of them can
/// meet, and every depth the fan is drawn at — one commit each, so the
/// whole rule reads down one graph (デザイン規約 §ref の種別).
///
/// Newest first, which is how the graph draws them:
///
/// | commit | what the row carries                                        | cards |
/// |--------|-------------------------------------------------------------|-------|
/// | `extend the guide` (HEAD) | `main` = `origin/main`, `side`, `origin/mirror`, `v9.9` `v1.5` `v4.5` | 4 |
/// | `finish the app`   | `deep`, `origin/deep`, `v4.0`, origin's `v4.5`     | 4 |
/// | `widen the app`    | `wide`, `wide-2`, `wide-parked` (held), `origin/wide`, `v3.0` | 5 |
/// | `fix a typo`       | `origin/staging`, `v2.0`                          | 2 |
/// | `tidy the app`     | `origin/legacy`                                   | 1 |
/// | `rework the app`   | `topic`, `topic-2`                                | 2 |
/// | `read settings`    | `next`, origin's `v1.5`                           | 2 |
/// | `add a guide`      | `hotfix`, `spike` (held)                          | 2 |
/// | `harden the app`   | `qa`, `origin/preview`                            | 2 |
/// | `add the library`  | `release/1.0`, `v1.1`                             | 2 |
/// | `add the app`      | `v1.0`                                            | 1 |
/// | `start the readme` | `v0.1` `v0.2` `v0.3`                              | 2 |
///
/// **The head row is the everyday one**: the branch
/// this checkout is on, a second local beside it, a remote of its own, and
/// a tag. Its second local is one of the two places the gap between two
/// sheets of a single colour is read; the other is the readme's three
/// tags, where the card in front is the tag.
///
/// **`v1.5` and `v4.5` are drifts, not tags only the remote has.** A tag
/// whose commit this repository holds comes down with the next fetch
/// (auto-following takes the tags of objects it already has), and a tag on
/// a commit that is not here has no row to stand on — so a reading that is
/// only on the remote is the far half of a tag that moved: pushed where it
/// was made, then forced up to the head here, which leaves origin naming
/// the old commit and no fetch willing to overwrite the name (デザイン規約
/// §ref の種別「タグの枠だけは明度でも語る」). Both halves show once the
/// window has fetched, which it does as it opens.
///
/// The detached HEAD marker is not here — it would take the checkout off
/// `main` and every row would lose the current branch. That colour is
/// [`stack_max`]'s.
pub(super) fn stack(repo: &mut DemoRepo) -> Result<(), String> {
    // One tag colour, worn three times: the card in front is a tag, so the
    // sheet behind it is the one that repeats a colour.
    repo.commit("README.md", "# stack\n", "docs: start the readme")?;
    for tag in ["v0.1", "v0.2", "v0.3"] {
        repo.git(&["tag", tag])?;
    }

    // The plainest row there is: one name, no fan.
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.git(&["tag", "v1.0"])?;

    repo.commit("src/lib.txt", "lib v1\n", "feat: add the library")?;
    repo.git(&["branch", "release/1.0"])?;
    repo.git(&["tag", "v1.1"])?;

    repo.commit("src/app.txt", "app v2\n", "fix: harden the app")?;
    repo.git(&["branch", "qa"])?;

    repo.commit("docs/guide.md", "guide v1\n", "docs: add a guide")?;
    repo.git(&["branch", "hotfix"])?;
    // Checked out over in another working copy, which is what dulls its
    // sheet: git refuses a move onto a branch someone else holds.
    repo.git(&["worktree", "add", "-b", "spike", "../spike"])?;

    repo.commit("src/settings.txt", "on\n", "feat: read settings")?;
    repo.git(&["branch", "next"])?;
    repo.git(&["tag", "v1.5"])?;

    // Two locals and nothing else: the same colour twice with neither of
    // them the branch this checkout is on.
    repo.commit("src/app.txt", "app v3\n", "feat: rework the app")?;
    repo.git(&["branch", "topic"])?;
    repo.git(&["branch", "topic-2"])?;

    repo.commit("src/app.txt", "app v4\n", "fix: tidy the app")?;

    repo.commit("README.md", "# stack demo\n", "docs: fix a typo")?;
    repo.git(&["tag", "v2.0"])?;

    // The deepest row a checkout with a current branch elsewhere can
    // carry: two locals, one of them held, a remote of its own and a tag.
    repo.commit("src/wide.txt", "wide\n", "feat: widen the app")?;
    repo.git(&["branch", "wide"])?;
    repo.git(&["branch", "wide-2"])?;
    repo.git(&["tag", "v3.0"])?;
    // **Named to sort after the two beside it**: the chips come out in the
    // order core sorted the refs, so a held branch called `parked` would
    // take the front card and the row would read as a dulled one rather
    // than as a branch with a held one behind it.
    repo.git(&["worktree", "add", "-b", "wide-parked", "../parked"])?;

    repo.commit("src/deep.txt", "deep\n", "feat: finish the app")?;
    repo.git(&["branch", "deep"])?;
    repo.git(&["tag", "v4.0"])?;
    repo.git(&["tag", "v4.5"])?;

    repo.commit("docs/guide.md", "guide v2\n", "docs: extend the guide")?;
    repo.git(&["branch", "side"])?;
    repo.git(&["tag", "v9.9"])?;

    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;
    // Remote branches under names no local branch carries, so each keeps a
    // row of its own rather than folding into one
    // (`RemoteBranches::folded_into_local`).
    repo.git(&["push", "origin", "qa:refs/heads/preview"])?;
    repo.git(&["push", "origin", "HEAD~4:refs/heads/legacy"])?;
    repo.git(&["push", "origin", "HEAD~3:refs/heads/staging"])?;
    repo.git(&["push", "origin", "wide:refs/heads/wide"])?;
    repo.git(&["push", "origin", "deep:refs/heads/deep"])?;
    repo.git(&["push", "origin", "main:refs/heads/mirror"])?;
    repo.git(&[
        "push", "origin", "v0.1", "v1.0", "v1.1", "v1.5", "v2.0", "v3.0", "v4.0", "v4.5", "v9.9",
    ])?;
    // Moved after they were published, which no fetch undoes: origin goes
    // on naming the commits they were made on.
    repo.git(&["tag", "--force", "v1.5"])?;
    repo.git(&["tag", "--force", "v4.5"])?;
    repo.git(&["fetch", "origin"])?;
    Ok(())
}

/// The deepest row there is: every colour at once, on the commit a
/// detached HEAD is standing on.
///
/// Six cards — the marker, a local branch, one another working copy holds,
/// a remote with no counterpart here, a tag, and the far half of a tag
/// that moved. **The marker cannot repeat**, so this is also the ceiling:
/// no commit can carry more than one card per colour once the front card's
/// own is the only one allowed twice (`RefChipStack.sheets`).
///
/// Detached on purpose, and in its own repository for that reason: with
/// HEAD off `main` there is no current branch anywhere in the graph, which
/// is the one thing [`stack`] is for.
pub(super) fn stack_max(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# every colour\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.commit("src/app.txt", "app v2\n", "feat: rework the app")?;
    repo.git(&["tag", "v3.0"])?;
    // Published from here and then moved down, so the reading left on the
    // remote is one this tip carries and this repository does not.
    repo.git(&["tag", "v2.5"])?;
    repo.git(&["worktree", "add", "-b", "parked", "../parked"])?;

    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;
    repo.git(&["push", "origin", "main:refs/heads/side"])?;
    repo.git(&["push", "origin", "v2.5", "v3.0"])?;
    // Moved off the tip, so origin goes on naming it and no fetch will
    // overwrite the name here — the far half of the drift is this row's
    // dim tag.
    repo.git(&["tag", "--force", "v2.5", "HEAD~1"])?;
    repo.git(&["fetch", "origin"])?;
    // Last, so everything above still had a branch to be written from.
    repo.git(&["switch", "--detach"])?;
    Ok(())
}
