//! The presets for the chip stack: the shapes the sheets behind the front
//! card can take, three rows apiece, and one repository for the deepest
//! row there is.

use super::repo::DemoRepo;

/// Every colour a chip column can put on a row, every way two of them can
/// meet, and every depth the fan is drawn at — **three rows to a shape**,
/// so the fan is read against its own repeat rather than against a
/// neighbour of some other depth: what a stack costs the row above and
/// below it is the whole question the picture answers, and a shape shown
/// once cannot answer it.
///
/// Newest first, which is how the graph draws them. Each line is three
/// commits carrying the same names under the same subject:
///
/// | rows | what each row carries | cards |
/// |------|-----------------------|-------|
/// | `main` `next` `side` | the branch, its own remote (folded, so the badge is on the name), a second local, a remote of its own and a tag | 4 |
/// | `wide` `deep` `tall` | two locals, one of them held by another working copy, a remote and a tag | 5 |
/// | `spot` `mark` `note` | a local, a remote and a tag | 3 |
/// | `stem` `leaf` `root` | a local and one another working copy holds | 2 |
/// | `origin/theirs-a` … | a remote branch, and a tag only origin has | 2 |
/// | `v1.1` `v2.1` `v3.1` | tags and nothing else — the colour repeated | 2 |
/// | `only` `lone` `solo` | one local branch | 1 |
/// | `v0.1` `v0.2` `v0.3` | one tag | 1 |
///
/// **The current branch stands on the newest row only** — a checkout has
/// one, so the two rows under it wear the same shape with a plain local
/// in front of it.
///
/// **`v9.1`, `v9.2` and `v9.3` are drifts, not tags only the remote has.**
/// A tag whose commit this repository holds comes down with the next
/// fetch (auto-following takes the tags of objects it already has), and a
/// tag on a commit that is not here has no row to stand on — so a reading
/// that is only on the remote is the far half of a tag that moved: pushed
/// where it was made, then forced down onto a row that already carries
/// tags, which leaves origin naming the old commit and no fetch willing
/// to overwrite the name (デザイン規約 §ref の種別「タグの枠だけは明度でも
/// 語る」). Both halves show once the window has fetched, which it does as
/// it opens.
///
/// The detached HEAD marker is not here — it would take the checkout off
/// `main` and every row would lose the current branch. That colour is
/// [`stack_max`]'s.
pub(super) fn stack(repo: &mut DemoRepo) -> Result<(), String> {
    stack_rows(repo)?;
    stack_published(repo)
}

/// The commits and the names this repository holds of its own, built from
/// the bottom of the graph up: `main` is carried along by them, so the
/// last shape written is the one the current branch stands on.
fn stack_rows(repo: &mut DemoRepo) -> Result<(), String> {
    for tag in ["v0.1", "v0.2", "v0.3"] {
        let body = format!("app {tag}\n");
        repo.commit("src/app.txt", &body, "feat: one tag and nothing else")?;
        repo.git(&["tag", tag])?;
    }

    for name in ["solo", "lone", "only"] {
        let body = format!("lib {name}\n");
        repo.commit(
            "src/lib.txt",
            &body,
            "feat: one local branch and nothing else",
        )?;
        repo.git(&["branch", name])?;
    }

    // The one colour a row may wear twice is the front card's own, and
    // these are the rows where the front card is the tag.
    for tags in [["v3.1", "v3.2"], ["v2.1", "v2.2"], ["v1.1", "v1.2"]] {
        let body = format!("notes {}\n", tags[0]);
        repo.commit("docs/notes.md", &body, "docs: two tags on one commit")?;
        for tag in tags {
            repo.git(&["tag", tag])?;
        }
    }

    // Nothing of this repository's own stands on these rows: the branch
    // is only on the remote, and so is the reading of the tag. The tag
    // made here is pushed and then moved away, which is what leaves the
    // remote naming this commit (see above); it also names the commit
    // for the push that publishes the branch.
    for drift in ["v9.3", "v9.2", "v9.1"] {
        let body = format!("theirs {drift}\n");
        repo.commit(
            "src/theirs.txt",
            &body,
            "feat: a remote branch and a tag only origin has",
        )?;
        repo.git(&["tag", drift])?;
    }

    for name in ["root", "leaf", "stem"] {
        let body = format!("held {name}\n");
        repo.commit(
            "src/held.txt",
            &body,
            "feat: a local branch and one another copy holds",
        )?;
        repo.git(&["branch", name])?;
        // Checked out over in another working copy, which is what dulls
        // it: git refuses a move onto a branch someone else holds. Added
        // where the commit is, so the branch starts on this row.
        let held = format!("{name}-held");
        let at = format!("../{held}");
        repo.git(&["worktree", "add", "-b", &held, &at])?;
    }

    for (name, tag) in [("note", "v4.3"), ("mark", "v4.2"), ("spot", "v4.1")] {
        let body = format!("three {name}\n");
        repo.commit("src/three.txt", &body, "feat: a local, a remote and a tag")?;
        repo.git(&["branch", name])?;
        repo.git(&["tag", tag])?;
    }

    // The deepest row a checkout whose current branch is elsewhere can
    // carry: two locals, one of them held, a remote of its own and a tag.
    for (name, tag) in [("tall", "v5.3"), ("deep", "v5.2"), ("wide", "v5.1")] {
        let body = format!("wide {name}\n");
        repo.commit(
            "src/wide.txt",
            &body,
            "feat: two locals, one held, a remote and a tag",
        )?;
        repo.git(&["branch", name])?;
        let second = format!("{name}-2");
        repo.git(&["branch", &second])?;
        repo.git(&["tag", tag])?;
        // **Named to sort after the two beside it**: the chips come out
        // in the order core sorted the refs, so a held branch whose name
        // sorts first would take the front card and the row would read as
        // a dulled one rather than as a branch with a held one behind it.
        let held = format!("{name}-held");
        let at = format!("../{held}");
        repo.git(&["worktree", "add", "-b", &held, &at])?;
    }

    // The everyday row, three deep: the branch this checkout is on, a
    // second local beside it, a remote of its own and a tag. `main` is
    // written last because it is the branch the commits are landing on.
    for (name, tag) in [("side", "v6.3"), ("next", "v6.2"), ("main", "v6.1")] {
        let body = format!("guide {name}\n");
        repo.commit(
            "docs/guide.md",
            &body,
            "docs: a branch, a second local, a remote and a tag",
        )?;
        if name != "main" {
            repo.git(&["branch", name])?;
        }
        let second = format!("{name}-2");
        repo.git(&["branch", &second])?;
        repo.git(&["tag", tag])?;
    }
    Ok(())
}

