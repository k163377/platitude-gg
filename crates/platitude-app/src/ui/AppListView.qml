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
    id: appList

    /// Which bar this list wears, named as the component to build it from. **Handed in, never fitted over the top.**
    /// `ScrollBar.vertical` does not replace the bar a view already had — it only unhooks it — so every list that
    /// wrote the attached property a second time left the first bar alive with nothing to answer for: ten of them on
    /// every start, one per list wanting the panel's slab (verify-ui 実測 2026-08-28, `Component.onCompleted` reading
    /// `view= null parent= null`). Naming the component here is what keeps the count at one. The lists that want the
    /// slab pass `PaneScrollBar` in (`NavList`, `DetailsPane`, `WipBucketPane`); the graph, the diff and the log take
    /// the default (rules-refs/app-ui.md のペインのバーの行).
    property Component verticalBar: styleBar
    Component {
        id: styleBar
        AutoScrollBar {}
    }

    clip: true
    reuseItems: true
    // Hard stop at the ends, as everywhere else that scrolls (デザイン規約 §QML 実装ルール).
    boundsBehavior: Flickable.StopAtBounds
    highlightFollowsCurrentItem: false
    // The one bar, built here from whatever the list named above.
    ScrollBar.vertical: appList.verticalBar.createObject(appList)
}
