import QtQuick
import QtTest
import platitude.ui

// The two writes that take the index whole and wait while a stage or unstage press stands, besides the commit button
// and HEAD's amend (`stage-held` / `stage-held-amend`): the stopped operation's `--continue` (`OpExitCard`) and the
// toolbar's `stash` (`BandStashButton`). Each goes down and says why in the same words (`Words.stagingWait`), the
// `git add` running included. Driven by their own inputs, the page's shape stood in for (`RepoTab.stagingUnseen`).
Item {
    id: root
    width: 400
    height: 600

    readonly property QtObject tab: QtObject {
        id: tab
        property int busyCount: 0
        property bool stagingUnseen: false
        property string state: "open"
        function resolveOperation(how) {}
        function pushStash(name) {}
    }
    readonly property QtObject tree: QtObject {
        id: tree
        property string opText: "REBASING"
        property string opAlso: ""
        property int opStep: 1
        property int opSteps: 2
        property bool opEditing: false
        property string opEditOid: ""
        property bool opMerging: false
        property bool opStepping: true
        property bool detached: false
        property string branch: "main"
        property bool opSkipFree: false
        property int conflictCount: 0
        property bool loaded: true
        property string stashStanding: "ready"
    }
    readonly property QtObject page: QtObject {
        id: page
        property var pageTab: tab
        property var pageWorktree: tree
        property bool planShown: false
        property bool wipWritable: true
        property string pageStashName: ""
    }

    OpExitCard {
        id: card
        width: root.width
        repoTab: tab
        worktree: tree
    }
    BandStashButton {
        id: stash
        y: 400
        curPage: page
    }

    TestCase {
        name: "StagingWaits"
        when: windowShown

        function init() {
            tab.busyCount = 0
            tab.stagingUnseen = false
            tree.conflictCount = 0
        }

        /// The first item under `from` that `wanted` answers for, depth first.
        function find(from, wanted) {
            for (let i = 0; i < from.children.length; i++) {
                const child = from.children[i]
                if (wanted(child))
                    return child
                const deeper = find(child, wanted)
                if (deeper !== null)
                    return deeper
            }
            return null
        }
        function continueRow() {
            const found = find(card, item => item.code === "--continue")
            verify(found !== null, "the --continue row stands")
            return found
        }

        function test_continue_waits_on_a_press_and_says_so_while_git_runs() {
            const row = continueRow()
            verify(row.enabled, "nothing standing: the row takes a press")
            compare(row.blockedWhy, "")
            tab.stagingUnseen = true
            verify(!row.enabled, "a press standing holds the step's commit")
            compare(row.blockedWhy, Words.stagingWait)
            tab.busyCount = 1
            compare(row.blockedWhy, Words.stagingWait, "the git add running: still the staging's word")
            tab.busyCount = 0
            tab.stagingUnseen = false
            verify(row.enabled, "the screen caught up: the row takes a press again")
            compare(row.blockedWhy, "")
        }

        function test_continue_says_nothing_over_a_conflict() {
            const row = continueRow()
            tab.stagingUnseen = true
            tree.conflictCount = 1
            verify(!row.enabled)
            compare(row.blockedWhy, "", "the conflicted rows say why")
        }

        function test_continue_asks_for_its_tip_under_the_hand() {
            const row = continueRow()
            tab.stagingUnseen = true
            mouseMove(row, row.width / 2, row.height / 2)
            tryVerify(() => row.blockedTip.asked, undefined, "the hand on a held row asks for its words")
            compare(row.blockedTip.words, Words.stagingWait)
            mouseMove(root, root.width - 1, root.height - 1)
            tryVerify(() => !row.blockedTip.asked, undefined, "the hand gone, the ask goes")
        }

        function test_stash_waits_on_a_press_and_says_so_while_git_runs() {
            verify(stash.enabled, "nothing standing: stash takes a press")
            tab.stagingUnseen = true
            verify(!stash.enabled, "a press standing holds what stash sets aside")
            compare(stash.tip, Words.stagingWait)
            tab.busyCount = 1
            compare(stash.tip, Words.stagingWait, "the git add running: still the staging's word")
            tab.stagingUnseen = false
            compare(stash.tip, "", "another command running alone: the band says why")
            tab.busyCount = 0
            verify(stash.enabled, "nothing standing again")
        }
    }
}
