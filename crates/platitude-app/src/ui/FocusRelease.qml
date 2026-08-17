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

    anchors.fill: parent
    anchors.topMargin: -watcher.window.contentItem.y
    z: 10000

    PointHandler {
        acceptedButtons: Qt.AllButtons
        onActiveChanged: {
            if (!active)
                return
            const item = watcher.window.activeFocusItem
            // Only text editors hold a caret worth releasing; list views and buttons manage their own focus.
            if (!item || item.cursorPosition === undefined)
                return
            const local = item.mapFromItem(null, point.scenePressPosition)
            if (local.x < 0 || local.y < 0 || local.x >= item.width || local.y >= item.height)
                watcher.window.contentItem.forceActiveFocus()
        }
    }
}
