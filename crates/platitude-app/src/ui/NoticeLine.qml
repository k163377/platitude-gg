import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One line of what a screen with nothing to show says instead: the window waiting on git, a tab whose repository would
// not open, the graph with no history behind it.
//
// Wrapped, never elided: such a line is the whole of what is on that screen, so there is nothing beside it to carry
// what a cut would drop. Centred over its own width for the same reason — it is the middle of an empty screen rather
// than a row in a column of others.
//
// The width is the instance's: the container is not always the bound (the graph's own pane holds the history's width,
// not the sentence's), and a default read off the parent would be a binding loop inside a column sized to its content.
Label {
    wrapMode: Text.Wrap
    horizontalAlignment: Text.AlignHCenter
}
