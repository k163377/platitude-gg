import QtQuick
import platitude.ui

// The one thing a screen with nothing to show offers: open a repository, close the tab that would not open, close the
// window another copy of the app is already holding.
//
// A plain frame: the accent is for the button somebody came to press, and nobody came to these screens
// (規約 §アクセント / §肯定側のボタン). A frame all the same — bare is the shape an answer takes beside a framed one, and there is
// nothing beside this to read it against; under two lines of centred text a button with no edge is a third line (規約
// §枠を持てる場所にだけ枠を出す).
//
// Reachable by Tab: it is the only thing on the screen to press, and on the window's gate the band that would otherwise
// carry the way out is inside the part that stays hidden.
ActionButton {
    implicitHeight: Theme.controlHeight
    frameColor: Theme.borderDefault
    activeFocusOnTab: true
    anchors.horizontalCenter: parent.horizontalCenter
}
