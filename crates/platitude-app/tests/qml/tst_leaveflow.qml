import QtQuick
import QtTest
import platitude.ui

// The question a blocked move raises says the tree as it is while it stands (`LeaveFlow`): a conflict resolved or an
// operation put down outside changes what the answer does, so the heading, the line, the dress, the hold, the command
// and the marked row move with the tree — none of them stays what it was at the press. With nothing in the way any
// more it asks the move the press was for, and its answer is that move.
Item {
    id: root
    width: 400
    height: 300

    /// What the bar's pill runs, as the page holds it (`RepoPage.rowAskRun`).
    property var run: null

    /// The worktree's facts the question reads (`WorktreeModel`), and its one notice that a read landed.
    readonly property QtObject tree: QtObject {
        id: tree
        signal changed()
        property bool movesBlocked: true
        property bool leaveUndoes: false
        property string leaveCode: "stash"
        property string opCommand: "cherry-pick"
        property int conflictCount: 1
        property string headOid: "aaaa000000000000000000000000000000000001"
        property var standing: ({ branch: "main" })
    }
    /// The bar's dress as `GraphPane` carries it, written as `GraphPane.startAsking` writes it.
    readonly property QtObject pane: QtObject {
        id: pane
        property string askLabel: ""
        property string askLabelSentence: ""
        property string askLabelRef: ""
        property color askLabelRefTint: "transparent"
        property string askDetail: ""
        property bool askDanger: false
        property bool askHold: false
        property bool askNeutral: false
        property string askCode: ""
        property string askOid: ""
        function startAsking(oidHex, label, detail, danger, hold, code, refName, refTint) {
            pane.askNeutral = false
            pane.askLabelSentence = label
            pane.askLabelRef = refName
            pane.askLabelRefTint = refTint
            pane.askLabel = label.arg(refName)
            pane.askDetail = detail
            pane.askCode = code
            pane.askDanger = danger
            pane.askHold = hold
            pane.askOid = oidHex
        }
    }

    LeaveFlow {
        id: leave
        worktree: tree
        graphPane: pane
        // As the page raises it: whoever dressed the bar lets go first (`RepoPage.startRowAsk`).
        onAskRequested: (oidHex, label, detail, danger, run, code, refName, refTint) => {
            leave.asking = false
            root.run = run
            pane.startAsking(oidHex, label, detail, danger, danger, code, refName, refTint)
        }
    }

    TestCase {
        name: "LeaveFlow"
        when: windowShown

        function init() {
            leave.asking = false
            tree.movesBlocked = true
            tree.leaveUndoes = false
            tree.leaveCode = "stash"
            tree.opCommand = "cherry-pick"
            tree.conflictCount = 1
            tree.headOid = "aaaa000000000000000000000000000000000001"
            tree.standing = { branch: "main" }
        }

        /// One status read: the facts change together, then the model says so once (`WorktreeModel.changed`).
        function read(facts) {
            for (const key in facts)
                tree[key] = facts[key]
            tree.changed()
        }

        /// The conflicts the heading names go from it once they are resolved outside; the line about the standing
        /// operation stays, since the operation does.
        function test_the_heading_follows_conflicts_resolved_outside() {
            leave.startAsk("feature/clash", () => {})
            compare(pane.askLabel, "all uncommitted changes including conflicts and switch to feature/clash?")
            compare(pane.askDetail, "The cherry-pick ends here.")
            tree.conflictCount = 0
            compare(pane.askLabel, "all uncommitted changes and switch to feature/clash?")
            compare(pane.askLabelSentence, "all uncommitted changes and switch to %1?")
            compare(pane.askDetail, "The cherry-pick ends here.")
            tree.conflictCount = 2
            compare(pane.askLabel, "all uncommitted changes including conflicts and switch to feature/clash?")
        }

        /// A rebase put down outside with its conflicts left behind: leaving no longer undoes anything, so the bar
        /// takes off the danger and the hold, says `stash`, and its line goes with the operation.
        function test_the_dress_and_the_command_follow_what_leaving_costs() {
            tree.leaveUndoes = true
            tree.leaveCode = "rebase --abort"
            tree.opCommand = "rebase"
            leave.startAsk("feature/clash", () => {})
            compare(pane.askLabel, "and switch to feature/clash?")
            compare(pane.askDetail, "Nothing it did since it started is kept.")
            verify(pane.askDanger, "a leave that undoes work wears the danger")
            verify(pane.askHold, "…and is held")
            compare(pane.askCode, "rebase --abort")
            tree.leaveUndoes = false
            tree.leaveCode = "stash"
            tree.opCommand = ""
            compare(pane.askLabel, "all uncommitted changes including conflicts and switch to feature/clash?")
            compare(pane.askDetail, "")
            verify(!pane.askDanger, "the danger goes with what leaving undid")
            verify(!pane.askHold, "…and so does the hold")
            compare(pane.askCode, "stash")
        }

        /// A rebase carried on outside stops again further along: the mark is on HEAD's row as it is now.
        function test_the_mark_follows_head() {
            leave.startAsk("feature/clash", () => {})
            compare(pane.askOid, "aaaa000000000000000000000000000000000001")
            tree.headOid = "bbbb000000000000000000000000000000000002"
            compare(pane.askOid, "bbbb000000000000000000000000000000000002")
        }

        /// Nothing in the way any more — the conflicts gone with nothing standing: the bar asks the move the press was
        /// for, in the dress of a question that costs nothing, and its line says what went; something in the way again
        /// turns it back.
        function test_the_question_turns_into_the_switch_once_nothing_stands() {
            tree.opCommand = ""
            leave.startAsk("feature/clash", () => {})
            read({ conflictCount: 0, movesBlocked: false })
            compare(pane.askLabel, "to feature/clash?")
            compare(pane.askLabelSentence, "to %1?")
            compare(pane.askDetail, "No conflicts are left.")
            compare(pane.askCode, "switch")
            verify(pane.askNeutral, "a move that costs nothing asks in the accent")
            verify(!pane.askDanger, "…with no danger")
            verify(!pane.askHold, "…and no hold")
            read({ conflictCount: 1, movesBlocked: true })
            compare(pane.askLabel, "all uncommitted changes including conflicts and switch to feature/clash?")
            compare(pane.askDetail, "")
            compare(pane.askCode, "stash")
            verify(!pane.askNeutral, "in the way again: the stash question's dress")
        }

        /// The operation ending outside clears the way: the line names it, by the name the last read in the way gave
        /// it — once it is over, git names it no more.
        function test_the_line_names_the_operation_that_ended() {
            leave.startAsk("feature/clash", () => {})
            read({ conflictCount: 0 })
            compare(pane.askDetail, "The cherry-pick ends here.")
            read({ opCommand: "", movesBlocked: false })
            compare(pane.askDetail, "The cherry-pick has ended.")
        }

        /// An abort takes its conflicts with it — they were not resolved — so the line names the operation alone.
        function test_an_operation_gone_with_its_conflicts_is_named_alone() {
            tree.leaveUndoes = true
            tree.leaveCode = "rebase --abort"
            tree.opCommand = "rebase"
            leave.startAsk("main", () => {})
            read({ leaveUndoes: false, leaveCode: "stash", opCommand: "", conflictCount: 0, movesBlocked: false })
            compare(pane.askLabel, "to main?")
            compare(pane.askDetail, "The rebase has ended.")
        }

        /// The line says what stood in the way at the last read, not at the press: a rebase put down first and its
        /// conflicts gone after say the conflicts, the rebase's going having already changed the bar.
        function test_the_line_says_the_last_thing_in_the_way() {
            tree.leaveUndoes = true
            tree.leaveCode = "rebase --abort"
            tree.opCommand = "rebase"
            leave.startAsk("main", () => {})
            read({ leaveUndoes: false, leaveCode: "stash", opCommand: "" })
            compare(pane.askLabel, "all uncommitted changes including conflicts and switch to main?")
            read({ conflictCount: 0, movesBlocked: false })
            compare(pane.askDetail, "No conflicts are left.")
        }

        /// The pill answers the question on screen: it leaves while something is in the way, and once nothing is it
        /// moves the plain way — never putting down what the bar no longer asks about.
        function test_the_answer_is_the_question_on_screen() {
            const sent = []
            leave.startAsk("feature/clash", leaving => sent.push(leaving))
            root.run()
            read({ opCommand: "", conflictCount: 0, movesBlocked: false })
            root.run()
            compare(sent, [true, false])
        }

        /// The landing branch is drawn in the current branch's colour only while HEAD is on it.
        function test_the_landing_name_takes_the_current_colour_from_the_tree() {
            leave.startAsk("feature/clash", () => {})
            verify(Qt.colorEqual(pane.askLabelRefTint, Theme.textPrimary), "a branch HEAD is not on: the plain ink")
            tree.standing = { branch: "feature/clash" }
            verify(Qt.colorEqual(pane.askLabelRefTint, Theme.textLink), "the branch HEAD is on: the link colour")
        }

        /// Another question raised over this one owns the bar: the tree moving no longer writes this one's words.
        function test_a_question_let_go_of_writes_nothing() {
            leave.startAsk("feature/clash", () => {})
            leave.asking = false
            pane.startAsking("cccc000000000000000000000000000000000003", "Move %1 here?", "", false, false, "",
                             "feature/clash", Theme.textPrimary)
            tree.conflictCount = 0
            tree.headOid = "bbbb000000000000000000000000000000000002"
            compare(pane.askLabel, "Move feature/clash here?")
            compare(pane.askOid, "cccc000000000000000000000000000000000003")
        }
    }
}
