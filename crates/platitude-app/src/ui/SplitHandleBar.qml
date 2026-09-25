import QtQuick
import QtQuick.Controls
import platitude.ui

// The bar between two panes: the line itself, plus word to the page about when it has the hand. Nothing here can see
// the drag (rules-refs/app-ui.md「`SplitView` のドラッグは QML から一切覗けない」); the front-most watcher
// (`SplitBarWatch`) can, and has to be told which boundary is moving.
Rectangle {
    id: bar

    /// Raised when this bar takes the hand or gives it up, and once on creation so the page can collect its bars.
    /// Carries itself: one `handle:` Component builds every bar of its SplitView.
    signal handChanged(Item which, bool held)

    readonly property bool sideways: bar.height > bar.width

    readonly property bool held: SplitHandle.pressed
    onHeldChanged: bar.handChanged(bar, bar.held)
    Component.onCompleted: bar.handChanged(bar, false)

    implicitWidth: Theme.splitterWidth
    implicitHeight: Theme.splitterWidth
    color: Theme.borderSubtle
    // Under the panes: SplitView parents a handle to itself and the panes to its contentItem, so otherwise a pane
    // drawing past its edge (the left menu's `NavNameBox`) is cut by this bar whatever its `z`. The hand still finds
    // the uncovered strip.
    z: -1
}
