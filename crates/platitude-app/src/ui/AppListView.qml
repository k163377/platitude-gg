import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The list every pane scrolls: clipped, recycled, hard stops, a bar only when there is somewhere to go, and the middle
// button's hand (デザイン規約 §中クリックの自動スクロール). The view does not chase the current item
// (rules-refs/app-ui.md「`ListView.highlightFollowsCurrentItem` は false にする」). Not for the combo card, whose rows
// chase the keyboard, or the tab strip, which keeps every delegate alive for automation.
ListView {
    id: appList

    /// The bar, as a component built once here: writing `ScrollBar.vertical` a second time only unhooks the first bar
    /// and leaves it alive (rules-refs/app-ui.md「左右のパネルのバーは `PaneScrollBar`」).
    property Component verticalBar: styleBar
    Component {
        id: styleBar
        AutoScrollBar {}
    }

    /// The strip of the right edge the bar stands on (0 while hidden). Whatever is laid over the rows takes its presses
    /// short of this, or the bar cannot be grabbed (rules-refs/app-ui.md「一覧の上に面を重ねたら」).
    readonly property real barRoom: appList.ScrollBar.vertical.visible ? appList.ScrollBar.vertical.width : 0

    /// Off where the pane seats its own middle-button hand above what it lays over the rows (`GraphPane` /
    /// `DiffCodeScroll` / `CommandsPane`); one in here would sit under all of it.
    property bool ownsHand: true
    /// Automation only: started and drifted without a pointer (a middle button cannot be injected).
    readonly property alias hand: hand

    /// The one clamp for every hand that moves the view by a distance: from `originY` with both margins, not
    /// `[0, contentHeight - height]` (rules-refs/app-ui.md「`contentY` を手で書く所は `originY` と両端の margin で丸める」).
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
    ScrollBar.vertical: appList.verticalBar.createObject(appList)

    // Standing only while the list has somewhere to go, as the bar does.
    MiddleAutoScroll {
        id: hand
        // The list's frame: a child the list adopts goes into `contentItem` and scrolls with the rows.
        parent: appList
        anchors.fill: parent
        visible: appList.ownsHand && appList.ScrollBar.vertical.visible
        onDrifted: dy => appList.contentY = appList.clampY(appList.contentY + dy)
    }
}
