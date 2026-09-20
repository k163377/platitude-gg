import QtQuick

/// Hands focus back to the window whenever a press lands outside the focused editor. QML never drops a text input's
/// focus on its own: a clicked filter or commit editor kept its caret until some other editor took focus.
///
/// On the window it must sit *above* every pane: press delivery stops at the first item that accepts, so a handler on
/// the window's content item never hears clicks a row's MouseArea takes (measured: focus survived a graph click). And
/// it must be a PointHandler — the one handler specified to take only passive grabs and accept nothing, so everything
/// below keeps working; a fronted TapHandler swallowed the press and the control underneath never received it
/// (measured: the filter field stopped taking focus at all).
///
/// **A modal dialog needs one of its own.** It stands in the window overlay, above the window's watcher, so nothing
/// up there ever reaches this — and a settings screen is a surface whose words are read (規約 §設定の画面), so a
/// caret left in one of them stayed lit through every press that followed. **Inside a dialog it stands on the face,
/// behind everything drawn**: what accepts a press there either takes the focus itself or is a reading surface's own
/// hand, so the only presses that reach this are the ones that would otherwise leave a caret standing — a row that
/// answers with a handler of its own, and the plain air (measured, `tests/qml/tst_fieldrelease.qml`).
Item {
    id: watcher

    // Required: what holds the caret is read off it, and a watcher given none would answer every press by dying in
    // a handler nobody is watching.
    required property var window
    /// How far up this reaches past its own parent. **The window's watcher reaches over the safe-area inset the
    /// chrome also covers**, so presses on the band are heard too; one standing inside a dialog has no such inset and
    /// covers exactly what it is laid on.
    property real reachUp: 0
    /// Where the keyboard goes when a caret is walked away from. **The page**, since the content item is above
    /// every page: a key is delivered to whatever holds the focus and then up its parents only,
    /// so focus resting there leaves the page's own Escape handler off the chain and the one gesture that puts a
    /// standing thing away stops working until the next press that takes the keyboard back
    /// (`RepoPage.escapePressed`, `tests/qml/tst_escape.qml`). Null falls back to the content item — a window with no
    /// page open has no handler to keep on the chain.
    property var home: null

    /// The press that took the caret away landed somewhere else, at `scenePos` in scene coordinates. Whatever was
    /// standing open on the strength of holding the keyboard is walked away from here — the left menu's in-place name
    /// box, which nothing else closes (デザイン規約 §左メニューの所作「Escape と他所のクリックで取り消す」), and the graph's own empty
    /// boxes (§コミットを探す). **Where** the press landed comes with it because "somewhere else" is not the same question
    /// for every one of them: a press on the find card's own `✕` is a press on that card.
    signal pressedAway(var scenePos)

    /// A press landed, wherever it went and whatever it was on. What the window put on screen to say "here is what you
    /// asked for" is taken back by the first thing the reader does next — a mark that has been acted through has
    /// already said what it had to say (デザイン規約 §hover のツールチップ). Presses inside a popup do not come here:
    /// the overlay stands above this item, which is the right answer for a card the reader is still reading.
    signal pressedAnywhere()

    anchors.fill: parent
    anchors.topMargin: -watcher.reachUp
    z: 10000

    /// What a press does, whatever it landed on. **Named, and the handler below is one line onto it**, so a run
    /// enters the same road a press does (規約 §UI 自動化の因果性) — a harness has no pointer to put anywhere.
    function pressedAt(scenePos) {
        watcher.pressedAnywhere()
        const item = watcher.window.activeFocusItem
        // Only text editors hold a caret worth releasing; list views and buttons manage their own focus.
        if (!item || item.cursorPosition === undefined)
            return
        const local = item.mapFromItem(null, scenePos)
        if (local.x < 0 || local.y < 0 || local.x >= item.width || local.y >= item.height) {
            if (watcher.home !== null)
                watcher.home.forceActiveFocus()
            else
                watcher.window.contentItem.forceActiveFocus()
            watcher.pressedAway(scenePos)
        }
    }
    /// And that there is a hand at all, covering what it was laid on. The half a run cannot say for itself: it
    /// enters the function above, so a watcher whose handler had been taken out or shrunk would answer every press
    /// it was asked and never see one (the same thing `SweepPad.handStands` is for).
    readonly property bool stands: hand.enabled && watcher.width > 0 && watcher.height > 0

    PointHandler {
        id: hand
        acceptedButtons: Qt.AllButtons
        onActiveChanged: if (active) watcher.pressedAt(point.scenePressPosition)
    }
}
