import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The two message boxes are surfaces a click drops a caret on (デザイン規約 §コミットメッセージの 2 つの枠), and the
// box is the whole of what the frame encloses. Two unseen strips can take the press — the inset the view is held off
// the frame by (`SweepBand` stands in it) and the sideways bar the style hands every `ScrollView` — both questions of
// how Qt hands a press to an item tree (規約 §右のペインの字は掴める).
Item {
    id: root
    width: 400
    height: 400

    Component {
        id: pair
        MessageEditor {
            width: 320
            height: 220
        }
    }

    // The boxes' own shape (a frame, a view held off it by `spaceXs`, the real hand under the inset), built here
    // because the pair reports nothing of the text's selection.
    Component {
        id: box
        Rectangle {
            property alias field: words
            width: 320
            height: 80
            SweepBand {
                anchors.fill: parent
                field: words
            }
            ScrollView {
                anchors.fill: parent
                anchors.margins: Theme.spaceXs
                ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
                TextArea {
                    id: words
                    wrapMode: TextArea.Wrap
                    background: null
                    padding: 0
                }
            }
        }
    }

    TestCase {
        name: "MessageBand"
        when: windowShown

        /// The chain of items at that point, front-most last, with sizes and a bar's numbers — a miss has to name what
        /// took the press.
        function under(item, x, y) {
            let at = item
            let px = x
            let py = y
            let out = ""
            for (let depth = 0; depth < 12; depth++) {
                const kid = at.childAt(px, py)
                if (!kid)
                    break
                const p = kid.mapFromItem(at, px, py)
                px = p.x
                py = p.y
                at = kid
                out += " > " + kid + "[" + Math.round(kid.x) + "," + Math.round(kid.y)
                       + " " + Math.round(kid.width) + "x" + Math.round(kid.height) + "]"
                if (kid.contentWidth !== undefined && kid.contentHeight !== undefined)
                    out += "{cw=" + Math.round(kid.contentWidth) + " ch=" + Math.round(kid.contentHeight) + "}"
                if (kid.orientation !== undefined && kid.policy !== undefined)
                    out += "{orient=" + kid.orientation + " policy=" + kid.policy
                           + " size=" + kid.size.toFixed(2) + "}"
            }
            return out
        }

        function makePair() {
            const editor = createTemporaryObject(pair, root)
            verify(editor !== null)
            editor.setMessage("alpha beta gamma delta", "body of the message")
            verify(waitForRendering(editor), "the pair was rendered")
            return editor
        }

        /// The sideways bar is refused by policy (`DescriptionBox`): `null` leaves a `TypeError` per box on every
        /// start, which no screenshot and no press can see.
        function test_a_refused_sideways_bar_says_nothing() {
            failOnWarning(/Cannot read property 'active' of null/)
            makePair()
            const frame = createTemporaryObject(box, root)
            verify(frame !== null)
            verify(waitForRendering(frame), "the box was rendered")
        }

        /// Swept on a grid: a run that names its points walks over an unseen strip (`details-sweep`, the same rule).
        function test_the_whole_summary_frame_lands_a_caret() {
            const editor = makePair()
            const top = editor.summaryNeed
            verify(top > 2 * Theme.spaceXs)
            const steps = 7
            for (let i = 0; i < steps; i++) {
                for (let j = 0; j < steps; j++) {
                    const x = Math.round(editor.width * (i + 0.5) / steps)
                    const y = Math.round(top * (j + 0.5) / steps)
                    editor.leaveBoxes()
                    verify(!editor.anyFocused)
                    mouseClick(editor, x, y)
                    if (!editor.anyFocused)
                        console.log("MISS " + x + "," + y + " of " + editor.width + "x" + top
                                    + " under=" + under(editor, x, y))
                    verify(editor.anyFocused)
                    verify(!editor.descriptionFocused)
                }
            }
        }

        function test_the_whole_description_frame_lands_a_caret() {
            const editor = makePair()
            const top = editor.summaryNeed + Theme.spaceXs
            const tall = editor.height - top
            verify(tall > 2 * Theme.spaceXs)
            const steps = 7
            for (let i = 0; i < steps; i++) {
                for (let j = 0; j < steps; j++) {
                    const x = Math.round(editor.width * (i + 0.5) / steps)
                    const y = top + Math.round((tall - 1) * (j + 0.5) / steps)
                    editor.leaveBoxes()
                    verify(!editor.descriptionFocused)
                    mouseClick(editor, x, y)
                    if (!editor.descriptionFocused)
                        console.log("MISS " + x + "," + y + " of " + editor.width + "x" + tall
                                    + " under=" + under(editor, x, y))
                    verify(editor.descriptionFocused)
                }
            }
        }

        /// The hand lies under the view, so a double click still selects a word and typing replaces it; a hand laid
        /// over the view would take the press, and the same keystroke would make the summary longer.
        function test_words_keep_their_own_press() {
            const editor = makePair()
            const was = editor.subjectText
            mouseDoubleClickSequence(editor, Math.round(editor.width / 2), Math.round(editor.summaryNeed / 2))
            verify(editor.anyFocused)
            keyClick("X")
            verify(editor.subjectText.length < was.length)
        }

        /// The top of the box is the head of the text and the bottom is its end, whatever the words are.
        function test_the_band_lands_on_the_nearest_character() {
            const frame = createTemporaryObject(box, root)
            verify(frame !== null)
            frame.field.text = "line one of the message\nline two of the message"
            verify(waitForRendering(frame), "the box was rendered")
            mouseClick(frame, Math.round(frame.width / 2), 1)
            compare(frame.field.cursorPosition, 0)
            mouseClick(frame, Math.round(frame.width / 2), frame.height - 2)
            compare(frame.field.cursorPosition, frame.field.length)
        }

        function test_a_drag_from_the_band_selects() {
            const frame = createTemporaryObject(box, root)
            verify(frame !== null)
            frame.field.text = "line one of the message\nline two of the message"
            verify(waitForRendering(frame), "the box was rendered")
            mousePress(frame, 1, 1)
            mouseMove(frame, Math.round(frame.width / 2), Math.round(frame.height / 2))
            mouseRelease(frame, Math.round(frame.width / 2), Math.round(frame.height / 2))
            verify(frame.field.selectedText !== "")
        }
    }
}
