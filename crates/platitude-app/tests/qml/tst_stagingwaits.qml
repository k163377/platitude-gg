import QtQuick
import QtTest
import platitude.ui
import platitude.ui.words

// The two writes that take the index whole and wait while a stage or unstage press holds them back
// (`RepoTab.stagingUnseen`), besides the commit button and HEAD's amend (`stage-held` / `stage-held-amend`): the
// stopped operation's `--continue` (`OpExitCard`) and the toolbar's `stash` (`BandStashButton`). Each goes down and
// says why in the same words (`Words.stagingWait`), the `git add` running included. The `stash` and every way out of
// the stopped operation stay down besides until what any command of the window's left has been read again
// (`RepoTab.settling`). Driven by their own inputs, the page's shape stood in for.
Item {
    id: root
    width: 400
    height: 600

    readonly property QtObject tab: QtObject {
        id: tab
        property int busyCount: 0
        property bool settling: false
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
            tab.settling = false
            tab.stagingUnseen = false
            tree.conflictCount = 0
            tree.opMerging = false
            tree.opStepping = true
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
        /// The ways out the card shows now: its rows by their flag, and the stopped merge's one button.
        function waysOut() {
            const out = []
            for (const code of ["--continue", "--skip", "--quit", "--abort"]) {
                const row = find(card, item => item.code === code)
                if (row !== null && row.visible)
                    out.push({ "name": code, "item": row })
            }
            const button = find(card, item => item.phraseHead === "merge --abort")
            if (button !== null && button.visible)
                out.push({ "name": "merge --abort", "item": button })
            return out
        }
        /// Every way out on show goes down while a command of the window's runs and stays down, with no word of its
        /// own, until what the command left has been read again — and takes no press from a run meanwhile.
        function staysDownUntilRead(shown) {
            const ways = waysOut()
            compare(ways.map(way => way.name), shown)
            for (const way of ways)
                verify(way.item.enabled, way.name + ": nothing out, it takes a press")
            tab.busyCount = 1
            tab.settling = true
            for (const way of ways)
                verify(!way.item.enabled, way.name + ": a command running")
            tab.busyCount = 0
            for (const way of ways) {
                verify(!way.item.enabled, way.name + ": git answered, and the stop on screen is still the one before")
                verify(!way.item.blockedWhy, way.name + ": no words, as while the command ran")
                verify(!card.completeOpExit(way.name === "merge --abort" ? "--abort" : way.name),
                       way.name + ": a run's press is turned away as a hand's is")
            }
            tab.settling = false
            for (const way of ways)
                verify(way.item.enabled, way.name + ": the screen has what the command left")
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

        function test_every_way_out_stays_down_until_what_a_command_left_is_read() {
            staysDownUntilRead(["--continue", "--skip", "--quit", "--abort"])
        }

        function test_a_row_pressed_through_waits_on_the_reading_too() {
            const row = continueRow()
            verify(!row.waiting)
            tab.busyCount = 1
            tab.settling = true
            verify(row.waiting, "the command running")
            tab.busyCount = 0
            verify(row.waiting, "git answered: the fill and the ring stay for the reading")
            tab.settling = false
            verify(!row.waiting)
        }

        /// A stopped merge's card is its one button (`OpExitCard.bare`).
        function test_the_merge_button_stays_down_until_what_a_command_left_is_read() {
            tree.opMerging = true
            tree.opStepping = false
            staysDownUntilRead(["merge --abort"])
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

        function test_stash_stays_down_until_what_a_command_left_is_read() {
            verify(stash.enabled, "nothing out: stash takes a press")
            tab.busyCount = 1
            tab.settling = true
            verify(!stash.enabled, "a command running")
            tab.busyCount = 0
            verify(!stash.enabled, "git answered, and what it left is still being read")
            compare(stash.tip, "", "no words, as while the command ran")
            tab.settling = false
            verify(stash.enabled, "the screen has what the command left")
        }
    }
}
