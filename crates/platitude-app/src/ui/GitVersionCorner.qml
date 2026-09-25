import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

// The git version, and in a worktree build which tree built it (デザイン規約 §アプリ名), in the right pane's corner.
// Both are fields (規約 §右のペインの字は掴める).
RowLayout {
    id: gitCorner

    /// The height the pane under it leaves bare; the page picks which pane answers.
    required property real roomLeft
    /// Whether the seat is offered at all (規約 §コミットメッセージの 2 つの枠「ペインの底は常に埋まる」). A property, not
    /// `visible` on the instance — that would replace the binding below, room test and all.
    property bool offered: true
    /// One step, not two: a list's rows already carry their own spacing.
    readonly property real roomNeeded: gitCorner.implicitHeight + Theme.spaceXs

    spacing: Theme.spaceXs
    // Hidden once the list reaches the corner. Hiding keeps the implicit height, so `roomNeeded` cannot oscillate.
    visible: gitCorner.offered && AppBackend.gitVersion !== ""
             && gitCorner.roomLeft >= gitCorner.roomNeeded

    LineText {
        text: qsTr("git %1").arg(AppBackend.gitVersion)
        color: Theme.textMuted
        pixelSize: Theme.fontSm
    }
    // Drawn, not a glyph: a glyph's spacing is a full-width cell's leftover (規約 §余白).
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
