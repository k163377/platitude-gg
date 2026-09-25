//! Presets parked mid-operation: merges, rebases, cherry-picks and drops,
//! each stopped on the state its card has to describe.

use super::repo::DemoRepo;

const BASE: &str = "\
Release checklist
=================

- pick the version number
- write the notes
- tag the commit
- upload the archives
";

/// `main` and `feature/clash`, each having changed the same line of
/// `shared.txt`, with the merge not yet made.
///
/// Long enough that the combined diff shows context, both sides and the
/// markers in one hunk; one side alone also changes another line, so a
/// cleanly merged part is on screen beside the fenced one.
fn two_sides_of_one_line(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("shared.txt", BASE, "feat: shared base")?;
    repo.commit("other.txt", "calm\n", "feat: untouched elsewhere")?;

    repo.git(&["switch", "--create", "feature/clash"])?;
    repo.commit(
        "shared.txt",
        &BASE
            .replace("- pick the version number", "- agree the version number")
            .replace(
                "- upload the archives",
                "- upload the archives and checksums",
            ),
        "feat: change shared",
    )?;

    repo.git(&["switch", "main"])?;
    repo.commit(
        "shared.txt",
        &BASE.replace("- pick the version number", "- decide the version number"),
        "fix: change shared too",
    )?;
    Ok(())
}

/// The two sides with the merge still to come, so pressing `merge` in the
/// window is what stops it (`merge-stops`).
pub(super) fn clashing(repo: &mut DemoRepo) -> Result<(), String> {
    two_sides_of_one_line(repo)
}

/// A commit whose undoing collides with the one after it, the revert not
/// yet made — pressing `revert` in the window is what stops it
/// (`revert-stops`). The row to revert is `row:1` (a clean tree puts the
/// newest commit at row 0).
pub(super) fn revert_clashes(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("shared.txt", BASE, "feat: shared base")?;
    repo.commit("other.txt", "calm\n", "feat: untouched elsewhere")?;
    repo.commit(
        "shared.txt",
        &BASE.replace("- pick the version number", "- agree the version number"),
        "docs: say how the version is chosen",
    )?;
    repo.commit(
        "shared.txt",
        &BASE.replace("- pick the version number", "- decide the version number"),
        "docs: say it another way",
    )?;
    Ok(())
}

/// A merge stopped on conflicts: MERGE_HEAD present, one unmerged path.
pub(super) fn conflict(repo: &mut DemoRepo) -> Result<(), String> {
    two_sides_of_one_line(repo)?;
    repo.git_expecting_stop(&["merge", "--no-edit", "feature/clash"])?;
    Ok(())
}

/// The same stopped merge with the conflicted file typed over: markers
/// gone, one line where the sides disagreed, the path still unmerged in
/// the index.
///
/// The combined diff flips here — both sides' lines turn from additions
/// into removals against their own parent — and no other preset stands
/// here, since a staged resolution leaves the conflict bucket.
pub(super) fn conflict_typed(repo: &mut DemoRepo) -> Result<(), String> {
    conflict(repo)?;
    repo.write(
        "shared.txt",
        &BASE
            .replace("- pick the version number", "- settle the version number")
            .replace(
                "- upload the archives",
                "- upload the archives and checksums",
            ),
    )?;
    Ok(())
}

/// The same merge with that resolution staged: `MERGE_HEAD` still stands
/// and the way out is the commit button (`merge-commit`). The counterpart
/// of [`conflict_ours`] with something left to list.
pub(super) fn conflict_staged(repo: &mut DemoRepo) -> Result<(), String> {
    conflict_typed(repo)?;
    repo.git(&["add", "--", "shared.txt"])?;
    Ok(())
}

/// The same merge resolved by keeping ours, and staged: `git status`
/// answers empty while `MERGE_HEAD` stands, and the commit still writes
/// two parents. The only preset where the uncommitted row is drawn over a
/// clean tree.
pub(super) fn conflict_ours(repo: &mut DemoRepo) -> Result<(), String> {
    conflict(repo)?;
    repo.write(
        "shared.txt",
        &BASE.replace("- pick the version number", "- decide the version number"),
    )?;
    repo.git(&["add", "--", "shared.txt"])?;
    Ok(())
}

/// The two branches of a rebase that will stop, not yet run — pressing
/// `rebase` onto `main` from `feature/clash` is what stops it
/// (`rebase-stops`).
pub(super) fn rebase_clashes(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("shared.txt", "base\n", "feat: shared base")?;
    repo.commit("other.txt", "calm\n", "feat: untouched elsewhere")?;
    repo.git(&["switch", "--create", "feature/clash"])?;
    repo.commit("shared.txt", "feature side\n", "feat: change shared")?;
    // A second commit behind the conflicting one, so the count has
    // somewhere to go and `--skip` has a next step to move to.
    repo.commit("later.txt", "after\n", "feat: one more on the branch")?;
    repo.git(&["switch", "main"])?;
    repo.commit("shared.txt", "main side\n", "fix: change shared too")?;
    repo.git(&["switch", "feature/clash"])?;
    Ok(())
}

/// A rebase stopped part-way: it steps (counts `1/2`, takes `--skip` /
/// `--quit`) and the sides swap over — the commit being replayed is
/// "theirs".
pub(super) fn rebase_conflict(repo: &mut DemoRepo) -> Result<(), String> {
    rebase_clashes(repo)?;
    repo.git_expecting_stop(&["rebase", "main"])?;
    Ok(())
}

