import QtQuick
import QtTest
import platitude.ui

// The colours a confirmed control's mark and fill are drawn in (デザイン規約 §回数押しの色). The mark wears the colour
// its words wear — the lift over the fill, an asleep heading's grey, a warning — but a white delete keeps the plain ink
// for its words alone: its mark and fill carry the warning. A control that cannot be pressed greys its words, and its
// mark keeps the hue it wears when it can, a step down; a mark with no hue to keep greys with the words (§無効).
Item {
    id: root
    width: 400
    height: 200

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
    AppMenuItem {
        id: row
        text: "worktree remove"
        holdPresses: Metrics.holdPressesPlain
        holdTone: Theme.textPrimary
    }
    OpExitRow {
        id: exitRow
        code: "--skip"
        holdPresses: Metrics.holdPressesWarned
        holdTone: Theme.danger
    }
    ActionButton {
        id: button
        text: "Discard hunk"
        holdPresses: Metrics.holdPressesPlain
        holdTone: Theme.textPrimary
    }
    AskBar {
        id: ask
        width: 400
        open: true
        hold: true
        accept: "Replace"
    }

    TestCase {
        name: "HoldInk"

        /// Every press mark inside `item` that has presses to show — a button carries one per seat it can show, and a
        /// seat it does not use counts none.
        function marksIn(item) {
            let found = []
            for (const child of item.children) {
                if (child.presses !== undefined && child.ink !== undefined && child.presses > 0)
                    found.push(child)
                found = found.concat(marksIn(child))
            }
            return found
        }
        function compareMarks(item, ink, what) {
            const marks = marksIn(item)
            verify(marks.length > 0, what + ": a mark to read")
            for (const one of marks)
                verify(Qt.colorEqual(one.ink, ink), what + ": " + one.ink)
        }

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
            mark.off = false
            mark.tint = given.tint
            verify(Qt.colorEqual(mark.ink, given.ink), given.tag + ": " + mark.ink)
        }
        // Off, a hue goes a step down (§暗く落とした段); a grey has no step of its own and takes §無効's.
        function test_mark_off_data() {
            return [
                { tag: "white delete", tint: Theme.textPrimary, ink: Theme.warningDim },
                { tag: "warning", tint: Theme.warning, ink: Theme.warningDim },
                { tag: "danger", tint: Theme.danger, ink: Theme.dangerDim },
                { tag: "accent", tint: Theme.accent, ink: Theme.accentDim },
                { tag: "asleep", tint: Theme.textSecondary, ink: Theme.textMuted }
            ]
        }
        function test_mark_off(given) {
            mark.tint = given.tint
            mark.off = true
            verify(Qt.colorEqual(mark.ink, given.ink), given.tag + ": " + mark.ink)
            mark.off = false
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

        function test_menu_row_data() {
            return [
                { tag: "white delete", tone: Theme.textPrimary, live: Theme.warning, off: Theme.warningDim },
                { tag: "warning", tone: Theme.warning, live: Theme.warning, off: Theme.warningDim },
                { tag: "danger", tone: Theme.danger, live: Theme.danger, off: Theme.dangerDim }
            ]
        }
        function test_menu_row(given) {
            row.holdTone = given.tone
            row.enabled = true
            compareMarks(row, given.live, given.tag + " live")
            row.enabled = false
            verify(Qt.colorEqual(row.wordColor, Theme.textMuted), given.tag + " words: " + row.wordColor)
            compareMarks(row, given.off, given.tag + " off")
            row.enabled = true
        }
        function test_exit_row() {
            compareMarks(exitRow, Theme.danger, "live")
            exitRow.enabled = false
            verify(Qt.colorEqual(exitRow.wordColor, Theme.textMuted), "words: " + exitRow.wordColor)
            compareMarks(exitRow, Theme.dangerDim, "off")
            exitRow.enabled = true
        }
        // The hunk heading's `Discard hunk`: awake under the pointer, asleep in the heading's grey off it
        // (デザイン規約 §diff の中のステージ). Off, it keeps whichever it wore; the words grey either way.
        function test_button_data() {
            return [
                { tag: "awake", tone: Theme.textPrimary, hold: Theme.textPrimary,
                  live: Theme.warning, off: Theme.warningDim },
                { tag: "awake, unrecorded", tone: Theme.danger, hold: Theme.danger,
                  live: Theme.danger, off: Theme.dangerDim },
                { tag: "asleep", tone: Theme.textSecondary, hold: Theme.textPrimary,
                  live: Theme.textSecondary, off: Theme.textMuted }
            ]
        }
        function test_button(given) {
            button.tone = given.tone
            button.holdTone = given.hold
            button.enabled = true
            compareMarks(button, given.live, given.tag + " live")
            button.enabled = false
            verify(Qt.colorEqual(button.fg, Theme.textMuted), given.tag + " words: " + button.fg)
            compareMarks(button, given.off, given.tag + " off")
            button.enabled = true
        }
        function test_ask_pill() {
            compareMarks(ask, Theme.warning, "live")
            ask.answerable = false
            compareMarks(ask, Theme.warningDim, "off")
            ask.answerable = true
        }
    }
}