/// What origin holds of those rows, and the far halves the moved tags
/// leave on it. Last of all, so every push has a whole row to name.
fn stack_published(repo: &mut DemoRepo) -> Result<(), String> {
    repo.add_origin()?;
    // Each of the three newest rows carries its own remote under its own
    // name, which folds into the local and puts the badge on the name
    // (`RemoteBranches::folded_into_local`).
    for name in ["main", "next", "side"] {
        repo.git(&["push", "--set-upstream", "origin", name])?;
    }
    // Remote branches under names no local branch carries, so each keeps
    // a card of its own instead of folding into one.
    for name in [
        "main", "next", "side", "wide", "deep", "tall", "spot", "mark", "note",
    ] {
        let far = format!("{name}:refs/heads/{name}-far");
        repo.git(&["push", "origin", &far])?;
    }
    // The rows with nothing of their own: the tag standing on each of
    // them names the commit the branch is published from.
    for (drift, far) in [
        ("v9.1", "theirs-a"),
        ("v9.2", "theirs-b"),
        ("v9.3", "theirs-c"),
    ] {
        let spec = format!("{drift}:refs/heads/{far}");
        repo.git(&["push", "origin", &spec])?;
    }
    repo.git(&["push", "origin", "--tags"])?;
    // Moved after they were published, which no fetch undoes: origin goes
    // on naming the commits they were made on. They land on rows that
    // already carry a tag, so the drift costs no card where it arrives.
    for (drift, onto) in [("v9.1", "v1.1"), ("v9.2", "v2.1"), ("v9.3", "v3.1")] {
        repo.git(&["tag", "--force", drift, onto])?;
    }
    repo.git(&["fetch", "origin"])?;
    Ok(())
}

/// The deepest row there is, three rows of it: every colour at once, and
/// the detached HEAD standing on the newest of them.
///
/// Six cards on the top row — the marker, a local branch, one another
/// working copy holds, a remote with no counterpart here, a tag, and the
/// far half of a tag that moved. **The marker cannot repeat**, so this is
/// also the ceiling: no commit can carry more than one card per colour
/// once the front card's own is the only one allowed twice
/// (`RefChipStack.sheets`). The two rows under it carry the same five
/// without it, which is what a shape shown three times means where one of
/// the three is HEAD's.
///
/// Detached on purpose, and in its own repository for that reason: with
/// HEAD off `main` there is no current branch anywhere in the graph,
/// which is the one thing [`stack`] is for.
pub(super) fn stack_max(repo: &mut DemoRepo) -> Result<(), String> {
    // `main` is the newest row's own local — it is the branch the commits
    // land on, and the two rows under it are named as they are made.
    for (name, tag, drift) in [
        ("base", "v1.0", "v9.3"),
        ("mid", "v2.0", "v9.2"),
        ("main", "v3.0", "v9.1"),
    ] {
        let body = format!("app {name}\n");
        repo.commit("src/app.txt", &body, "feat: every colour on one row")?;
        if name != "main" {
            repo.git(&["branch", name])?;
        }
        repo.git(&["tag", tag])?;
        repo.git(&["tag", drift])?;
        let held = format!("{name}-held");
        let at = format!("../{held}");
        repo.git(&["worktree", "add", "-b", &held, &at])?;
    }

    repo.add_origin()?;
    repo.git(&["push", "origin", "main"])?;
    // A remote branch per row under a name no local carries, so it stands
    // as a card of its own.
    for name in ["main", "mid", "base"] {
        let far = format!("{name}:refs/heads/{name}-far");
        repo.git(&["push", "origin", &far])?;
    }
    repo.git(&["push", "origin", "--tags"])?;
    // Published where they were made and then moved one row down, so
    // origin goes on naming the commit each was pushed from and no fetch
    // will overwrite the name here — the far half of the drift is every
    // row's dim tag, and the near half lands where a tag already stands.
    for (drift, onto) in [("v9.1", "v2.0"), ("v9.2", "v1.0"), ("v9.3", "v3.0")] {
        repo.git(&["tag", "--force", drift, onto])?;
    }
    repo.git(&["fetch", "origin"])?;
    // Last, so everything above still had a branch to be written from.
    repo.git(&["switch", "--detach"])?;
    Ok(())
}
