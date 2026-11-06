import QtQuick
import platitude.ui

// What the discard log's rows say: one operation that took something away, as the model reads it —
// `{key, kind, name, remote, worktree, worktreePath, at, lost, parts, byPart}`, each part `{restore, name, with, look,
// tip, remote, lost, files, notCopied, worktreePath}` (`DiscardModel.rows`). The rows draw `{key, mark, tint, badged,
// paired, title, titleMarks, listTitle, listMarks, at, worktree, worktreeMark, parts, byPart}`, each part `{mark, tint,
// badged, text, restore, restoreMarks}` (`RecoverPane`, `RecoverBand`, `RecoverHoverCard`, the marks through
// `RecoverMark`) — the words are said here, where they can be translated.
//
// The verbs are the ones other tools bring things back with: GitHub's `Restore branch`, the Windows Recycle Bin's
// `Restore all items` (`RecoverBand`). The times, what each part took and what its restore makes of it, and the
// marks' tints are user decisions too — ask before changing.
//
// No word says what kind a thing is — the mark in front of its name does (user decision — ask before changing;
// デザイン規約 §ref の種別「ref の名前の印」): `as %1, tracking %2`, `%1 deleted both locally and on %2`, `%1 removed`,
// `%1 force-pushed to %2`, never `as branch …`, which says it twice.
//
// **A title opens on the name and says what was done to it after** (user decision — ask before changing): the log is
// for tracing back what was just thrown away and restoring it, so the name outranks telling the operations apart. A
// row the narrow left panel cuts at its end loses the doing first, and rows may read alike there (`Detached …`): no
// word is shortened to tell them apart. What has no name opens on what it was (`2 files discarded`,
// `Detached HEAD left behind`). A left menu row opens on the kind's mark and names the thing itself — a stash by its
// message (`On main: experiment dropped`; the card's sentence keeps `Stash dropped`) — its names code too, with no
// mark of their own (user decision: bare, a name sinks into the sentence).
QtObject {
    id: shown

    /// The model's entries, newest first.
    property var rows: []
    /// How the worktree at a path is named, as everything else names it — the repository's own by the branch it has
    /// out (デザイン規約 §ref の種別「リポジトリ自身の worktree の呼び名」): `{name, home}`, or null for none to say.
    property var calledOf: path => null
    /// The branch the repository's own worktree has out — read by `entries`, so the words follow it.
    property string homeBranch: ""

    readonly property var entries: {
        const branch_ = shown.homeBranch
        return shown.rows.map(row => shown.entryOf(row))
    }

    /// A worktree's name and mark: as `calledOf` names the one at `path`, else the folder the record gives, behind the
    /// tree.
    function worktreeAt(path, folder) {
        const called = path ? shown.calledOf(path) : null
        return called && called.name !== "" ? { name: called.name, mark: called.home ? "home" : "tree" }
                                            : { name: folder, mark: "tree" }
    }

    /// A mark wears the tint the left menu gives its kind (`NavRail.sections`); uncommitted work wears the deleted
    /// file's red of the right panel's lists (`ChangeIcon`), being thrown away. A remote's tag drops a step, as a
    /// chip's does for a tag this repository does not hold (`RefChip.kindColourFor`).
    function tintOf(mark, theirs) {
        return mark === "branch" ? Theme.accent
             : mark === "tree" ? Theme.success
             : mark === "tag" ? (theirs ? Theme.refTagDim : Theme.refTag)
             : mark === "minus" ? Theme.diffRemovedFg
             : Theme.textSecondary
    }
    /// Where the operation took it (`RecoverMark`): `here`, `theirs` — a remote's alone — or `both` at once.
    function whereOf(row) {
        switch (row.kind) {
        case "deleted-remote-tag":
        case "force-pushed-tag":
        case "deleted-remote-branch":
        case "force-pushed":
            return "theirs"
        case "deleted":
        case "deleted-tag":
            return row.remote === "" ? "here" : "both"
        default:
            return "here"
        }
    }
    /// The row's mark: what kind of thing the operation took.
    function markOf(row) {
        switch (row.kind) {
        case "discarded":
        case "deleted-untracked":
            return "minus"
        case "dropped-stash":
        case "popped-stash":
            return "stash"
        case "deleted-tag":
        case "deleted-remote-tag":
        case "force-pushed-tag":
            return "tag"
        case "removed-worktree":
            return "tree"
        case "deleted-remote-branch":
        case "force-pushed":
            return "remote"
        default:
            return "branch"
        }
    }
    function commits(n) {
        return n === 1 ? qsTr("1 commit") : qsTr("%1 commits").arg(n)
    }
    /// How long ago `at` was, in the list's one column of times. Within the hour by minutes, within the day by hours,
    /// then by the calendar; a week back and further reads as the date, where a count stops telling days apart. The
    /// moment in full is the card's and the band's (`whenWords`). Minutes and hours are written short, the unit's
    /// letter against the number (`39m`, `11h`), so the narrow menu gives the row's title what it can.
    function ago(at, now) {
        const secs = Math.max(0, now - at)
        if (secs < 60)
            return qsTr("Just now")
        if (secs < 3600)
            return qsTr("%1m ago").arg(Math.floor(secs / 60))
        if (secs < 86400)
            return qsTr("%1h ago").arg(Math.floor(secs / 3600))
        const then = new Date(at * 1000)
        const today = new Date(now * 1000)
        const days = Math.round((new Date(today.getFullYear(), today.getMonth(), today.getDate())
                                 - new Date(then.getFullYear(), then.getMonth(), then.getDate())) / 86400000)
        if (days <= 1)
            return qsTr("Yesterday")
        if (days < 7)
            return qsTr("%1 days ago").arg(days)
        return Qt.formatDate(then, "yyyy-MM-dd")
    }
    /// The time in full: the moment first, so every such line starts in one column, then how long ago — left off once
    /// the list already writes the date, which the moment begins with.
    function whenWords(at, now) {
        const ago = shown.ago(at, now)
        const stamp = Words.stamp(at)
        return stamp.indexOf(ago) === 0 ? stamp : qsTr("%1 (%2)").arg(stamp).arg(ago)
    }
    /// The paths the entry threw away: those its copy holds and those that went uncopied alike.
    function filesOf(row) {
        let files = 0
        for (let i = 0; i < row.parts.length; i++) {
            if (row.parts[i].restore === "changes")
                files += row.parts[i].files + row.parts[i].notCopied
        }
        return files
    }
    /// What the operation did to what: the name first, then the doing, by git's word for it where git wrote the line.
    /// A move made on no branch names none (`name` empty): it was a detached HEAD's, said as leaving one is. The kind
    /// is the mark's to say (デザイン規約 §ref の種別「ref の名前の印」): a tag and a worktree are named with no word for
    /// what they are.
    function titleOf(row) {
        return shown.titleWith(row, shown.spokenOf(row), row.remote)
    }
    /// What a title calls the thing. The branches one rebase moved come from the model `, `-joined, which no branch
    /// name holds; a sentence says `a and b`, `a, b and c`, so the names read apart from one another.
    function spokenOf(row) {
        const names = shown.movedNames(row)
        return names.length < 2 ? row.name : shown.namesAnd(names.slice(0, -1).join(", "), names[names.length - 1])
    }
    function movedNames(row) {
        return row.kind === "rebase" ? row.name.split(", ") : [row.name]
    }
    function namesAnd(head, last) {
        return qsTr("%1 and %2").arg(head).arg(last)
    }
    /// The same in a row of the left menu, which opens on the kind's mark: every thing by its own name — a stash by
    /// its message, where a sentence says what it was.
    function listTitleOf(row) {
        return shown.listTitleWith(row, shown.spokenOf(row))
    }
    /// That row's words with `name` written where they say it.
    function listTitleWith(row, name) {
        switch (row.kind) {
        case "dropped-stash":
            return qsTr("%1 dropped").arg(name)
        case "popped-stash":
            return qsTr("%1 popped").arg(name)
        default:
            return shown.titleWith(row, name, row.remote)
        }
    }
    /// The names that row says, as code with no mark of their own — the row's mark says the kind (`CardText.marks`
    /// with the kind left empty): the title's, and a stash's message.
    function listMarksOf(row) {
        const named = row.kind === "dropped-stash" || row.kind === "popped-stash"
                      ? Words.marksOf(name => shown.listTitleWith(row, name), [[row.name, "stash"]])
                      : shown.titleMarksOf(row)
        return named.map(mark => [mark[0], "", mark[2], mark[3]])
    }
    /// The names the title says, each behind its kind's mark (デザイン規約 §ref の種別「名前の印」, `Words.marksOf`): the
    /// branch, the tag, the folder; the remote — its branch's `remote/name` behind the one cloud.
    function titleMarksOf(row) {
        const named = row.name === "" ? ""
            : ["reset", "amend", "rebase", "deleted", "moved"].indexOf(row.kind) >= 0 ? "branch"
            : ["deleted-tag", "deleted-remote-tag", "force-pushed-tag"].indexOf(row.kind) >= 0 ? "tag"
            : row.kind === "removed-worktree" ? "tree" : ""
        const marks = Words.marksOf((name, remote) => shown.titleWith(row, name, remote),
                                    [[shown.spokenOf(row), named], [row.remote, row.remote === "" ? "" : "remote"]])
        // A remote's branch is one name, `remote/name`, behind the one cloud: the cloud's name runs on over the `/`.
        const branchOver = "/" + row.name
        if (["deleted-remote-branch", "force-pushed"].indexOf(row.kind) >= 0)
            return marks.map(mark => mark[1] === "remote"
                                     && shown.titleOf(row).substr(mark[0] + mark[3], branchOver.length) === branchOver
                                     ? [mark[0], mark[1], mark[2], mark[3] + branchOver.length] : mark)
        const names = shown.movedNames(row)
        if (names.length < 2)
            return marks
        // One rebase that moved several branches names them all, each behind its own mark. A rebase reaches no remote,
        // so the names are the title's only mark.
        const head = names.slice(0, -1).join(", ")
        const last = names[names.length - 1]
        const headAt = Words.marksOf(shown.namesAnd, [[head, named], [last, ""]])[0][0]
        const lastAt = Words.marksOf(shown.namesAnd, [[head, ""], [last, named]])[0][0]
        return Words.listMarks(head, head, named).map(one => [marks[0][0] + headAt + one[0], named, undefined, one[3]])
                    .concat([[marks[0][0] + lastAt, named, undefined, last.length]])
    }
    /// That title with `name` and `remote` written where it says them.
    function titleWith(row, name, remote) {
        const files = shown.filesOf(row)
        const moved = name === "" ? qsTr("Detached HEAD") : name
        switch (row.kind) {
        case "reset":
            return qsTr("%1 reset").arg(moved)
        case "amend":
            return qsTr("%1 amended").arg(moved)
        case "rebase":
            return qsTr("%1 rebased").arg(moved)
        case "left-detached":
            // git's own words for it: `you are leaving 1 commit behind`.
            return qsTr("Detached HEAD left behind")
        case "deleted":
        case "deleted-tag":
            return remote === "" ? qsTr("%1 deleted").arg(name)
                                 : qsTr("%1 deleted both locally and on %2").arg(name).arg(remote)
        case "discarded":
            return files === 1 ? qsTr("1 file discarded") : qsTr("%1 files discarded").arg(files)
        case "deleted-untracked":
            return files === 1 ? qsTr("1 untracked file deleted") : qsTr("%1 untracked files deleted").arg(files)
        case "dropped-stash":
            return qsTr("Stash dropped")
        case "popped-stash":
            return qsTr("Stash popped")
        case "removed-worktree":
            return qsTr("%1 removed").arg(name)
        case "deleted-remote-branch":
            return qsTr("%1/%2 deleted").arg(remote).arg(name)
        case "deleted-remote-tag":
            return qsTr("%1 deleted on %2").arg(name).arg(remote)
        case "force-pushed":
            return qsTr("%1/%2 force-pushed").arg(remote).arg(name)
        case "force-pushed-tag":
            return qsTr("%1 force-pushed to %2").arg(name).arg(remote)
        default:
            // "moved": the branch set somewhere by hand — `branch --force`, the graph's `Move here`.
            return qsTr("%1 moved").arg(name)
        }
    }
    /// What one part took: the commits only its tip reaches, the work a copy holds, the stash by its message, the
    /// worktree by where it was, the tag by its commit.
    function takenOf(row, part) {
        switch (part.restore) {
        case "changes": {
            const held = row.kind === "deleted-untracked" ? qsTr("%1 untracked").arg(part.files)
                                                         : qsTr("%1 uncommitted").arg(part.files)
            if (part.notCopied === 0)
                return held
            // Nothing copied: only what went uncopied is said.
            if (part.files === 0)
                return qsTr("%1 not copied").arg(part.notCopied)
            return qsTr("%1, %2 not copied").arg(held).arg(part.notCopied)
        }
        case "stash":
            return part.name
        case "worktree":
            return part.name
        case "tag":
            return qsTr("At %1").arg(part.tip)
        default:
            return part.lost === 0 ? qsTr("No commits of its own") : shown.commits(part.lost)
        }
    }
    /// What the restore makes of it — a branch at the old tip under its own name where that is
    /// free and with the suffix where it is not, the tag on its own object, the worktree where it was, the stash
    /// back in the list, the work back in the worktree it was thrown out of.
    function restoredAs(part) {
        return shown.restoredWith(part, part.name, part.with)
    }
    /// That half with `name` and `with` written where it says them.
    function restoredWith(part, name, withName) {
        switch (part.restore) {
        case "changes":
            return qsTr("back into %1").arg(name)
        case "stash":
            return qsTr("back to the stashes")
        case "worktree":
            return withName === "" ? qsTr("as a worktree there")
                                   : qsTr("as a worktree there, on %1").arg(withName)
        case "tag":
            return qsTr("as %1").arg(name)
        default:
            return withName === "" ? qsTr("as %1").arg(name)
                                   : qsTr("as %1, tracking %2").arg(name).arg(withName)
        }
    }
    /// The names that half says, behind their marks: the worktree the work goes back into (`intoMark`), the branch a
    /// worktree comes back on, the tag, the branch and what it tracks.
    function restoreMarksOf(part, intoMark) {
        const named = part.restore === "changes" ? intoMark
                    : part.restore === "tag" ? "tag"
                    : part.restore === "stash" || part.restore === "worktree" ? "" : "branch"
        const withMark = part.restore === "worktree" ? "branch" : part.restore === "branch" || named === "branch"
                         ? "remote" : ""
        return Words.marksOf((name, withName) => shown.restoredWith(part, name, withName),
                             [[part.name, named], [part.with === undefined ? "" : part.with, withMark]])
    }
    function partOf(row, part) {
        // Thrown-away work goes back into a worktree named as everything else names it.
        const into = shown.worktreeAt(part.worktreePath, part.name)
        const said = part.restore === "changes" ? Object.assign({}, part, { name: into.name }) : part
        const mark = part.restore === "changes" ? "minus"
                   : part.restore === "stash" ? "stash"
                   : part.restore === "worktree" ? "tree"
                   : part.restore === "tag" ? "tag"
                   : part.remote !== "" ? "remote"
                   : "branch"
        // A part is one thing: the remote's tag wears its cloud; the remote's branch is the cloud already.
        const theirs = mark === "tag" && part.remote !== ""
        return {
            mark: mark,
            tint: shown.tintOf(mark, theirs),
            badged: theirs,
            text: shown.takenOf(row, part),
            restore: shown.restoredAs(said),
            restoreMarks: shown.restoreMarksOf(said, into.mark),
            look: part.look
        }
    }
    function entryOf(row) {
        const mark = shown.markOf(row)
        const where = shown.whereOf(row)
        // A remote's branch is REMOTES' cloud itself; a remote's tag has no mark of its own, so it wears the cloud.
        const theirs = where === "theirs" && mark === "tag"
        const worktree = shown.worktreeAt(row.worktreePath, row.worktree)
        return {
            // The entry across reads (`DiscardModel.keyAt`): what the list keeps its rows by.
            key: row.key,
            kind: row.kind,
            mark: mark,
            tint: shown.tintOf(mark, theirs),
            badged: theirs,
            paired: where === "both",
            title: shown.titleOf(row),
            titleMarks: shown.titleMarksOf(row),
            listTitle: shown.listTitleOf(row),
            listMarks: shown.listMarksOf(row),
            at: row.at,
            worktree: row.worktree === "" ? "" : worktree.name,
            worktreeMark: worktree.mark,
            parts: row.parts.map(part => shown.partOf(row, part)),
            byPart: row.byPart
        }
    }
}
