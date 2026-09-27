pragma Singleton

import QtQuick
import platitude
import platitude.ui

/// What a row of the left panel opens under itself: every section that opens, as one table
/// (デザイン規約 §左メニューの所作). What each section says is here, not in the shared list, row and lines
/// (`NavList` / `NavItemDelegate` / `NavRowFacts` / `NavFactLine`), so none of them tests which section it is in.
///
/// The relations are one read from different ends — a branch names its copy and its reading, a remote-tracking ref
/// the branch reading it and that branch's copy, a working copy its branch — so the lines are the same parts in a
/// different order.
QtObject {
    id: navFacts

    /// The fill and rim of the band a line that goes somewhere wears under the hand (`NavRowFacts.aimBand`), and of
    /// the same line on a stacked card (`RefChip`). The row's own wash once more over the row's: lit the same as the
    /// row, nothing would change under the hand. A rim with no alpha draws none.
    readonly property color aimFill: Theme.bgHover
    readonly property color aimRim: "transparent"

    /// The answers one row opens with, read as it opens (slots freeze in a binding — rules/app-ui.md). Every field is
    /// filled for every section; the ones a section does not answer stay empty, which decides the lines. Asked of the
    /// row, which carries the models (`sectionModel` / `worktreesModel` / `branchesModel`).
    function answers(row) {
        const leaf = !row.folder
        const kind = row.kindHint
        // The branch measured against this reading, or — on a working copy's row — the branch that copy holds. Both
        // name a BRANCHES row.
        const local = kind === "remote" && leaf && row.sectionModel !== null
                    ? row.sectionModel.trackedBy(row.fullName) : ""
        const branch = kind === "worktree" && leaf ? row.bucket : ""
        // What that branch reads, and whether git can reach it. A branch row reads the slot its badge is drawn from
        // (`Role::Bucket`), so the mark and the line cannot disagree; a row naming another's branch asks its section.
        const asked = kind === "worktree" && branch !== "" && row.branchesModel !== null ? branch : ""
        const upstream = kind === "branch" && row.sectionModel !== null
                       ? row.sectionModel.upstreamOf(row.fullName)
                       : asked !== "" ? row.branchesModel.upstreamOf(asked) : ""
        const gone = kind === "branch" ? row.bucket
                   : asked !== "" ? row.branchesModel.upstreamGoneOf(asked) : ""
        // The remotes carrying this tag, each `{remote, apart}` — apart from where the remote this window acts on has
        // it (`NavSectionModel.tagRemotes`). One call for both halves, so they cannot disagree.
        const tagged = kind === "tag" && leaf && row.sectionModel !== null
        const remotes = tagged ? row.sectionModel.tagRemotes(row.fullName, row.pushRemote) : []
        // The copy here, weighed by the same test as those remotes: its own commit against the right reading
        // (`NavSectionModel.tagApartAt`). A name only a remote has has no copy here to weigh.
        const hereApart = tagged && !row.only_remote
                        && row.sectionModel.tagApartAt(row.fullName, row.oid_hex, row.pushRemote)
        // Who that reading belongs to, as the note of a holder apart from it names them; empty where the remotes
        // disagree and nobody decides (`NavSectionModel.tagWeighedAgainst`).
        const by = tagged ? row.sectionModel.tagWeighedAgainst(row.fullName, row.pushRemote) : ""
        // The copy holding the branch this row is about: its own on a BRANCHES row, the one named above on a
        // REMOTES row. A working copy's row asks nobody — it is the copy.
        const holds = kind === "branch" ? row.fullName : kind === "remote" ? local : ""
        const held = row.folder || holds === "" || row.worktreesModel === null ? ""
                   : row.worktreesModel.worktreeHolding(holds)
        // The commit each named line stands on — where a press on it takes the graph. A gone reading, or a name
        // whose section this row was not handed, goes nowhere.
        const reading = gone !== "" || upstream === "" ? ""
                      : kind === "branch" && row.sectionModel !== null ? row.sectionModel.upstreamOidOf(row.fullName)
                      : asked !== "" ? row.branchesModel.upstreamOidOf(asked) : ""
        const named = local !== "" ? local : branch
        const tip = named === "" || row.branchesModel === null ? "" : row.branchesModel.oidOfName(named)
        return {
            // A working copy is named by its folder and lives at a path; every other row is named by its full name.
            "name": kind === "worktree" && leaf ? row.name : row.fullName,
            "path": kind === "worktree" && leaf ? row.fullName : "",
            "remotes": remotes,
            // The remote this window's tag rows act on, and who the right reading belongs to. TAGS only.
            "against": kind === "tag" ? row.pushRemote : "",
            "by": by,
            // Why a holder apart from that reading wears the warning — one sentence for every holder of this name.
            "note": tagged ? navFacts.apartNote(by) : "",
            // The copy here stands apart from it: the row's own name wears the carriers' warning, and a rest on it
            // says why (`nameNote`).
            "hereApart": hereApart,
            "nameNote": hereApart ? navFacts.apartNote(by) : "",
            "local": local,
            "branch": branch,
            // Shown as the folder the path ends in. Asked only when there is one, so this table runs where `GitFacts`
            // is absent (`tests/qml/tst_navrowlight.qml`).
            "heldBy": held === "" ? "" : GitFacts.pathLeaf(held),
            "upstream": gone !== "" ? gone : upstream,
            "gone": gone !== "",
            // The commit the row itself stands on — a line going there is the row's own (`joined`).
            "at": row.oid_hex,
            // Where a press on the line naming each of them goes (`place`).
            "branchTo": navFacts.place("branch", named, tip),
            "heldTo": held === "" ? null : navFacts.place("worktree", held, row.worktreesModel.headOfCopy(held)),
            "upstreamTo": navFacts.place("remote", upstream, reading),
            // The branch's measure: its own row carries it as a role (a fetch moves it); a row naming another's asks.
            "ahead": asked !== "" ? row.branchesModel.aheadOf(asked) : row.ahead,
            "behind": asked !== "" ? row.branchesModel.behindOf(asked) : row.behind,
            // The state git noted on a checkout, and the words that came with it (`Role::Change` / `Role::OrigPath`).
            "state": kind === "worktree" && leaf ? row.change : "",
            "why": kind === "worktree" && leaf ? row.orig_path : ""
        }
    }

    /// Those answers as the lines they draw, in reading order — the table. A line nobody filled is left out.
    function lines(kind, a) {
        const drawn = kind === "branch" ? [navFacts.copyLine(a), navFacts.readingLine(a)]
                    : kind === "remote" ? [navFacts.branchLine(a.local, a, a.branchTo), navFacts.copyLine(a)]
                    // The repository's own copy is already named by its branch (`NavRowBody.homeCopy`), so the line
                    // naming it again is left out; its measure goes to the row's own line (`NavRowBody`'s track seat).
                    : kind === "worktree" ? [a.state === "MAIN" ? null : navFacts.branchLine(a.branch, a, a.branchTo),
                                             navFacts.readingLine(a), navFacts.stateLine(a)]
                    // One line per remote carrying the tag — the one section whose lines are a list.
                    : kind === "tag" ? a.remotes.map(carried => navFacts.carrierLine(carried, a.note))
                    : []
        return navFacts.joined(drawn.filter(line => line !== null), a.at)
    }
    /// Two ways to one commit are one target (デザイン規約 §左メニューの所作「行き先が同じなら判定は 1 つ」): a line
    /// going where the row stands drops its `to` (its press is the row's click), and one going where the line above
    /// goes shares that band (`withAbove`). Only neighbours are compared — the table's order already puts every such
    /// pair side by side.
    function joined(lines, at) {
        for (let i = 0; i < lines.length; i++) {
            if (lines[i].to && lines[i].to.oid === at)
                lines[i].to = null
        }
        for (let j = 0; j < lines.length; j++) {
            const to = lines[j].to
            const above = j > 0 ? lines[j - 1].to : null
            lines[j].withAbove = !!to && !!above && above.oid === to.oid
        }
        return lines
    }

    /// Where a line goes when pressed: the commit, and the row of this panel that is that name (its key,
    /// `NavList.keyOf`). Null where there is no commit — the line is then the row's own to click.
    function place(section, full, oid) {
        return full === "" || oid === "" ? null : { "key": section + ":" + full, "oid": oid }
    }

    /// A BRANCHES branch named from elsewhere: that section's mark and colour, and the measure its own row draws.
    function branchLine(name, a, to) {
        return name === "" ? null
             : { "mark": "branch", "markTint": Theme.accent, "text": name, "tone": Theme.textSecondary,
                 "ahead": a.ahead, "behind": a.behind, "note": "", "to": to }
    }
    /// The working copy holding the branch this row is about, in the WORKTREES section's mark and colour
    /// (規約 §ref の種別).
    function copyLine(a) {
        return a.heldBy === "" ? null
             : { "mark": "tree", "markTint": Theme.success, "text": a.heldBy,
                 "tone": Theme.textSecondary, "ahead": 0, "behind": 0, "note": "", "to": a.heldTo }
    }
    /// What that branch is measured against. One git cannot reach says so in git's word (`gone`), name first, and
    /// goes nowhere.
    function readingLine(a) {
        if (a.upstream === "")
            return null
        const tint = a.gone ? Theme.warning : Theme.textSecondary
        return { "mark": "remote", "markTint": tint,
                 "text": a.gone ? qsTr("%1 is gone").arg(a.upstream) : a.upstream,
                 "tone": tint, "ahead": 0, "behind": 0, "note": "", "to": a.gone ? null : a.upstreamTo }
    }
    /// One remote carrying this tag, by name alone. Whether it stands on the right reading is said by colour
    /// (デザイン規約 §左メニューの所作 の TAGS の段), and why by the note a rest opens — a clause on one line would make
    /// the column ragged. It goes nowhere: a remote is not a row of this panel. But it names that remote on its own, so
    /// a menu asked for on it acts on that remote (`aim`, `NavRowFacts.lineMenu`).
    function carrierLine(carried, note) {
        const tint = carried.apart ? Theme.warning : Theme.textSecondary
        return { "mark": "remote", "markTint": tint, "text": carried.remote,
                 "tone": tint, "ahead": 0, "behind": 0,
                 "note": carried.apart ? note : "", "to": null, "aim": carried.remote }
    }
    /// Why a holder of a tag wears the warning, given who the right reading belongs to (`by`, as a sentence lists
    /// them): its reading stands elsewhere — or, with nobody to name, the remotes disagree and every holder does. One
    /// sentence for every holder — a remote's line, the copy here (the row's own name), a reading in a chip's card —
    /// and the same one the menus' delete rows say (`Words.differsFrom`).
    function apartNote(by) {
        return by !== "" ? Words.differsFrom(by) : Words.remotesDisagree
    }
    /// The same said inside a chip's card, as the line under a tag's name (`RowHoverHost.mateOf`): nothing can hover
    /// over a card that hover opened (デザイン規約 §グラフ行のダブルクリック), so the reason stands on the card itself,
    /// in the shape a gone upstream's line has there. It goes nowhere: it names no ref.
    function apartLine(note) {
        return { "mark": "remote", "markTint": Theme.warning, "text": note,
                 "tone": Theme.warning, "ahead": 0, "behind": 0, "note": "", "to": null }
    }
    /// The one state a mark cannot name: a folder git can no longer find (`!` names nothing). A lock's padlock says it
    /// all — its reason is the locker's words, not this panel's (デザイン規約 §左メニューの所作).
    function stateLine(a) {
        return a.state !== "PRUNABLE" ? null
             : { "mark": "", "markTint": Theme.warning, "text": qsTr("Folder is gone"),
                 "tone": Theme.warning, "ahead": 0, "behind": 0, "note": "", "to": null }
    }
}
