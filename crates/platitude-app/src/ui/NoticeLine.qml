import QtQuick
import platitude.ui

// One line of what a screen with nothing to show says instead — git's words as often as the app's.
//
// **A field** (`CardText`, 規約 §右のペインの字は掴める): on the gate no command log exists yet, so these lines are the
// only place to copy git's words from. Wrapped and centred: it is the whole of an empty screen.
//
// The width is the instance's: a default read off the parent would be a binding loop inside a column sized to its
// content (`CardText` measures `implicitWidth` off a ruler of its own, so a caller may bound itself by it).
CardText {
    horizontalAlignment: Text.AlignHCenter
}
