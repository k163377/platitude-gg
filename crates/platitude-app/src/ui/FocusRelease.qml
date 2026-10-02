import QtQuick

/// Hands focus back to the window whenever a press lands outside the focused editor — QML never drops a text input's
/// focus on its own.
///
/// Topmost, and a `PointHandler` (rules-refs/app-ui.md「画面全体の入力観測は最前面オーバーレイ + `PointHandler`」): lower
/// down it misses presses a row's MouseArea takes, and a TapHandler would swallow the press.
///
/// A modal dialog needs one of its own, on its face behind everything drawn — the window's cannot reach the overlay
/// (rules-refs/app-ui.md「モーダルには窓の `FocusRelease` が届かない」, `tests/qml/tst_fieldrelease.qml`).
Item {
    id: watcher

    required property var window
    /// How far up this reaches past its parent: the window's watcher also covers the safe-area inset; 0 in a dialog.
    property real reachUp: 0
    /// Where the keyboard goes when a caret is walked away from — the page, so its Escape handler stays on the key's
    /// way up (keys climb parents only; `tests/qml/tst_escape.qml`). Null falls back to the content item.
    property var home: null

    /// The press that took the caret away, at `scenePos` (scene coordinates). What stands open only while holding the
    /// keyboard closes on it — the left menu's name box, which nothing else closes, and the graph's empty boxes. The
    /// position comes along because each box answers "elsewhere" for itself: the find card's `✕` is on the card.
    signal pressedAway(var scenePos)

    /// Any press anywhere; marks that say "here is what you asked for" clear on it (デザイン規約 §hover のツールチップ).
    /// Presses inside a popup never come here — the overlay stands above this.
    signal pressedAnywhere()

    anchors.fill: parent
    anchors.topMargin: -watcher.reachUp
    z: 10000

    /// What a press does. The handler below is one line onto it, so a headless run enters the same road.
    function pressedAt(scenePos) {
        watcher.pressedAnywhere()
        const item = watcher.window.activeFocusItem
        // Only text editors hold a caret worth releasing; list views and buttons manage their own focus.
        if (!item || item.cursorPosition === undefined)
            return
        const local = item.mapFromItem(null, scenePos)
        if (local.x < 0 || local.y < 0 || local.x >= item.width || local.y >= item.height) {
            watcher.letGo(item, watcher.home !== null ? watcher.home : watcher.window.contentItem)
            watcher.pressedAway(scenePos)
        }
    }
    /// The keyboard from `box` to `to`. A focus scope handed the focus passes it straight back down to the child that
    /// last held it — the page is one, and so is a list or a scroll view between — so while `to` stands above the box,
    /// every link up to it lets go first, or the caret comes back to the box it was walked away from
    /// (`tests/qml/tst_pagerelease.qml`).
    function letGo(box, to) {
        const links = []
        for (let link = box; link !== null; link = link.parent) {
            if (link === to) {
                for (const held of links)
                    held.focus = false
                break
            }
            links.push(link)
        }
        to.forceActiveFocus()
    }
    /// The hand is live and covers what it was laid on — the half a run entering `pressedAt` cannot prove.
    readonly property bool stands: hand.enabled && watcher.width > 0 && watcher.height > 0

    PointHandler {
        id: hand
        acceptedButtons: Qt.AllButtons
        onActiveChanged: if (active) watcher.pressedAt(point.scenePressPosition)
    }
}
