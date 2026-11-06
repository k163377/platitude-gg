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
    /// The hand over the pane these rows move under (`HandWatch`), which the pane tells as they move — what the tips of
    /// the rows read to tell a hand that moved from rows that moved under it (`TipAsk.hand`, found through this). Null
    /// for a list whose pane keeps none.
    property HandWatch rowsHand: null
    /// For a list that takes the keyboard for itself: gives it up on leaving the screen (デザイン規約 §矢印で履歴を辿る
    /// 「画面から退いたペインはキーボードを手放す」). Qt keeps active focus on an item it made invisible and keys keep
    /// arriving; only `focus = false` lets go. Off where a walk lets go for its whole pane (`FileRowWalk`, `DiffRowWalk`).
    property bool letsGoOffScreen: false
    onVisibleChanged: {
        if (appList.letsGoOffScreen && !appList.visible)
            appList.focus = false
    }
    /// What stands over these rows and is no row: a stand-in on an edge of the list (`HeadPinRow`, `GraphHeadPin`'s
    /// press area, `RecoverPane.pickPin`). A row under it is not under the hand — rows read the hand by geometry, and
    /// the stand-in's own hand is the same pane's.
    property Item shield: null
    function shieldAt(scenePoint) {
        const over = appList.shield
        if (over === null || !over.visible)
            return false
        const p = over.mapFromItem(null, scenePoint.x, scenePoint.y)
        return p.x >= 0 && p.y >= 0 && p.x < over.width && p.y < over.height
    }
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
