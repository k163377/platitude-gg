import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Which git is doing all this. It sits in the right pane's corner rather than at the window's edge so the command log
// can open without landing on top of it. A build made in a worktree adds which one — parallel sessions' windows are
// otherwise identical (デザイン規約 §アプリ名).
RowLayout {
    id: gitCorner

    /// What the pane under it is leaving bare. Only one pane is on screen at a time and each measures its own file
    /// list, so which of them is answering is the page's to say.
    required property real roomLeft
    /// Its own box and nothing more — no extra air above: a list's rows are already spaced by their own height.
    readonly property real roomNeeded: gitCorner.implicitHeight + Theme.spaceXs

    spacing: Theme.spaceXs
    // Out of the way as soon as the list reaches this corner. Hidden outright rather than held at
    // zero opacity: a Label keeps its implicit height while `visible: false`, so the answer never eats what it read
    // (qmltestrunner, measured).
    visible: AppBackend.gitVersion !== "" && gitCorner.roomLeft >= gitCorner.roomNeeded

    Label {
        text: qsTr("git %1").arg(AppBackend.gitVersion)
        color: Theme.textMuted
        font.pixelSize: Theme.fontSm
    }
    // Drawn rather than typed: as a glyph the spacing here was a full-width cell's leftover (規約 §余白).
    DotMark {
        visible: AppBackend.buildTree !== ""
        Layout.alignment: Qt.AlignVCenter
    }
    Label {
        visible: AppBackend.buildTree !== ""
        text: AppBackend.buildTree
        color: Theme.textMuted
        font.pixelSize: Theme.fontSm
    }
}
