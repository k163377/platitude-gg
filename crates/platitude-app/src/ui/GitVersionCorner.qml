import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

// Which git is doing all this. It sits in the right pane's corner so the command log
// can open without landing on top of it. A build made in a worktree adds which one — parallel sessions' windows are
// otherwise identical (デザイン規約 §アプリ名).
//
// **Both are fields** (`LineText`, 規約 §右のペインの字は掴める). The version is what a reader copies into a bug
// report and the letter is which tree a window was built from, and neither is written anywhere else in this window.
// They stand on the pane's own empty room — this corner is hidden the moment the list reaches it — so nothing else
// wants the press.
RowLayout {
    id: gitCorner

    /// What the pane under it is leaving bare. Only one pane is on screen at a time and each measures its own file
    /// list, so which of them is answering is the page's to say.
    required property real roomLeft
    /// Whether whoever hangs this corner is offering the seat at all — the run button takes the pane's foot while a
    /// plan stands (規約 §コミットメッセージの 2 つの枠「ペインの底は空かない」).
    ///
    /// **It comes in as a property**: a `visible` written on the instance replaces
    /// this one outright, and the room test goes with it — the version was drawn across the commit button of a pane
    /// that lends no corner at all (observed).
    property bool offered: true
    /// Its own box and the one step: a list's rows are already spaced by their own height.
    readonly property real roomNeeded: gitCorner.implicitHeight + Theme.spaceXs

    spacing: Theme.spaceXs
    // Out of the way as soon as the list reaches this corner. Hidden outright: a Label keeps its implicit height
    // while `visible: false`, so the answer never eats what it read (qmltestrunner, measured).
    visible: gitCorner.offered && AppBackend.gitVersion !== ""
             && gitCorner.roomLeft >= gitCorner.roomNeeded

    LineText {
        text: qsTr("git %1").arg(AppBackend.gitVersion)
        color: Theme.textMuted
        pixelSize: Theme.fontSm
    }
    // Drawn: as a glyph the spacing here was a full-width cell's leftover (規約 §余白).
    DotMark {
        visible: AppBackend.buildTree !== ""
        Layout.alignment: Qt.AlignVCenter
    }
    LineText {
        visible: AppBackend.buildTree !== ""
        text: AppBackend.buildTree
        color: Theme.textMuted
        pixelSize: Theme.fontSm
    }
}
