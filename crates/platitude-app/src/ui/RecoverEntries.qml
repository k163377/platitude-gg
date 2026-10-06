import QtQuick
import platitude.ui

// What the discard log's rows say (破棄記録仕様.md §3): one operation that took something away, as the model reads it —
// `{kind, name, remote, worktree, at, lost, parts, byPart}`, each part `{restore, name, with, look, tip, remote, lost,
// files, notCopied}` (`DiscardModel.rows`). The rows draw `{mark, tint, badged, paired, title, at, worktree, parts,
// byPart}`, each part `{mark, tint, badged, text, restore}` (`RecoverPane`, `RecoverBand`, `RecoverHoverCard`, the
// marks through `RecoverMark`) — the words are said here, where they can be translated.
//
// The verbs are the ones other tools bring things back with: GitHub's `Restore branch`, the Windows Recycle Bin's
// `Restore all items` (`RecoverBand`).
QtObject {
    id: shown

    /// The model's entries, newest first.
    property var rows: []

    readonly property var entries: shown.rows.map(row => shown.entryOf(row))

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
    /// moment in full is the card's and the band's (`whenWords`).
    function ago(at, now) {
        const secs = Math.max(0, now - at)
        if (secs < 60)
            return qsTr("Just now")
        if (secs < 3600)
            return qsTr("%1 min ago").arg(Math.floor(secs / 60))
        if (secs < 86400)
            return qsTr("%1 h ago").arg(Math.floor(secs / 3600))
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
    /// What the operation did, by git's word for it where git wrote the line. A move made on no branch names none
    /// (`name` empty): it was a detached HEAD's, said as leaving one is.
    function titleOf(row) {
        const files = shown.filesOf(row)
        const moved = row.name === "" ? qsTr("a detached HEAD") : row.name
        switch (row.kind) {
        case "reset":
            return qsTr("Reset %1").arg(moved)
        case "amend":
            return qsTr("Amended %1").arg(moved)
        case "rebase":
            return qsTr("Rebased %1").arg(moved)
        case "left-detached":
            return qsTr("Left a detached HEAD")
        case "deleted":
            return row.remote === "" ? qsTr("Deleted %1").arg(row.name)
                                     : qsTr("Deleted %1 here and on %2").arg(row.name).arg(row.remote)
        case "discarded":
            return files === 1 ? qsTr("Discarded 1 file") : qsTr("Discarded %1 files").arg(files)
        case "deleted-untracked":
            return files === 1 ? qsTr("Deleted 1 untracked file") : qsTr("Deleted %1 untracked files").arg(files)
        case "dropped-stash":
            return qsTr("Dropped a stash")
        case "popped-stash":
            return qsTr("Popped a stash")
        case "deleted-tag":
            return row.remote === "" ? qsTr("Deleted tag %1").arg(row.name)
                                     : qsTr("Deleted tag %1 here and on %2").arg(row.name).arg(row.remote)
        case "removed-worktree":
            // The kind said, as a tag's is: the folder's name alone does not say what it names.
            return qsTr("Removed worktree %1").arg(row.name)
        case "deleted-remote-branch":
            return qsTr("Deleted %1/%2").arg(row.remote).arg(row.name)
        case "deleted-remote-tag":
            return qsTr("Deleted tag %1 on %2").arg(row.name).arg(row.remote)
        case "force-pushed":
            return qsTr("Force-pushed %1/%2").arg(row.remote).arg(row.name)
        case "force-pushed-tag":
            return qsTr("Force-pushed tag %1 to %2").arg(row.name).arg(row.remote)
        default:
            // "moved": the branch set somewhere by hand — `branch --force`, the graph's `Move here`.
            return qsTr("Moved %1").arg(row.name)
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
    /// What the restore makes of it (破棄記録仕様.md §4) — a branch at the old tip under its own name where that is
    /// free and with the suffix where it is not, the tag on its own object, the worktree where it was, the stash
    /// back in the list, the work back in the worktree it was thrown out of.
    function restoredAs(part) {
        switch (part.restore) {
        case "changes":
            return qsTr("back into %1").arg(part.name)
        case "stash":
            return qsTr("back to the stashes")
        case "worktree":
            return part.with === "" ? qsTr("as a worktree there")
                                    : qsTr("as a worktree there, on %1").arg(part.with)
        case "tag":
            return qsTr("as tag %1").arg(part.name)
        default:
            return part.with === "" ? qsTr("as branch %1").arg(part.name)
                                    : qsTr("as branch %1, tracking %2").arg(part.name).arg(part.with)
        }
    }
    function partOf(row, part) {
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
            restore: shown.restoredAs(part),
            look: part.look
        }
    }
    function entryOf(row) {
        const mark = shown.markOf(row)
        const where = shown.whereOf(row)
        // A remote's branch is REMOTES' cloud itself; a remote's tag has no mark of its own, so it wears the cloud.
        const theirs = where === "theirs" && mark === "tag"
        return {
            kind: row.kind,
            mark: mark,
            tint: shown.tintOf(mark, theirs),
            badged: theirs,
            paired: where === "both",
            title: shown.titleOf(row),
            at: row.at,
            worktree: row.worktree,
            parts: row.parts.map(part => shown.partOf(row, part)),
            byPart: row.byPart
        }
    }
}
