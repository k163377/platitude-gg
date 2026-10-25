import QtQuick
import platitude.ui

// The ground git's spelling stands on — a command's, a name's (デザイン規約 §git 用語のコード表記): faint, rounded, as tall
// as the word's step and not its line box, which the code face makes different on every OS. Where it starts and ends
// is the caller's: half a gap off a word's ends, or from the head of a name's mark's seat to half a gap past the name's
// ink (`WordFace.groundStart` / `groundEnd`).
Rectangle {
    id: ground

    /// The pixel size of the word it is under.
    property real size: Theme.fontMd

    height: ground.size + Theme.spaceXs / 2
    radius: Theme.radiusSm
    color: Theme.bgHover
}
