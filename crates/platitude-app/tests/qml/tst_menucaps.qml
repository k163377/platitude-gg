import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui
import "painted.js" as Painted

// A menu row stands its words' capitals' middle on the row's middle, not their line box's (`AppMenuItem.capsLift`;
// デザイン規約 §メニュー): the UI faces keep more room over the capitals than under the baseline, so a centred line box
// stands the words low. A stopped operation's row is a menu row in a pane, its first line standing alike
// (`OpExitRow.capsLift`). Read off the label that paints the words, through every transform and margin over it.
Item {
    id: root
    width: 600
    height: 400

    AppMenu {
        id: menu
        AppMenuItem {
            text: "Create branch here…"
        }
        AppMenuItem {
            code: "rebase"
            text: "onto it"
        }
        AppMenu {
            title: "BRANCH"
            titleKind: "branch"
            AppMenuItem {
                text: "one"
            }
        }
    }

    OpExitRow {
        id: exitRow
        y: 300
        width: 400
        code: "--continue"
        text: "Carry on with what is staged"
    }

    TestCase {
        name: "MenuCaps"
        when: windowShown

        function cleanup() {
            menu.dismiss()
            tryVerify(() => !menu.visible)
        }

        /// How far below `middle` the capitals of `label`'s first line have their middle, in `row`'s y.
        function capitalsOff(row, label, middle) {
            const base = label.mapToItem(row, 0, label.baselineOffset).y
            return base - WordFace.capitalMiddle(label.font.pixelSize) - middle
        }

        function test_a_menu_rows_capitals_stand_on_its_middle() {
            verify(menu.offer())
            tryVerify(() => menu.opened)
            compare(menu.count, 3)
            for (let i = 0; i < menu.count; i++) {
                const row = menu.itemAt(i)
                const label = Painted.labelSaying(row.contentItem, row.text)
                verify(label !== null, "the words of " + row.text)
                const off = capitalsOff(row, label, row.height / 2)
                verify(Math.abs(off) <= 0.5, row.text + ": the capitals' middle is " + off + " off the row's")
            }
        }

        function test_a_stopped_operations_row_stands_its_first_line_as_a_menu_rows() {
            const label = Painted.labelSaying(exitRow, exitRow.text)
            verify(label !== null)
            const off = capitalsOff(exitRow, label, Theme.rowHeight / 2)
            verify(Math.abs(off) <= 0.5, "the capitals' middle is " + off + " off a menu row's")
        }
    }
}
