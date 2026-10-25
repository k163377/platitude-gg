import QtQuick
import QtTest
import platitude.ui

// What a menu's rows wait for (`AppMenu.rowsHeldWhy`): the rows that start git grey behind held doors and under a
// command running as the menu opened; a row that only reads what is on screen (`Copy path`) stays live under both.
// When that command ends with the menu up, the menu says so (`waitEnded`) and its opener stands the rows again with the
// menu keeping its shape (`holdShape`). Which rows a real menu holds is `tst_commitrowmenu.qml`'s.
Item {
    id: root
    width: 320
    height: 240

    /// What the shaped menu's last two rows stand on.
    property bool goesOffered: true
    property bool comesOffered: false

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
            text: "Stays"
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

    /// What a remote's menu reads off its tab.
    readonly property QtObject remoteTab: QtObject {
        id: remoteTab
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

    SignalSpy {
        id: ended
        target: menu
        signalName: "waitEnded"
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
            ended.clear()
            remoteMenu.menu.dismiss()
            remoteMenu.runningWhy = ""
            remoteTab.markedOrigin = "origin"
            remoteTab.remoteNames = ["origin", "fork"]
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

        /// Said once, and only for a menu still up that opened under the command: one started under a menu already up
        /// was never waited for, and a menu gone has nothing to stand again.
        function test_a_wait_ends_with_the_menu_up_that_waited() {
            menu.runningWhy = Words.otherCommandRunning
            verify(menu.offer())
            menu.runningWhy = ""
            compare(ended.count, 1)
            verify(!starts.blocked, "the rows come back")

            menu.runningWhy = Words.otherCommandRunning
            menu.runningWhy = ""
            compare(ended.count, 1, "a command started under the menu up is not one it waited for")
            menu.dismiss()

            menu.runningWhy = Words.otherCommandRunning
            verify(menu.offer())
            menu.dismiss()
            tryVerify(() => !menu.visible)
            menu.runningWhy = ""
            compare(ended.count, 1, "nor one that ends with the menu down")
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

        /// A remote's menu stands again the same way: the mark row the command made moot greys where it is; a remote
        /// the command took away takes the menu down.
        function test_a_remote_menu_stands_again_keeping_its_shape() {
            remoteMenu.runningWhy = Words.otherCommandRunning
            verify(remoteMenu.offerOn("fork"))
            compare(remoteMenu.menu.offeredRows, 2)
            const mark = remoteMenu.menu.itemAt(1)
            remoteTab.markedOrigin = "fork"
            verify(mark.present, "held still while it waits")
            remoteMenu.runningWhy = ""
            compare(remoteMenu.menu.offeredRows, 2, "stood again with its rows")
            verify(mark.withdrawn, "the mark it no longer needs greys where it is")

            remoteMenu.menu.dismiss()
            tryVerify(() => !remoteMenu.menu.visible)
            remoteTab.markedOrigin = "origin"
            remoteMenu.runningWhy = Words.otherCommandRunning
            verify(remoteMenu.offerOn("fork"))
            remoteTab.remoteNames = ["origin"]
            remoteMenu.runningWhy = ""
            tryVerify(() => !remoteMenu.menu.visible, undefined, "a remote gone leaves the menu nothing to act on")
        }
    }
}
