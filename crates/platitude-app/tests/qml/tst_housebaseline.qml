import QtQuick
import QtQuick.Layouts
import QtTest
import platitude.ui

// The house stands its foot on the baseline of the word beside it, the two set in one line's middle; a line-drawn
// mark keeps that middle (デザイン規約 §ref の種別「家は語のベースラインに立つ」, `NavIcon.standOn`). Centred, the
// house's flat foot floated a pixel or two over the letters.
Item {
    id: root
    width: 320
    height: 120

    Component {
        id: line
        RowLayout {
            property alias mark: mark
            property alias word: word
            width: 300
            height: Theme.rowHeight
            spacing: 0
            Item {
                Layout.preferredWidth: Theme.iconSm
                Layout.preferredHeight: Theme.iconSm
                Layout.alignment: Qt.AlignVCenter
                NavIcon {
                    id: mark
                    anchors.centerIn: parent
                    anchors.verticalCenterOffset: mark.standOn(word.wordBase, word.height)
                    width: Theme.iconSm
                    height: Theme.iconSm
                }
            }
            CutName {
                id: word
                text: "repo"
                pixelSize: Theme.fontMd
            }
        }
    }

    TestCase {
        name: "HouseBaseline"
        when: windowShown

        /// Where the mark's ink ends at the foot, in the line's own frame.
        function footOf(made) {
            const ink = made.mark.inkTallGrid / 16 * made.mark.height
            return made.mark.mapToItem(made, 0, (made.mark.height + ink) / 2).y
        }
        /// Where the word's letters stand, in the same frame.
        function baselineOf(made) {
            return made.word.mapToItem(made, 0, made.word.wordBase).y
        }

        function test_the_house_stands_on_the_words_baseline() {
            const made = createTemporaryObject(line, root)
            made.mark.kind = "home"
            verify(waitForItemPolished(made), "the line is laid out")
            verify(made.mark.anchors.verticalCenterOffset > 0, "the house goes down from the line's middle")
            fuzzyCompare(footOf(made), baselineOf(made), 0.5, "its foot is on the baseline")
        }

        function test_a_line_drawn_mark_keeps_the_middle() {
            const made = createTemporaryObject(line, root)
            made.mark.kind = "tree"
            verify(waitForItemPolished(made), "the line is laid out")
            compare(made.mark.anchors.verticalCenterOffset, 0)
        }
    }
}
