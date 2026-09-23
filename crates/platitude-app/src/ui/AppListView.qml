import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The list every pane in the app scrolls: rows kept inside their box, delegates recycled, a hard stop at each end, the
// bar that only appears when there is somewhere to go, and the middle button's hand (デザイン規約 §中クリックの自動スクロール).
//
// **The view stays put under a moving current item.** Chasing it drags a reader parked in the history to wherever a
// background rebuild re-resolved the selection. Whatever moves the view moves it on purpose — an arrow key, a reveal,
// a restored reading position — and by as little as will do.
//
// The other two lists say why: the combo card's rows do chase, because there the current item is the keyboard's
// highlight and following it is the whole point, and the tab strip keeps every delegate alive for the automation to
// walk.
ListView {
    id: appList

    /// Which bar this list wears, named as the component to build it from. **Handed in, built once.**
    /// `ScrollBar.vertical` only unhooks the bar a view already had, so every list that wrote the attached property
    /// a second time left the first bar alive with nothing to answer for: ten of them on every start, one per list
    /// wanting the panel's slab (verify-ui measured, `Component.onCompleted` reading `view= null parent= null`).
    /// Naming the component here is what keeps the count at one. The lists that want the slab pass `PaneScrollBar`
    /// in (`NavList`, `DetailsPane`, `WipBucketPane`); the graph, the diff and the log take the default
    /// (rules-refs/app-ui.md のペインのバーの行).
    property Component verticalBar: styleBar
    Component {
        id: styleBar
        AutoScrollBar {}
    }

    /// How much of this list's right edge its own bar is standing on, and nothing while the bar is hidden.
    /// **Whatever is laid over these rows takes its presses short of this.** The bar is drawn over the rows,
    /// so anything covering the frame covers the bar too, and a press taken there is a bar that
    /// cannot be grabbed at all: the graph's stand-in answered the trough with a jump to HEAD,
    /// and the two hands that pick text out of a list took the whole strip. Read the width off
    /// the bar — it is the style's, and a guess leaves either a strip of trough taken or a
    /// strip of nothing answered.
    readonly property real barRoom: appList.ScrollBar.vertical.visible ? appList.ScrollBar.vertical.width : 0

    /// Whether this list is sent by its own hand under the middle button. **Off where the pane lays one over it**:
    /// the graph and the diff go sideways as well, and the log is drawn over by the hand that picks its text — so each
    /// of the three seats a hand of its own above what it laid over the rows (`GraphPane` / `DiffCodeScroll` /
    /// `CommandsPane`), where one in here would sit under all of it.
    property bool ownsHand: true
    /// Automation only: this list's own hand, started and drifted without a pointer — a middle button cannot be
    /// injected any more than a hover can (verify-ui).
    readonly property alias hand: hand

    /// Where the view may be sent to: every hand that moves it by a distance goes through this one clamp. **Not
    /// `[0, contentHeight - height]`**: after `positionViewAtIndex` over rows of differing heights the list moves its
    /// own origin as it fixes the items up, and the top rows go out of reach while the bottom overshoots the last
    /// one. The margins are the list's own ground above and below the rows, so they are part of the range — left out,
    /// the gap at the top cannot be reached again once the view has left it.
    function clampY(y) {
        const minY = appList.originY - appList.topMargin
        const maxY = Math.max(minY, appList.originY + appList.contentHeight - appList.height + appList.bottomMargin)
        return Math.max(minY, Math.min(y, maxY))
    }

    clip: true
    reuseItems: true
    // Hard stop at the ends, as everywhere else that scrolls (デザイン規約 §QML 実装ルール).
    boundsBehavior: Flickable.StopAtBounds
    highlightFollowsCurrentItem: false
    // The one bar, built here from whatever the list named above.
    ScrollBar.vertical: appList.verticalBar.createObject(appList)

    // The middle button's hand, over the rows, **standing while the list has somewhere to go** — the bar's own answer,
    // a browser's too: a middle press on a list that holds all of its rows starts nothing.
    MiddleAutoScroll {
        id: hand
        // The list's own frame, not its content: a child the list adopts goes into `contentItem` and travels with the
        // rows (`CommandsPane` pins its empty label the same way).
        parent: appList
        anchors.fill: parent
        visible: appList.ownsHand && appList.ScrollBar.vertical.visible
        onDrifted: dy => appList.contentY = appList.clampY(appList.contentY + dy)
    }
}
