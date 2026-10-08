import QtQuick
import QtTest
import platitude.ui

// The exit card's words are wrapped, never cut (デザイン規約 §フル interactive rebase / §進行中の操作から出る): the
// `edit` stop's line ends on what to do, and a row's sentence is what the row does — a cut takes exactly those. Narrower
// than any pane gives, so every line of it has to wrap: what is read is the wrapping, and the card growing by it.
Item {
    id: root
    width: 200
    height: 600

    QtObject {
        id: tab
        property int busyCount: 0
        function resolveOperation(how) {}
    }
    QtObject {
        id: tree
        property string opText: "REBASING"
        property string opAlso: ""
        property int opStep: 1
        property int opSteps: 2
        property bool opEditing: true
        property string opEditOid: "80efa6640000000000000000000000000000abcd"
        property bool opMerging: false
        property bool opStepping: true
        property bool opSkipFree: false
        property int conflictCount: 0
    }

    OpExitCard {
        id: card
        width: root.width
        repoTab: tab
        worktree: tree
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
            return item.mapToItem(card, 0, item.height).y
        }
        function topOf(item) {
            return item.mapToItem(card, 0, 0).y
        }
        function row(code) {
            const found = find(card, item => item.code === code)
            verify(found !== null, "the " + code + " row stands")
            return found
        }
        /// The row's sentence: the one piece of its text that is the row's own `text`.
        function sentenceOf(item) {
            const found = find(item, child => child.text === item.text && child.wrapMode !== undefined)
            verify(found !== null, "the " + item.code + " row has its sentence")
            return found
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

        function test_every_row_wraps_whole_and_the_card_holds_them() {
            const codes = ["--continue", "--skip", "--quit", "--abort"]
            for (const code of codes) {
                const item = row(code)
                const words = sentenceOf(item)
                tryVerify(() => words.height >= 1.5 * rowLine.height, undefined,
                          code + " wrapped at this width: " + words.height + " tall, a line being " + rowLine.height)
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
    }
}
