pragma Singleton

import QtQuick
import platitude
import platitude.ui

/// What a row of the left panel opens under itself — **the three sections that open, as one table**
/// (デザイン規約 §左メニューの所作).
///
/// The parts were already shared: one list (`NavList`), one row (`NavItemDelegate`), one set of lines
/// (`NavRowFacts` / `NavFactLine`). What each section *says* was not — it sat in the row as a run of tests on which
/// section the row was in, and in the lines as a run of tests on which answer was filled. Both are here now, so a
/// section's answer is one entry and the rest of the panel cannot tell the three apart.
///
/// **The relations are the same one read from different ends.** A branch names the copy holding it and the reading
/// it is measured against; a remote-tracking ref names the branch that reads it and the copy holding *that*; a
/// working copy names the branch it holds and says of it what that branch's own row would. So the lines are the
/// same parts in a different order, and a section that gains one gains a line in the table below.
QtObject {
    id: navFacts

    /// The answers one row opens with, read as it opens (the slots below freeze in a binding — app-ui.md). Every
    /// field is filled for every section; the ones that section does not answer stay empty, which is what decides
    /// the lines.
    ///
    /// **Asked of the row**, because the row is what carries the slots and the models: its own section
    /// (`sectionModel`), the working copies (`worktreesModel`) and the branches (`branchesModel`).
    function answers(row) {
        const leaf = !row.folder
        const kind = row.kindHint
        // The branch measured against this reading, and — on a working copy's row — the branch that copy holds.
        // Both name a row of BRANCHES, which is why they lead the lines the same way.
        const local = kind === "remote" && leaf && row.sectionModel !== null
                    ? row.sectionModel.trackedBy(row.fullName) : ""
        const branch = kind === "worktree" && leaf ? row.bucket : ""
        // What that branch reads, and whether git can reach it. **A branch row carries its own state in a slot the
        // row draws its badge from**, so the mark and the line cannot disagree (`models::nav::field` の
        // `Role::Bucket`); a row naming somebody else's branch has to ask the section holding it.
        const asked = kind === "worktree" && branch !== "" && row.branchesModel !== null ? branch : ""
        const upstream = kind === "branch" && row.sectionModel !== null
                       ? row.sectionModel.upstreamOf(row.fullName)
                       : asked !== "" ? row.branchesModel.upstreamOf(asked) : ""
        const gone = kind === "branch" ? row.bucket
                   : asked !== "" ? row.branchesModel.upstreamGoneOf(asked) : ""
        // The copy holding the branch this row is about: its own on a BRANCHES row, the one named above on a
        // REMOTES row. A working copy's row asks nobody — it is the copy.
        const holds = kind === "branch" ? row.fullName : kind === "remote" ? local : ""
        const held = row.folder || holds === "" || row.worktreesModel === null ? ""
                   : row.worktreesModel.worktreeHolding(holds)
        return {
            // **A working copy is named by its folder and lives at a path** — two answers, and the row goes on
            // showing the first. Every other row here is named by the whole of what git knows it by.
            "name": kind === "worktree" && leaf ? row.name : row.fullName,
            "path": kind === "worktree" && leaf ? row.fullName : "",
            "local": local,
            "branch": branch,
            // The section answers with the path git prints, and the rows of that section show the folder it ends
            // in — asked only when there is one, so this table can be read where that singleton is not
            // (`tests/qml/tst_navrowlight.qml`).
            "heldBy": held === "" ? "" : GitFacts.pathLeaf(held),
            "upstream": gone !== "" ? gone : upstream,
            "gone": gone !== "",
            // The one measure a branch draws, wherever its name is drawn. A row that names its own branch carries
            // it as a role, because a fetch moves it; a row that names somebody else's has to ask.
            "ahead": asked !== "" ? row.branchesModel.aheadOf(asked) : row.ahead,
            "behind": asked !== "" ? row.branchesModel.behindOf(asked) : row.behind,
            // The state git noted on a checkout, and the words that came with it (`Role::Change` / `Role::OrigPath`).
            "state": kind === "worktree" && leaf ? row.change : "",
            "why": kind === "worktree" && leaf ? row.orig_path : ""
        }
    }

    /// Those answers as the lines they draw, in reading order — **the table**. A line nobody filled is left out, so
    /// a row with nothing to add opens on its name alone.
    function lines(kind, a) {
        const drawn = kind === "branch" ? [navFacts.copyLine(a), navFacts.readingLine(a)]
                    : kind === "remote" ? [navFacts.branchLine(a.local, a), navFacts.copyLine(a)]
                    : kind === "worktree" ? [navFacts.branchLine(a.branch, a), navFacts.readingLine(a),
                                             navFacts.stateLine(a)]
                    : []
        return drawn.filter(line => line !== null)
    }

    /// A branch of the BRANCHES section, named from somewhere else: that section's own mark in its own colour, and
    /// the measure that branch's own row draws at the same right edge.
    function branchLine(name, a) {
        return name === "" ? null
             : { "mark": "branch", "markTint": Theme.accent, "text": name, "tone": Theme.textSecondary,
                 "ahead": a.ahead, "behind": a.behind }
    }
    /// The working copy holding the branch this row is about — the WORKTREES section's own mark.
    function copyLine(a) {
        return a.heldBy === "" ? null
             : { "mark": "tree", "markTint": Theme.textSecondary, "text": a.heldBy,
                 "tone": Theme.textSecondary, "ahead": 0, "behind": 0 }
    }
    /// What that branch is measured against. **A reading git cannot reach wears the state whole** — git's own word
    /// for it is `gone`, and the name leads so every line begins in the same column.
    function readingLine(a) {
        if (a.upstream === "")
            return null
        const tint = a.gone ? Theme.warning : Theme.textSecondary
        return { "mark": "remote", "markTint": tint,
                 "text": a.gone ? qsTr("%1 is gone").arg(a.upstream) : a.upstream,
                 "tone": tint, "ahead": 0, "behind": 0 }
    }
    /// And the one state a mark cannot name. **The padlock says everything a lock has to say here** — what it was
    /// taken for is the words somebody gave `git worktree lock`, and those are theirs to keep rather than a line of
    /// this panel's (デザイン規約 §左メニューの所作). A folder git can no longer find is the other case: `!` is a
    /// warning and names nothing, so the words say which warning it is.
    function stateLine(a) {
        return a.state !== "PRUNABLE" ? null
             : { "mark": "", "markTint": Theme.warning, "text": qsTr("Folder is gone"),
                 "tone": Theme.warning, "ahead": 0, "behind": 0 }
    }
}
