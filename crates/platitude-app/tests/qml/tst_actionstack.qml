import QtQuick
import QtTest
import platitude.ui

// The depth a button written on two lines asks of whoever holds it to one (rules-refs/app-ui.md「パネルの右端のボタンの
// 深さは `TopBar.actionDepth`」). Under a UI face whose two lines are shallower than the word over the mark (macOS) this
// figure is the frame's depth, and no desk here has such a face — so the figure is held to the layout it stands for.
Item {
    id: root
    width: 400
    height: 200
    /// The overwrite's colours here, apart from every other the button draws.
    readonly property color ink: "red"
    readonly property color face: "black"

    // As `TopBar` dresses the panel's four: a cell the row lays out, measured for the set's longest wording.
    component PanelButton: ActionButton {
        stacked: true
        laidByBand: true
        kind: "search"
        text: "Search"
        widestText: "Search"
        frameColor: Theme.borderStrong
        width: naturalWidth
        foldedDepth: stackDepth
    }

    PanelButton {
        id: worded
        height: stackDepth
    }
    PanelButton {
        id: marked
        x: 200
        foldRequested: true
        height: Theme.opsBarHeight
    }
    // `push -f` at rest, as `BandPushButton` dresses it: the push mark and the hold's ring as a fraction.
    PanelButton {
        id: forced
        y: 100
        kind: "push"
        text: "push -f"
        widestText: "push -f"
        code: true
        widestCode: true
        holdMs: Metrics.holdMs
        tone: root.ink
        faceColor: root.face
        height: stackDepth
    }

    TestCase {
        name: "ActionStack"
        when: windowShown

        /// The figure is the depth the layout takes with the word on it: a frame held to it holds both lines.
        function test_the_depth_asked_is_what_the_word_over_the_mark_takes() {
            verify(!worded.folded, "the cell holds its word")
            tryCompare(worded, "implicitHeight", worded.stackDepth)
        }

        /// The fold gives up the word, not the depth. What a folded cell draws is the mark alone, so the figure is not
        /// read off what is drawn now.
        function test_the_depth_asked_holds_once_the_word_is_given_up() {
            verify(marked.folded, "the row took the word")
            tryVerify(() => marked.implicitHeight < marked.stackDepth, undefined, "a folded cell draws less")
            compare(marked.stackDepth, worded.stackDepth)
        }

        /// In a frame that deep the hold's fraction keeps air over the frame's bottom line. One as deep as the seat put
        /// its ring on the line: the seat's foot is the frame's.
        function test_the_hold_ring_keeps_off_the_frame_line() {
            compare(forced.armedMs, Metrics.holdMs, "the button wears the hold")
            // The ring is the lower right of the fraction, and a canvas: on screen once red stands there.
            tryVerify(() => {
                const shot = grabImage(forced)
                for (let y = Math.floor(forced.height * 3 / 4); y < forced.height - Theme.borderWidth; ++y) {
                    for (let x = Math.ceil(forced.width / 2) + Theme.spaceXs / 2; x < forced.width; ++x) {
                        if (shot.pixel(x, y).r > 0.5)
                            return true
                    }
                }
                return false
            }, undefined, "the ring is on screen")
            const shot = grabImage(forced)
            const line = forced.height - Theme.borderWidth - 1
            for (let x = Theme.radiusSm + Theme.borderWidth; x < forced.width - Theme.radiusSm - Theme.borderWidth; ++x)
                verify(Qt.colorEqual(shot.pixel(x, line), root.face), "ink over the frame's line at x=" + x)
        }
    }
}
