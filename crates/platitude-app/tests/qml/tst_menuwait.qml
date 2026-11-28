import QtQuick
import QtTest
import platitude.ui
import platitude.ui.words

// What a menu's rows wait for (`AppMenu.rowsHeldWhy`): the rows that start git grey behind held doors and under a
// command running as the menu opened; a row that only reads what is on screen (`Copy path`) stays live under both.
// When that command ends with the menu up, its `MenuLines` stands it again once (`commandEnded`) and the opener stands
// the rows with the menu keeping its shape (`holdShape`). A card's title row greys with the rows inside it and still
// opens the card.
// Which rows a real menu holds is `tst_commitrowmenu.qml`'s.
Item {
    id: root
    width: 320
    height: 240

    /// What the shaped menu's last two rows stand on.
    property bool goesOffered: true
    property bool comesOffered: false
    /// What the shaped menu's first row says.
    property string staysText: "Stays"
    /// Why the carried card's command row is greyed, and whether its reading row is on show.
    property string insideWhy: ""
    property bool readsInside: false

    AppMenu {
        id: menu
        AppMenuItem {
            id: starts
            text: "Discard"
        }
        AppMenuItem {
            id: reads
            text: "Copy path"
            startsCommand: false
        }
    }

    AppMenu {
        id: shaped
        AppMenuItem {
            id: stays
            text: root.staysText
        }
        AppMenuItem {
            id: goes
            text: "Goes"
            offered: root.goesOffered
        }
        AppMenuItem {
            id: comes
            text: "Comes"
            offered: root.comesOffered
        }
    }

    AppMenu {
        id: carrying
        AppMenuItem {
            id: carried
            text: "Pull"
        }
        AppMenu {
            id: card
            title: "BRANCH"
            runningWhy: carrying.runningWhy
            AppMenuItem {
                id: inside
                text: "Set upstream…"
                blockedReason: root.insideWhy
            }
            AppMenuItem {
                text: "Copy name"
                startsCommand: false
                offered: root.readsInside
            }
        }
    }

    /// What a remote's menu reads off its tab.
    readonly property QtObject remoteTab: QtObject {
        id: remoteTab
        property string state: "open"
        property string markedOrigin: "origin"
        property var remoteNames: ["origin", "fork"]
    }

    /// No remote here is GitHub's: the menu's GITHUB card stays out (`GitHubLinks.ofRemote`).
    readonly property QtObject noLinks: QtObject {
        id: noLinks
        function ofRemote(name) {
            return []
        }
    }

    RemoteRowMenu {
        id: remoteMenu
        repoTab: remoteTab
        githubLinks: noLinks
    }

    /// How often the remote's menu stood again, and as what (`MenuLines.standAgain`).
    SignalSpy {
        id: stood
        target: remoteMenu.lines
        signalName: "standAgain"
    }
    /// Each follow the remote's menu asked for a turn later, as it comes (`MenuLines.followLater`).
    SignalSpy {
        id: followCame
        target: remoteMenu.lines.followLater
        signalName: "triggered"
    }

    TestCase {
        name: "MenuWait"
        when: windowShown

        function cleanup() {
            menu.dismiss()
            menu.runningWhy = ""
            menu.heldReason = ""
            shaped.dismiss()
            shaped.releaseShape()
            root.goesOffered = true
            root.comesOffered = false
            root.staysText = "Stays"
            remoteMenu.menu.dismiss()
            remoteMenu.runningWhy = ""
            remoteMenu.treeAtAsk = null
            stood.clear()
            followCame.clear()
            remoteTab.markedOrigin = "origin"
            remoteTab.remoteNames = ["origin", "fork"]
            card.dismiss()
            carrying.dismiss()
            carrying.runningWhy = ""
            root.insideWhy = ""
            root.readsInside = false
        }

        /// The title row `sub` hangs off in the carrying menu.
        function titleOf(sub) {
            for (let i = 0; i < carrying.count; i++) {
                const row = carrying.itemAt(i)
                if (row && row.subMenu === sub)
                    return row
            }
            return undefined
        }

        /// A card whose every row is greyed greys the title row it hangs off, which still opens it: the rows inside say
        /// why (`AppMenuItem.cardOut`). One row in it that can be pressed keeps the title live.
        function test_a_card_title_greys_with_every_row_inside_and_still_opens() {
            root.insideWhy = "No commits yet"
            verify(carrying.offer())
            const title = titleOf(card)
            verify(title.greyed, "the title greys with the row inside")
            verify(!title.blocked, "with no reason of its own")
            verify(carrying.openSub(card), "and the card still opens")
            compare(inside.blockedWhy, "No commits yet", "where the row says why")
            root.readsInside = true
            verify(!title.greyed, "a row in it that can be pressed keeps the title live")
            root.readsInside = false
            root.insideWhy = ""
            verify(!title.greyed, "and so does the greyed row coming back")
        }

        /// Opened under a command, a menu's cards wait with it before they open (`AppMenu.settleWaiting`), so their
        /// titles grey from the first; the command over, they come back.
        function test_a_card_title_greys_under_the_command_its_menu_opened_under() {
            carrying.runningWhy = Words.otherCommandRunning
            verify(carrying.offer())
            verify(carried.blocked, "the menu's own row waits")
            const title = titleOf(card)
            verify(!card.visible, "the card is not open")
            verify(title.greyed, "and its title greys with the rows waiting in it")
            carrying.runningWhy = ""
            verify(!carried.blocked)
            verify(!title.greyed, "the command over, the title is live again")
        }

        function test_a_row_that_only_reads_is_never_held() {
            menu.runningWhy = Words.otherCommandRunning
            verify(menu.offer())
            compare(starts.blockedWhy, Words.otherCommandRunning, "a row that starts git waits for the one running")
            verify(!reads.blocked, "one that reads the screen does not")
            menu.dismiss()

            menu.runningWhy = ""
            menu.heldReason = Words.otherCommandRunning
            verify(menu.offer())
            verify(starts.blocked)
            verify(!reads.blocked, "nor behind the held doors")
        }

        /// The page lets a command go as `RepoPage.onRunningWhyChanged` does: the rows' reason and the tree it was
        /// asked on, in the order `treeFirst` says.
        function endCommand(treeFirst) {
            if (treeFirst)
                remoteMenu.treeAtAsk = null
            remoteMenu.runningWhy = ""
            remoteMenu.treeAtAsk = null
        }
        function startCommand() {
            remoteMenu.runningWhy = Words.otherCommandRunning
            remoteMenu.treeAtAsk = ({})
        }

        /// A command's end stands a menu that waited for it once, as a wait, whichever of the two the page lets go
        /// first (`MenuLines.commandEnded`): standing on both would ask git's answers again for nothing
        /// (`RefBranchMenu.standOn`).
        function test_a_command_end_stands_a_menu_that_waited_once() {
            for (const treeFirst of [false, true]) {
                stood.clear()
                startCommand()
                verify(remoteMenu.offerOn("fork"))
                endCommand(treeFirst)
                compare(stood.count, 1, "once, the tree let go first: " + treeFirst)
                compare(stood.signalArguments[0][0], "wait", "as a wait")
                remoteMenu.menu.dismiss()
                tryVerify(() => !remoteMenu.menu.visible)
            }
        }

        /// A command started under a menu already up was never waited for: its end stands the menu once, following
        /// what it moved. A menu down has nothing to stand again.
        function test_a_command_end_under_a_menu_raised_before_it_follows_once() {
            verify(remoteMenu.offerOn("fork"))
            startCommand()
            endCommand(false)
            compare(stood.count, 1)
            compare(stood.signalArguments[0][0], "follow", "following what it moved")
            remoteMenu.menu.dismiss()
            tryVerify(() => !remoteMenu.menu.visible)

            stood.clear()
            startCommand()
            verify(remoteMenu.offerOn("fork"))
            remoteMenu.menu.dismiss()
            tryVerify(() => !remoteMenu.menu.visible)
            endCommand(false)
            compare(stood.count, 0, "nor one that ends with the menu down")
        }

        /// A command's end lets go the wait of a menu that waited for it (`MenuLines.commandEnded`): the next command,
        /// started with the menu still up, was not out as it opened and greys nothing (`AppMenu.waiting`).
        function test_a_command_end_lets_the_wait_go_for_the_next_command() {
            startCommand()
            verify(remoteMenu.offerOn("fork"))
            verify(remoteMenu.menu.waiting, "opened under the command")
            endCommand(false)
            verify(!remoteMenu.menu.waiting, "let go at its end")
            startCommand()
            compare(remoteMenu.menu.rowsHeldWhy, "", "the next command was not out as the menu opened")
            endCommand(false)
        }

        /// A command's end landing in the turn of its move stands the menu once, as a wait: that stand reads what
        /// moved, so the follow the move asked for a turn later finds nothing left (`MenuLines.followDue`).
        function test_a_command_end_in_the_turn_of_its_move_stands_once() {
            startCommand()
            verify(remoteMenu.offerOn("fork"))
            stood.clear()
            followCame.clear()
            remoteTab.markedOrigin = "fork"
            endCommand(false)
            compare(stood.count, 1)
            compare(stood.signalArguments[0][0], "wait")
            tryCompare(followCame, "count", 1)
            compare(stood.count, 1, "the follow came and stood nothing")
        }

        /// The rows on show as the shape is held stay, a row that stopped applying greyed where it is; a row that
        /// started applying waits for the menu to be raised again (デザイン規約 §メニュー).
        function test_a_held_shape_keeps_its_rows_and_lets_no_other_in() {
            verify(shaped.offer())
            compare(shaped.offeredRows, 2)
            shaped.holdShape()
            root.goesOffered = false
            root.comesOffered = true
            verify(goes.present, "a row that stopped applying stays where it was")
            verify(goes.withdrawn)
            compare(goes.blockedWhy, goes.withdrawnWhy, "greyed, saying the menu has to be raised again")
            verify(!comes.present, "no row comes in under the hand")
            compare(shaped.offeredRows, 2)
            verify(!stays.blocked, "and the rest stay live")

            shaped.holdShape()
            verify(!comes.present, "holding a held shape again changes nothing")

            root.goesOffered = true
            verify(!goes.withdrawn, "a row that applies again is live again")

            shaped.dismiss()
            shaped.releaseShape()
            root.goesOffered = false
            verify(shaped.offer())
            verify(!goes.present, "raised again, the menu shows what applies then")
            verify(comes.present)
        }

        /// What the rows say is not held with the shape: a card whose row now says more settles as wide as one raised
        /// on those words, and back again as they shrink (デザイン規約 §メニュー「凍らせるのは行の出入りとチップ列の幅だけ」).
        function test_a_held_shape_lets_the_width_follow_the_words() {
            verify(shaped.offer())
            const opened = shaped.width
            shaped.holdShape()
            root.staysText = "Stays, and says a great deal more than it did as the menu opened"
            verify(shaped.width > opened, "the card widens to the words")
            const grown = shaped.width
            shaped.dismiss()
            tryVerify(() => !shaped.visible)
            shaped.releaseShape()
            verify(shaped.offer())
            compare(shaped.width, grown, "as wide as a menu raised on those words")
            shaped.holdShape()
            root.staysText = "Stays"
            compare(shaped.width, opened, "and narrow again with them")
        }

        /// A remote's menu raised with nothing running follows what its rows read as it moves outside: the mark row
        /// made moot greys where it is, saying the mark is there; a remote taken away takes the menu down.
        function test_a_remote_menu_follows_what_moves_outside() {
            verify(remoteMenu.offerOn("fork"))
            const mark = remoteMenu.menu.itemAt(1)
            verify(!mark.greyed)
            remoteTab.markedOrigin = "fork"
            tryVerify(() => mark.greyed, undefined, "the mark it no longer needs greys where it is")
            compare(mark.blockedReason, Words.menuWhy("already-origin"), "saying the mark is there")
            compare(remoteMenu.menu.offeredRows, 2, "and the menu keeps its rows")
            remoteTab.remoteNames = ["origin"]
            tryVerify(() => !remoteMenu.menu.visible, undefined, "a remote gone leaves the menu nothing to act on")
        }

        /// While a command the menus wait for is out (`treeAtAsk` set), a remote's menu up follows nothing it reads:
        /// the rows keep what they stood on, and the command's end stands them on what it left.
        function test_a_remote_menu_follows_nothing_while_a_command_is_out() {
            verify(remoteMenu.offerOn("fork"))
            const mark = remoteMenu.menu.itemAt(1)
            remoteMenu.treeAtAsk = ({})
            remoteTab.markedOrigin = "fork"
            remoteMenu.lines.follow()
            verify(!mark.greyed, "nothing followed while the command is out")
            remoteMenu.treeAtAsk = null
            verify(mark.greyed, "its end stands the rows on what it left")
            compare(mark.blockedReason, Words.menuWhy("already-origin"))
        }

        /// A remote's menu stands again the same way: the mark row the command made moot greys where it is; a remote
        /// the command took away takes the menu down.
        function test_a_remote_menu_stands_again_keeping_its_shape() {
            startCommand()
            verify(remoteMenu.offerOn("fork"))
            compare(remoteMenu.menu.offeredRows, 2)
            const mark = remoteMenu.menu.itemAt(1)
            remoteTab.markedOrigin = "fork"
            verify(mark.present, "held still while it waits")
            endCommand(false)
            compare(remoteMenu.menu.offeredRows, 2, "stood again with its rows")
            verify(mark.greyed, "the mark it no longer needs greys where it is")
            compare(mark.blockedReason, Words.menuWhy("already-origin"))

            remoteMenu.menu.dismiss()
            tryVerify(() => !remoteMenu.menu.visible)
            remoteTab.markedOrigin = "origin"
            startCommand()
            verify(remoteMenu.offerOn("fork"))
            remoteTab.remoteNames = ["origin"]
            endCommand(false)
            tryVerify(() => !remoteMenu.menu.visible, undefined, "a remote gone leaves the menu nothing to act on")
        }
    }
}
