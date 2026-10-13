import QtQuick
import QtTest
import platitude.ui

// The colours a confirmed control's mark and fill are drawn in, from the colour its words wear (デザイン規約
// §回数押しの色). A white delete keeps the plain ink for its words alone: its mark and fill carry the warning, and every
// other colour the words take — the lift over the fill, a greyed row, an asleep heading, a warning — is the mark's too.
Item {
    id: root
    width: 200
    height: 40

    HoldIcon {
        id: mark
    }
    Item {
        width: 100
        height: 20
        HoldFill {
            id: fill
            progress: 0.5
            tone: Theme.warning
        }
    }

    TestCase {
        name: "HoldInk"

        function test_mark_data() {
            return [
                { tag: "white delete", tint: Theme.textPrimary, ink: Theme.warning },
                { tag: "over the fill", tint: Theme.textOnAccent, ink: Theme.textOnAccent },
                { tag: "greyed", tint: Theme.textMuted, ink: Theme.textMuted },
                { tag: "asleep", tint: Theme.textSecondary, ink: Theme.textSecondary },
                { tag: "warning", tint: Theme.warning, ink: Theme.warning },
                { tag: "danger", tint: Theme.danger, ink: Theme.danger }
            ]
        }
        function test_mark(given) {
            mark.tint = given.tint
            verify(Qt.colorEqual(mark.ink, given.ink), given.tag + ": " + mark.ink)
        }

        function test_fill_data() {
            return [
                { tag: "white delete", tone: Theme.textPrimary, full: Theme.warning, down: Theme.warningDim },
                { tag: "warning", tone: Theme.warning, full: Theme.warning, down: Theme.warningDim },
                { tag: "danger", tone: Theme.danger, full: Theme.danger, down: Theme.dangerDim }
            ]
        }
        function test_fill(given) {
            fill.tone = given.tone
            verify(Qt.colorEqual(fill.full, given.full), given.tag + " full: " + fill.full)
            verify(Qt.colorEqual(fill.down, given.down), given.tag + " down: " + fill.down)
        }
    }
}
