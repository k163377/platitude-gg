import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The two message boxes are surfaces a click drops a caret on (デザイン規約 §コミットメッセージの 2 つの枠), and the
// box is the whole of what the frame encloses. **Twice now it has not been**, and neither strip could be seen: the
// inset the view is held off the frame by, which nothing stood in until `SweepBand`, and the sideways bar the style
// hands every `ScrollView` whether or not it has anywhere to go. Both are questions about how Qt hands a press to an
// item tree, which is what this file is for (規約 §右のペインの字は掴める).
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

    // The boxes' own shape, with the real hand under the real inset: a frame, a view held off it by `spaceXs`, and
    // the words inside the view. Here rather than through `MessageEditor`, because what these ask about is the text's
    // selection, which the pair reports nothing of.
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
                ScrollBar.horizontal: null
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

        /// The diagnosis a miss needs: the chain of items standing at that point, front-most last, with the sizes and
        /// the two numbers a bar answers by. "The caret did not land" names nothing on its own — what it was that
        /// took the press is the whole of the answer, and it is a strip nobody can see.
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
            waitForRendering(editor)
            return editor
        }

        /// **The whole of the summary frame drops a caret in the summary** — the words, the band around them, and
        /// the four corners. Swept on a grid rather than aimed at named points: what has gone wrong here twice is a
        /// strip nobody could see (the inset the view leaves, and the style's sideways bar), and a run that names
        /// its points is a run that walks over one (`details-sweep`, the same rule).
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

        /// And the same over the description's own frame, which is the same two strips one box further down.
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

        /// **The words keep every press they had.** The hand lies under the view, so a double click still reaches the
        /// text and selects a word — typing then replaces it, which shortens the summary. A hand laid over the view
        /// would take that press, leave nothing selected, and the same keystroke would make the summary longer.
        function test_words_keep_their_own_press() {
            const editor = makePair()
            const was = editor.subjectText
            mouseDoubleClickSequence(editor, Math.round(editor.width / 2), Math.round(editor.summaryNeed / 2))
            verify(editor.anyFocused)
            keyClick("X")
            verify(editor.subjectText.length < was.length)
        }

        /// A press in the band lands on the character nearest it: the top of the box is the head of the text and the
        /// bottom is its end, whatever the words are.
        function test_the_band_lands_on_the_nearest_character() {
            const frame = createTemporaryObject(box, root)
            verify(frame !== null)
            frame.field.text = "line one of the message\nline two of the message"
            waitForRendering(frame)
            mouseClick(frame, Math.round(frame.width / 2), 1)
            compare(frame.field.cursorPosition, 0)
            mouseClick(frame, Math.round(frame.width / 2), frame.height - 2)
            compare(frame.field.cursorPosition, frame.field.length)
        }

        /// And a drag out of the band is a selection, the way a drag out of any other margin is.
        function test_a_drag_from_the_band_selects() {
            const frame = createTemporaryObject(box, root)
            verify(frame !== null)
            frame.field.text = "line one of the message\nline two of the message"
            waitForRendering(frame)
            mousePress(frame, 1, 1)
            mouseMove(frame, Math.round(frame.width / 2), Math.round(frame.height / 2))
            mouseRelease(frame, Math.round(frame.width / 2), Math.round(frame.height / 2))
            verify(frame.field.selectedText !== "")
        }
    }
}
