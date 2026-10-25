import QtQuick
import QtTest
import platitude.ui

// The two hands a drag over words is taken with from the air beside them, entered as a pointer enters them — through
// their own `MouseArea`s, which runs pass by (they call the hands' functions): `SweepPad` under a card's words, and
// `SweepRoom` under the details pane's author row. The hands and the words stand at neither the root's origin nor
// each other's, and every point has x != y, so a point read in the wrong item or with its axes crossed lands on other
// characters. Characters are told apart by the mono face's one advance.
Item {
    id: root
    width: 700
    height: 300

    /// A card's ground: the pad under its words, which stand in from the card's corner.
    Item {
        id: card
        x: 30
        y: 20
        width: 320
        height: 100
        SweepPad {
            id: pad
            anchors.fill: parent
            content: words
        }
        /// Two lines with air between them, so the field a press lands nearest depends on where it is read.
        Item {
            id: words
            x: 50
            y: 10
            width: 220
            height: 80
            LineText {
                id: line
                y: 6
                width: parent.width
                mono: true
                text: "abcdefghijklmnop"
            }
            LineText {
                id: second
                y: 50
                width: parent.width
                mono: true
                text: "qrstuvwxyz012345"
            }
        }
    }
    TextMetrics {
        id: glyph
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontMd
        text: "a"
    }

    QtObject {
        id: commit
        property string authorName: "Ada Lovelace"
        property string authorEmail: "demo@example.com"
        property int authorTime: 1790751420
        property int avatar: 3
        property string avatarUrl: ""
        property string sha8: "3c1b3390"
        property string shaHex: "3c1b3390aaaabbbbccccddddeeeeffff00001111"
        property var coAuthors: []
        property var parentHexes: ["0a1b2c3d4aaaabbbbccccddddeeeeffff0000441"]
        property string parentHex: parentHexes[0]
    }
    CommitAuthorRow {
        id: row
        x: 20
        y: 160
        width: 640
        details: commit
        paneWidth: row.width
    }

    TestCase {
        name: "SweepHands"
        when: windowShown

        function init() {
            line.deselect()
            second.deselect()
            row.dropValues()
        }

        /// A quarter into the character after boundary `i` of the card's first line, `dy` under the line's foot — the
        /// card's air, which only the pad takes a press in. Whole pixels, as a pointer delivers them.
        function underLine(i, dy) {
            return inRoot(line, Qt.point((i + 0.25) * glyph.advanceWidth, line.height + dy))
        }
        /// A point of `item` in the root's coordinates, in whole pixels.
        function inRoot(item, position) {
            const p = item.mapToItem(root, position)
            return Qt.point(Math.round(p.x), Math.round(p.y))
        }

        // From the air between the two lines, nearer the first.
        function test_a_drag_from_a_card_s_air_takes_the_characters_it_crossed() {
            const from = underLine(2, 6)
            const to = underLine(6, 6)
            mousePress(root, from.x, from.y)
            mouseMove(root, (from.x + to.x) / 2, from.y)
            mouseMove(root, to.x, to.y)
            mouseRelease(root, to.x, to.y)
            compare(pad.endedOn, line.text, "the pad took the press and anchored on the nearer line")
            compare(line.selected, "cdef")
            compare(second.selected, "")
        }

        // From the gap beside the name, at the middle of the band the row gives the name's line (`bandFor`), onto the
        // name's far end and back across it at the height the press went down at.
        function test_a_drag_from_the_row_s_gap_takes_the_value_it_crossed() {
            tryVerify(() => row.valueReady("author"), undefined, "the name stands at its own width")
            const name = row.fieldFor("author")
            const slack = row.slackFor("author")
            const band = row.bandFor("author")
            const y = inRoot(row, Qt.point(0, (band[0] + band[1]) / 2)).y
            const from = Qt.point(inRoot(slack, Qt.point(slack.width / 2, 0)).x, y)
            const near = Qt.point(inRoot(name, Qt.point(name.width - 1, 0)).x, y)
            const far = Qt.point(inRoot(name, Qt.point(1, 0)).x, y)
            mousePress(root, from.x, from.y)
            mouseMove(root, near.x, near.y)
            mouseMove(root, far.x, far.y)
            mouseRelease(root, far.x, far.y)
            compare(row.sweptField(), commit.authorName, "the room anchored on the name, not the hash beside it")
            compare(row.selectedValue("author"), commit.authorName)
        }
    }
}
