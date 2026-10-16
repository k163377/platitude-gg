import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// A menu row for a tab standing in a linked worktree draws the tab's own run after its words (`AppMenuItem.trail`,
// `TabTreeMark`): the row asks for the run's whole width, so the card that opens round it never cuts the folder's name,
// and the run stands a step past the words.
//
// A control, to hand the card the window's font as `Main` does: the run is priced in the face its name is drawn in.
Control {
    id: root
    width: 600
    height: 300
    font.family: Theme.uiFamily
    font.pixelSize: Theme.fontMd

    AppMenu {
        id: menu
        AppMenuItem {
            text: "platitude-gg"
            trail: "a-worktree-folder-with-a-long-name"
        }
        AppMenuItem {
            text: "short"
        }
    }

    TestCase {
        name: "MenuTrail"
        when: windowShown

        /// The first item under `item` that answers `probe`, depth first.
        function under(item, probe) {
            for (const child of item.children) {
                if (probe(child))
                    return child
                const found = under(child, probe)
                if (found !== null)
                    return found
            }
            return null
        }

        function test_the_worktree_s_run_is_drawn_whole_a_step_past_the_words() {
            verify(menu.offer(), "the card has rows to offer")
            tryCompare(menu, "opened", true)
            const row = menu.itemAt(0)
            const run = under(row, child => child.floorWidth !== undefined && child.naturalWidth !== undefined)
            verify(run !== null, "the row draws the tab's run")
            const name = under(run, child => child.cutting !== undefined)
            tryVerify(() => run.width >= run.naturalWidth, undefined, "the run is laid out at its own width")
            compare(name.cutting, false, "the folder's name is whole")
            const words = under(row, child => child.text === "platitude-gg")
            const wordsEnd = words.mapToItem(row, words.width, 0).x
            verify(run.mapToItem(row, 0, 0).x >= wordsEnd, "the run starts past the words")
            compare(under(menu.itemAt(1), child => child.floorWidth !== undefined).visible, false,
                    "a row with no worktree draws no run")
            menu.dismiss()
        }
    }
}
