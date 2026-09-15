import QtQuick

/// Hands focus back to the window whenever a press lands outside the focused editor. QML never drops a text input's
/// focus on its own: a clicked filter or commit editor kept its caret until some other editor took focus.
///
/// It must sit *above* every pane: press delivery stops at the first item that accepts, so a handler on the window's
/// content item never hears clicks a row's MouseArea takes (measured: focus survived a graph click). And it must be a
/// PointHandler — the one handler specified to take only passive grabs and accept nothing, so everything below keeps
/// working; a fronted TapHandler swallowed the press and the control underneath never received it (measured: the
/// filter field stopped taking focus at all). Modal dialogs live in the window overlay above this item and are
/// unaffected.
///
/// The top margin reaches up over the safe-area inset the same way the chrome does, so presses on the band are heard
/// too.
Item {
    id: watcher

    // Required: the margin below is read while this is built, and an unset window would fail it once in silence.
    required property var window
    /// Where the keyboard goes when a caret is walked away from. **Not the window's content item**, which is above
    /// every page rather than inside one: a key is delivered to whatever holds the focus and then up its parents only,
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
    anchors.topMargin: -watcher.window.contentItem.y
    z: 10000

    PointHandler {
        acceptedButtons: Qt.AllButtons
        onActiveChanged: {
            if (!active)
                return
            watcher.pressedAnywhere()
            const item = watcher.window.activeFocusItem
            // Only text editors hold a caret worth releasing; list views and buttons manage their own focus.
            if (!item || item.cursorPosition === undefined)
                return
            const local = item.mapFromItem(null, point.scenePressPosition)
            if (local.x < 0 || local.y < 0 || local.x >= item.width || local.y >= item.height) {
                if (watcher.home !== null)
                    watcher.home.forceActiveFocus()
                else
                    watcher.window.contentItem.forceActiveFocus()
                watcher.pressedAway(point.scenePressPosition)
            }
        }
    }
}
