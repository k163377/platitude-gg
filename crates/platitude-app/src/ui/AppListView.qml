import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The list every pane in the app scrolls: rows kept inside their box, delegates recycled, a hard stop at each end, and
// the bar that only appears when there is somewhere to go.
//
// **It does not chase its current item.** Left on, a background rebuild that re-resolves the selection onto a row one
// further down drags a reader parked in the history to wherever the selection went. Whatever moves the view moves it on
// purpose — an arrow key, a reveal, a restored reading position — and by as little as will do.
//
// The two lists that are not this one say why: the combo card's rows do chase, because there the current item is the
// keyboard's highlight and following it is the whole point, and the tab strip keeps every delegate alive for the
// automation to walk.
ListView {
    clip: true
    reuseItems: true
    // Hard stop at the ends, as everywhere else that scrolls (デザイン規約 §QML 実装ルール).
    boundsBehavior: Flickable.StopAtBounds
    highlightFollowsCurrentItem: false
    ScrollBar.vertical: AutoScrollBar {}
}
