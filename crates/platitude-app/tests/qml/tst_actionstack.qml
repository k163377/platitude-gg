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
    }
}
