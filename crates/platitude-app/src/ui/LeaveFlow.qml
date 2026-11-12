pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

// A move something stands in the way of — an operation stopped, or paths left unmerged — asked about in the one
// question bar before anything is sent (デザイン規約 §進行中の操作から出る, §立っている質問は 1 か所で聞く). What blocks a
// move and what leaving costs are core's (`offers::moves_blocked` / `leaving_undoes` / `leave_code`); the words are
// here. Draws nothing: the bar belongs to the graph.
//
// **The bar says the tree as it is while it stands**, never as it was at the press: a conflict resolved or a rebase
// carried on outside changes what the answer does, so the heading, the line, the dress, the hold, the command and the
// marked row move with it (the bindings below).
//
// **Once nothing stands in the way, the bar asks the move the press was for** (`switch`, in the accent, its line naming
// what went), and its answer is that plain move (user decision — ask before changing): what conflict screens in other
// git GUIs do when the conflicts are resolved outside (they turn to `Continue`). Neither left saying a stash the answer
// would no longer make, nor taken down with nothing said. Something in the way again turns it back.
//
// Headless: `switch-mark <branch>:asks|plain`, `switch-stopped[-go]`, `switch-conflicted[-go]`, `switch-held`.
Item {
    id: leaveFlow

    /// Where the tree stands (`WorktreeModel`): what is in the way, what leaving it costs, and HEAD's commit.
    required property var worktree
    /// The one bar this question stands in, and whose words it dresses while it stands (`RepoPage.startRowAsk` raises
    /// it).
    required property var graphPane

    /// Raise the standing question; the page owns the bar. `label` comes with the landing branch's seat in it (`%1`).
    signal askRequested(string oidHex, string label, string detail, bool danger, var run, string code,
                        string refName, color refTint)

    /// Whether this question is the one standing, which is what its bindings on the bar hang off.
    property bool asking: false
    /// The branch the move lands on.
    property string local: ""
    /// Sends the move, given whether it leaves what stands in the way (`switchToRef` / `switchTo`'s `leaving`).
    property var go: null
    /// What stood in the way at the last read that had anything there: the operation by git's name ("" for none) and
    /// whether any path was unmerged. The line names what went from these — once an operation is over, the tree names
    /// it no more. Taken again at every read while the way is blocked (`noteBlockers`), so it is the change the bar
    /// just saw, never the tree at the press.
    property string lastOp: ""
    property bool lastConflicts: false

    /// Whether anything still stands in the move's way (core's `offers::moves_blocked`).
    readonly property bool blocked: leaveFlow.worktree.movesBlocked
    /// Whether leaving undoes work in hand: `danger` and a hold (§状態, §回数押し).
    readonly property bool undoes: leaveFlow.blocked && leaveFlow.worktree.leaveUndoes
    /// The command on the question's line and its pill.
    readonly property string code: leaveFlow.blocked ? leaveFlow.worktree.leaveCode : "switch"
    // Either heading reads on from its chip (`rebase --abort` / `stash`) as one sentence and names where the move
    // lands. What the stash takes is every uncommitted change, untracked files too (`session::build::leave_operation`).
    // Conflicts are named while any stand: git's own stash refuses an unmerged file, so they are what the reader would
    // not expect to go in. The stash question's second line only says the standing operation ends; with nothing
    // standing its heading is the whole question (user decision — ask before changing).
    readonly property string sentence:
        //: Read on from the `switch` chip at its head, once nothing stands in the move's way; %1 is the branch.
        !leaveFlow.blocked ? qsTr("to %1?")
        //: Read on from the `rebase --abort` chip at its head; %1 is the branch the move lands on.
        : leaveFlow.undoes ? qsTr("and switch to %1?")
        //: Read on from the `stash` chip at its head; %1 is the branch the move lands on.
        : leaveFlow.worktree.conflictCount > 0 ? qsTr("all uncommitted changes including conflicts and switch to %1?")
        //: Read on from the `stash` chip at its head; %1 is the branch the move lands on.
        : qsTr("all uncommitted changes and switch to %1?")
    // Once the way is clear, the line names what went (user decision — ask before changing): the operation, where one
    // stood — an abort takes its conflicts with it, so they are not called resolved —, else the conflicts, by what is
    // true however they went (resolved, or thrown away). A blocker git names neither way (a bisect) leaves the heading
    // alone.
    readonly property string detail:
        //: %1 is the operation that stood in the way, in git's own spelling, e.g. cherry-pick; it is over now.
        !leaveFlow.blocked ? (leaveFlow.lastOp !== "" ? qsTr("The %1 has ended.").arg(leaveFlow.lastOp)
                              //: Said once no path is unmerged any more, the conflicts having stood in the way.
                              : leaveFlow.lastConflicts ? qsTr("No conflicts are left.")
                              : "")
        : leaveFlow.undoes ? qsTr("Nothing it did since it started is kept.")
        : leaveFlow.worktree.opCommand === "" ? ""
        //: %1 is the standing operation in git's own spelling, e.g. cherry-pick.
        : qsTr("The %1 ends here.").arg(leaveFlow.worktree.opCommand)
    /// The landing branch in the current branch's colour where HEAD is on it (§ref の種別; `GitFacts.isCurrent`).
    readonly property color landingTint:
        GitFacts.isCurrent(leaveFlow.worktree.standing, leaveFlow.local) ? Theme.textLink : Theme.textPrimary

    /// Opens the question on a move to `local`; `go(leaving)` sends that move.
    function startAsk(local, go) {
        leaveFlow.local = local
        leaveFlow.go = go
        leaveFlow.noteBlockers()
        // Marked on HEAD's row — a rebase runs detached, so that is the only name the tree has for where it is
        // (デザイン規約 §立っている質問は 1 か所で聞く).
        leaveFlow.askRequested(leaveFlow.worktree.headOid, leaveFlow.sentence, leaveFlow.detail, leaveFlow.undoes,
                               leaveFlow.answer, leaveFlow.code, local, leaveFlow.landingTint)
        // After the bar is up: raising it resets the properties the `asking` bindings below own, and a binding whose
        // value has not changed does not push back.
        leaveFlow.asking = true
    }
    /// The pill: the question as it stands at the press. Leaving only while something is in the way — the plain move
    /// goes through the gate again, so a blocker come back meanwhile raises this question again rather than being put
    /// down unasked.
    function answer() {
        leaveFlow.go(leaveFlow.blocked)
    }
    /// Reads what stands in the way off the tree as one read left it. Read off the model, not off `blocked`: inside the
    /// read's one notice the bindings over it may not have caught up yet.
    function noteBlockers() {
        if (!leaveFlow.worktree.movesBlocked)
            return
        leaveFlow.lastOp = leaveFlow.worktree.opCommand
        leaveFlow.lastConflicts = leaveFlow.worktree.conflictCount > 0
    }
    // Every read while the question stands: the one notice a status read raises (`WorktreeModel.changed`).
    Connections {
        target: leaveFlow.worktree
        enabled: leaveFlow.asking
        function onChanged() {
            leaveFlow.noteBlockers()
        }
    }

    // Bindings, since the tree moves under the question while it stands. `RestoreNone`: letting go happens at the
    // press that walks away, with the bar still on screen (`PublishFlow`).
    Binding {
        target: leaveFlow.graphPane
        property: "askLabel"
        value: leaveFlow.sentence.arg(leaveFlow.local)
        when: leaveFlow.asking
        restoreMode: Binding.RestoreNone
    }
    // The landing branch in its own colour and behind its mark (`AskBar.labelSentence`).
    Binding {
        target: leaveFlow.graphPane
        property: "askLabelSentence"
        value: leaveFlow.sentence
        when: leaveFlow.asking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: leaveFlow.graphPane
        property: "askLabelRefTint"
        value: leaveFlow.landingTint
        when: leaveFlow.asking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: leaveFlow.graphPane
        property: "askDetail"
        value: leaveFlow.detail
        when: leaveFlow.asking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: leaveFlow.graphPane
        property: "askDanger"
        value: leaveFlow.undoes
        when: leaveFlow.asking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: leaveFlow.graphPane
        property: "askHold"
        value: leaveFlow.undoes
        when: leaveFlow.asking
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: leaveFlow.graphPane
        property: "askCode"
        value: leaveFlow.code
        when: leaveFlow.asking
        restoreMode: Binding.RestoreNone
    }
    // The move alone costs nothing, so it asks as an information question does (`AskBar.neutral`).
    Binding {
        target: leaveFlow.graphPane
        property: "askNeutral"
        value: !leaveFlow.blocked
        when: leaveFlow.asking
        restoreMode: Binding.RestoreNone
    }
    // A rebase carried on outside stops again further along: the mark goes with HEAD.
    Binding {
        target: leaveFlow.graphPane
        property: "askOid"
        value: leaveFlow.worktree.headOid
        when: leaveFlow.asking
        restoreMode: Binding.RestoreNone
    }
}
