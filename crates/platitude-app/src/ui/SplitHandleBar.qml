import QtQuick
import QtQuick.Controls
import platitude.ui

// The bar between two panes: the line itself, plus word to the page about
// when it has the hand.
//
// SplitView keeps its drag entirely to itself. Measured 2026-08-11 with a
// throwaway qmltestrunner scene: a `MouseArea` filling this delegate never
// sees the press, a `HoverHandler` on it never fires, and a `PointHandler`
// on any item *behind* the SplitView never becomes active — so nothing
// here can tell whether a drag is asking for more than the layout will
// give. The one thing that does work is a passive grab taken in front of
// the whole page (see `RepoPage`'s watcher), and that needs to be told
// which boundary is moving. This signal is that telling.
Rectangle {
    id: bar

    /// Raised when this bar takes the hand or gives it up, and once on
    /// creation so the page can collect the bars it will have to measure
    /// against. Carries itself, because one `handle:` Component builds
    /// every bar of its SplitView and they are otherwise alike.
    signal handChanged(Item which, bool held)

    /// Which way this bar travels. A tall thin bar moves sideways; a wide
    /// flat one moves up and down.
    readonly property bool sideways: bar.height > bar.width

    readonly property bool held: SplitHandle.pressed
    onHeldChanged: bar.handChanged(bar, bar.held)
    Component.onCompleted: bar.handChanged(bar, false)

    implicitWidth: Theme.splitterWidth
    implicitHeight: Theme.splitterWidth
    color: Theme.borderSubtle
}
