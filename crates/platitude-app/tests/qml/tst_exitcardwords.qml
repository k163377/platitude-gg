import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The exit card's words are wrapped, never cut (user decisions — `OpExitCard`'s `edit` line, `OpExitRow`): the
// `edit` stop's line ends on what to do, and a row's sentence is what the row does — a cut takes exactly those.
// Narrower than any pane gives: the `edit` line has to wrap, and no row's sentence breaks inside a word — beside its
// chip, or under it once a note leaves too little beside it (`OpExitRow.stacked`), the card growing by it. Wide, the
// sentences stand beside their chips.
//
// A control, to hand the card the window's font as `Main` does: whether a sentence breaks beside its chip is read in
// the face it draws in.
Control {
    id: root
    width: 200
    height: 600
    font.family: Theme.uiFamily
    font.pixelSize: Theme.fontMd

    readonly property QtObject tab: QtObject {
        id: tab
        property int busyCount: 0
        property bool settling: false
        property bool stagingUnseen: false
        function resolveOperation(how) {}
    }
    readonly property QtObject tree: QtObject {
        id: tree
        property string opText: "REBASING"
        property string opAlso: ""
        property int opStep: 1
        property int opSteps: 2
        property bool opEditing: true
        property string opEditOid: "80efa6640000000000000000000000000000abcd"
        property bool opMerging: false
        property bool opStepping: true
        property bool detached: false
        property string branch: "main"
        property bool opSkipFree: false
        property int conflictCount: 0
    }

    OpExitCard {
        id: card
        width: root.width
        repoTab: tab
        worktree: tree
    }
    /// A row on its own, walked across the width where its sentence goes under its chip (`breaksBeside`).
    OpExitRow {
        id: edgeRow
        y: root.height
        width: 480
        code: "--continue"
        text: "Carry on with what is staged"
        note: "conflicts stay"
    }
    /// One line of the card's words, off the font rather than the part: a part that cuts has a line height of its own
    /// to agree with.
    FontMetrics {
        id: smallLine
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontSm
    }
    FontMetrics {
        id: rowLine
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontMd
    }

    TestCase {
        name: "ExitCardWords"
        when: windowShown

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
        function footOf(item) {
            return item.mapToItem(card, Qt.point(0, item.height)).y
        }
        function topOf(item) {
            return item.mapToItem(card, Qt.point(0, 0)).y
        }
        function row(code) {
            const found = find(card, item => item.code === code)
            verify(found !== null, "the " + code + " row stands")
            return found
        }
        /// The row's sentence: the one piece of its text that is the row's own `text`, drawn (not the hidden ruler).
        function sentenceOf(item) {
            const found = find(item, child => child.text === item.text && child.wrapMode !== undefined
                                              && child.visible)
            verify(found !== null, "the " + item.code + " row has its sentence")
            return found
        }
        function chipOf(item) {
            const found = find(item, child => child.text === item.code && child.font !== undefined)
            verify(found !== null, "the " + item.code + " row has its chip")
            return found
        }
        /// The widest word of a sentence in the rows' face.
        function widestWord(text) {
            return Math.max(...text.split(" ").map(word => rowLine.advanceWidth(word)))
        }

        // Each test starts on the card as the root lays it out, with no conflict.
        function cleanup() {
            tree.conflictCount = 0
            card.width = Qt.binding(() => root.width)
        }

        function test_the_edit_line_wraps_whole_above_the_rows() {
            const words = find(card, item => typeof item.text === "string" && item.text.startsWith("Stopped"))
            verify(words !== null, "the edit line stands")
            // Layouts have no height-for-width: the line's height arrives a pass after its width.
            tryVerify(() => words.height >= 1.5 * smallLine.height, undefined,
                      "the line wrapped at this width: " + words.height + " tall, a line being " + smallLine.height)
            compare(words.height, words.implicitHeight, "no line of it was cut")
            const next = row("--continue")
            verify(footOf(words) <= topOf(next),
                   "the line ends at " + footOf(words) + ", under the --continue row at " + topOf(next))
        }

        function test_every_row_reads_whole_and_the_card_holds_them() {
            const codes = ["--continue", "--skip", "--quit", "--abort"]
            for (const code of codes) {
                const item = row(code)
                const words = sentenceOf(item)
                // Layouts have no height-for-width: the sentence's height arrives a pass after its width.
                tryVerify(() => Math.abs(words.height - words.implicitHeight) < 0.5, undefined,
                          code + "'s sentence took its height")
                verify(words.width >= widestWord(item.text) - 0.5,
                       code + "'s sentence has " + words.width + " for its widest word, "
                       + widestWord(item.text) + ": no word breaks inside")
                verify(!words.truncated, code + " was not cut")
                verify(footOf(words) <= footOf(item), code + "'s words end inside their row")
            }
            for (let i = 1; i < codes.length; i++)
                verify(footOf(row(codes[i - 1])) <= topOf(row(codes[i])),
                       codes[i - 1] + " ends at " + footOf(row(codes[i - 1])) + ", over " + codes[i] + " at "
                       + topOf(row(codes[i])))
            verify(footOf(row("--abort")) <= card.height,
                   "the last row stands inside a card " + card.height + " tall")
        }

        function leftOf(item) {
            return item.mapToItem(card, Qt.point(0, 0)).x
        }

        // Stacked, every row's sentence starts under its chip's line and at the chip's left edge — a row whose own
        // sentence would fit beside its chip too, since the card decides for all its rows at once; the note keeps the
        // first line. `--quit`'s note stands while a conflict does.
        function test_stacked_every_sentence_stands_under_its_chip() {
            tree.conflictCount = 1
            tryVerify(() => card.stacked)
            let alone = 0
            for (const code of ["--continue", "--skip", "--quit", "--abort"]) {
                const item = row(code)
                const chip = chipOf(item)
                const words = sentenceOf(item)
                if (!item.breaksBeside)
                    alone++
                tryVerify(() => topOf(words) >= footOf(chip) - 0.5, undefined,
                          code + "'s sentence starts at " + topOf(words) + ", under the chip ending at " + footOf(chip))
                verify(Math.abs(leftOf(words) - leftOf(chip)) < 0.5,
                       code + "'s sentence starts at x " + leftOf(words) + ", the chip at " + leftOf(chip))
            }
            verify(alone > 0, "a row that would fit beside its chip went down with the rest")
            const quit = row("--quit")
            const note = find(quit, child => child.text === quit.note && quit.note !== "" && child.visible)
            verify(note !== null, "--quit has its note")
            verify(topOf(note) < topOf(sentenceOf(quit)), "the note stands on the first line, over the sentence")
        }

        // Kept beside its chip, a sentence is handed room for its widest word on every width: the row tells where it
        // would break from the chip and the note as its layout spends them — each rounded up to a whole pixel —, so
        // just past where it stops going under, a word does not break inside.
        function test_a_sentence_kept_beside_its_chip_has_room_for_its_widest_word() {
            let under = 50
            let beside = 480
            edgeRow.width = under
            verify(edgeRow.breaksBeside, "a row this narrow sends its sentence under the chip")
            edgeRow.width = beside
            verify(!edgeRow.breaksBeside, "a row this wide keeps its sentence beside the chip")
            while (beside - under > 0.125) {
                const middle = (under + beside) / 2
                edgeRow.width = middle
                if (edgeRow.breaksBeside)
                    under = middle
                else
                    beside = middle
            }
            const grid = find(edgeRow, child => child.columns === 3)
            const words = sentenceOf(edgeRow)
            const ruler = find(edgeRow, child => child.text === edgeRow.text && child.wrapMode !== undefined
                                                 && !child.visible)
            verify(grid !== null && ruler !== null, "the row's layout and its widest word's ruler stand")
            for (let width = beside; width < beside + 3; width += 0.25) {
                edgeRow.width = width
                verify(!edgeRow.breaksBeside, "past " + beside + " the sentence stays beside, at " + width)
                verify(waitForItemPolished(grid), "the row is laid out at " + width)
                verify(words.width >= ruler.contentWidth,
                       "at " + width + " the sentence has " + words.width + " for its widest word, "
                       + ruler.contentWidth)
            }
        }

        // Wide, the sentences stand beside their chips, on the chip's line.
        function test_wide_the_sentence_stands_beside_its_chip() {
            card.width = 480
            tryVerify(() => !card.stacked, undefined, "a wide card keeps its rows on their lines")
            const item = row("--continue")
            const chip = chipOf(item)
            const words = sentenceOf(item)
            tryVerify(() => topOf(words) < footOf(chip), undefined,
                      "--continue's sentence starts at " + topOf(words) + ", beside the chip ending at "
                      + footOf(chip))
        }
    }
}
