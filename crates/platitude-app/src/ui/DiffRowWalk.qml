pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// Where the diff's reading position stands, and whether the arrows are this pane's to answer (the arrows' shared rules:
// 規約 §矢印で履歴を辿る). The one place the keyboard is written from — the wheel, a drag, the middle hand and the
// kept place send the view's `contentY` too; its visibility is the pane's.
//
// Up / Down move the view, one row a press: nothing here follows a selection, so a lit row would bring nothing along.
// The unit is the wheel's row — a notch sends `Metrics.wheelRows`, a press one — so the same surface is not sent in two
// kinds of step by two hands (規約 §グラフ・インタラクション定数). Stops at the ends.
Item {
    id: walk

    required property var view

    /// The hand has arrived in this pane — the wheel, or a press in the rows — and the keyboard comes with it (user
    /// decision — ask before changing). The press that opened the diff landed in the file list, so that hand is still
    /// the list's, and until it arrives here Up / Down are the list's to answer (`FileRowWalk`). Automation comes in by
    /// the same door. Only to a list on screen — an image-only
    /// preview draws no rows and hides it: Qt keeps active focus on an invisible item and the keys go on arriving
    /// there, so something not on screen would hold the keys.
    function handArrived() {
        if (walk.view.visible)
            walk.view.forceActiveFocus()
    }
    /// Sends the view `delta` rows (∓1 per press) and answers whether it moved — `false` at either end leaves the key
    /// unaccepted. The keys and the automation hook both come through here (verify-ui).
    function stepRows(delta) {
        if (!walk.visible || !walk.view.visible || walk.view.count === 0)
            return false
        const was = walk.view.contentY
        walk.view.cancelFlick()
        // A wheel notch still in flight was aimed at rows this step is walking away from (`WheelGlide.halt`).
        walk.view.haltGlide()
        walk.view.contentY = walk.view.clampY(was + delta * Theme.rowHeight)
        return walk.view.contentY !== was
    }
    /// Automation: whether the view is as far down as it goes — a diff with nothing to scroll is at its end.
    readonly property bool atEnd: walk.view.contentY >= walk.view.maxY - 0.5

    // Let go on the way out (規約 §矢印で履歴を辿る「画面から退いたペインはキーボードを手放す」); on the way in the
    // keyboard waits for the hand (`handArrived`).
    onVisibleChanged: {
        if (!walk.visible)
            walk.view.focus = false
    }
}
