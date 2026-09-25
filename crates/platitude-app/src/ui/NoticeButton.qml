import QtQuick
import platitude.ui

// The one button a screen with nothing to show offers: open a repository, close the tab, close the window.
//
// A plain frame, not the accent (rules-refs/app-ui.md「肯定側のボタンは枠で名乗り」). Framed at all because under two
// lines of centred text a bare button reads as a third line (規約 §枠を持てる場所にだけ枠を出す).
//
// Reachable by Tab: it is the only thing on the screen to press, and on the window's gate the band that would
// otherwise carry the way out stays hidden.
ActionButton {
    implicitHeight: Theme.controlHeight
    frameColor: Theme.borderDefault
    activeFocusOnTab: true
    anchors.horizontalCenter: parent.horizontalCenter
}