/// A drop whose replay goes through and whose restore collides: the
/// uncommitted edit sits on the line the newest commit rewrote. The drop
/// is of HEAD, so the verb needs no argument; the landing is a move's
/// (デザイン規約 §未コミット変更がある状態での移動).
pub(super) fn drop_collides(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit(
        "docs/release.md",
        "# releasing\n\n- pick the version number\n- tag it\n- upload the archives\n",
        "docs: write the release steps down",
    )?;
    repo.commit(
        "docs/release.md",
        "# releasing\n\n- agree the version number\n- tag it\n- upload the archives\n",
        "docs: reword the first step",
    )?;
    repo.write(
        "docs/release.md",
        "# releasing\n\n- read the version number off the milestone\n- tag it\n\
         - upload the archives\n",
    )?;
    Ok(())
}

/// A drop the commits after it depend on, over a dirty tree: once the work
/// is stashed out of the way the replay walks into the hole the drop
/// leaves and stops part-way.
///
/// The commit to take out is `row:2` (the WIP row sits above the newest
/// commit). The uncommitted edit is in another file, so it takes no part
/// in the collision.
pub(super) fn drop_stops(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit(
        "docs/release.md",
        "# releasing\n\n- tag it\n",
        "docs: tag it",
    )?;
    repo.commit(
        "docs/release.md",
        "# releasing\n\n- tag it\n- upload the archives\n",
        "docs: upload the archives",
    )?;
    repo.commit(
        "docs/release.md",
        "# releasing\n\n- tag it\n- upload the archives\n- announce it\n",
        "docs: announce it",
    )?;
    repo.write("README.md", "# demo\n\nnotes, still being written\n")?;
    Ok(())
}

/// The same rebase, one step further on: the conflict resolved and
/// staged, so `--continue` is live and the file list has left CONFLICTS.
pub(super) fn rebase_staged(repo: &mut DemoRepo) -> Result<(), String> {
    rebase_conflict(repo)?;
    repo.write("shared.txt", "main side\nfeature side\n")?;
    repo.git(&["add", "shared.txt"])?;
    Ok(())
}

/// A rebase stopped on a commit that came out empty — git's own
/// `Otherwise, please use 'git rebase --skip'`, with nothing conflicted or
/// staged.
///
/// Reached with `--empty=ask` rather than `-i`, which would need a
/// sequence editor on three OSes. `ask`, not its 2.45 rename `stop`: the
/// minimum git refuses a value it does not know (`git_expecting_stop`).
pub(super) fn rebase_empty(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("f.txt", "a\n", "feat: root")?;
    repo.git(&["switch", "--create", "topic"])?;
    repo.commit("f.txt", "a\nX\n", "feat: adds X")?;
    repo.commit("h.txt", "keep\n", "feat: one more on the branch")?;
    repo.git(&["switch", "main"])?;
    // The same net line, arriving with company: not a clean cherry-pick
    // of "adds X", so the cherry-pick filter cannot be what drops it.
    repo.write("f.txt", "a\nX\n")?;
    repo.write("g.txt", "unrelated\n")?;
    repo.git(&["add", "-A"])?;
    repo.git(&["commit", "-m", "fix: X arrives with company"])?;
    repo.git(&["switch", "topic"])?;
    repo.git_expecting_stop(&["rebase", "--empty=ask", "main"])?;
    Ok(())
}

/// A cherry-pick stopped on a conflict: it steps the way a rebase does
/// (takes `--skip` and `--quit`) but keeps no count.
pub(super) fn cherry_pick_conflict(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("shared.txt", "base\n", "feat: shared base")?;
    repo.git(&["switch", "--create", "feature/clash"])?;
    repo.commit("shared.txt", "feature side\n", "feat: change shared")?;
    repo.git(&["switch", "main"])?;
    repo.commit("shared.txt", "main side\n", "fix: change shared too")?;
    repo.git_expecting_stop(&["cherry-pick", "feature/clash"])?;
    Ok(())
}

/// The same stopped cherry-pick, then let go of with `--quit`: the
/// operation is gone (no badge, no exit card) while the unmerged paths
/// stay, so a switch is refused in git's other wording (`you need to
/// resolve your current index first`) with nothing on screen saying why.
pub(super) fn cherry_pick_quit(repo: &mut DemoRepo) -> Result<(), String> {
    cherry_pick_conflict(repo)?;
    repo.git(&["cherry-pick", "--quit"])?;
    Ok(())
}

/// A merge stopped on four different kinds of conflict at once, so the
/// rows that name what each side did can be read side by side: both
/// changed it (`UU`), both added it (`AA`), deleted here and changed
/// there (`DU`), changed here and deleted there (`UD`).
pub(super) fn conflict_kinds(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("both.txt", "base\n", "feat: shared base")?;
    repo.commit("ours-del.txt", "base\n", "feat: one we will drop")?;
    repo.commit("theirs-del.txt", "base\n", "feat: one they will drop")?;

    repo.git(&["switch", "--create", "feature/clash"])?;
    repo.write("both.txt", "feature side\n")?;
    repo.write("ours-del.txt", "feature keeps editing\n")?;
    repo.write("added.txt", "feature's new file\n")?;
    std::fs::remove_file(repo.work.join("theirs-del.txt")).map_err(|e| e.to_string())?;
    repo.git(&["add", "-A"])?;
    repo.git(&["commit", "-m", "feat: the feature side of all four"])?;

    repo.git(&["switch", "main"])?;
    repo.write("both.txt", "main side\n")?;
    repo.write("theirs-del.txt", "main keeps editing\n")?;
    repo.write("added.txt", "main's new file\n")?;
    std::fs::remove_file(repo.work.join("ours-del.txt")).map_err(|e| e.to_string())?;
    repo.git(&["add", "-A"])?;
    repo.git(&["commit", "-m", "fix: the main side of all four"])?;

    repo.git_expecting_stop(&["merge", "--no-edit", "feature/clash"])?;
    Ok(())
}
