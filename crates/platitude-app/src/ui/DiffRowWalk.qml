pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// Where the diff's reading position stands, and whether the arrows are
// this pane's to answer (規約 §diff を上下に送る).
//
// Nothing here draws — every line is a write to the view's `contentY` or
// to where the keyboard is, and this is the one place either is written
// from. Its own visibility is the pane's: it sits inside it.
Item {
    id: walk

    /// The list being sent.
    required property var view

    /// Where the keyboard goes when this pane comes on screen. Unlike the
    /// graph, which waits to be clicked because a window has several
    /// places worth typing into, the diff arrives *because* a hand pressed
    /// a file in CHANGES — that press already said "read here", so the
    /// pane takes the keyboard by arriving (デザイン規約 §diff を上下に送る).
    ///
    /// Refused to a list that is not on screen: an image-only preview
    /// hands its space to the picture and draws no rows. Focus on
    /// something invisible is the hole the graph closed from the other
    /// side — Qt keeps active focus there and the keys go on arriving.
    function takeKeyboard() {
        if (walk.view.visible)
            walk.view.forceActiveFocus()
    }
    /// Sends the view `delta` rows (∓1 per press) and answers whether it
    /// moved. The keys and the automation hook both come through here — a
    /// headless run cannot inject a keystroke, so the step has to be
    /// callable as well as pressable (verify-ui).
    ///
    /// The view is what moves, not a selection: nothing in this pane
    /// follows a lit row, and the "selection" it does own is the set of
    /// lines the next write carries, which the arrows must not touch
    /// (デザイン規約 §diff を上下に送る). Answering `false` at either end
    /// is how it stops rather than wraps — the key goes unaccepted there.
    function stepRows(delta) {
        if (!walk.visible || !walk.view.visible || walk.view.count === 0)
            return false
        const was = walk.view.contentY
        walk.view.cancelFlick()
        walk.view.contentY = walk.view.clampY(was + delta * Theme.rowHeight)
        return walk.view.contentY !== was
    }
    /// Whether the view is as far down as it goes — the end the arrows
    /// stop at, which a picture of a diff cannot be told from a short one.
    /// A diff with nothing to scroll reads as at its end, because it is.
    readonly property bool atEnd: walk.view.contentY >= walk.view.maxY - 0.5

    // Taken on the way in, let go on the way out. The second half is the
    // rule the graph is already keeping (規約 §矢印で履歴を辿る「画面から
    // 退いたペインはキーボードを手放す」): a pane swapped off the screen
    // that keeps focus goes on answering arrows nobody can see.
    onVisibleChanged: {
        if (walk.visible)
            walk.takeKeyboard()
        else
            walk.view.focus = false
    }
}
