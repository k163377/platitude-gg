pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// A changed-file row (commit file list): tree view shows indented, collapsible directory rows with leaf names;
// path view shows flat full paths.
Item {
    id: fileRow
    required property var model
    property real listWidth: 200

    readonly property string changeText: model.change ?? ""
    readonly property string nameText: model.name ?? ""
    readonly property string pathText: model.path ?? ""
    readonly property string origPathText: model.orig_path ?? ""
    /// The same source written the way this row writes names — what the row shows. The one above stays whole: that one
    /// addresses a diff.
    readonly property string origNameText: model.orig_name ?? ""
    readonly property bool isFolder: (model.folder ?? false) === true
    /// The path the middle pane is reading, handed down by the pane — one copy there rather than one per row. A folder
    /// is never it: a folder has no diff, and its own path is the fold key.
    property string readPath: ""
    readonly property bool selected: !fileRow.isFolder && fileRow.pathText === fileRow.readPath
    /// What the walk over this list calls this row, empty on a folder — the one name both file lists' rows answer to
    /// (`FileRowWalk`).
    readonly property string walkKey: fileRow.isFolder ? "" : fileRow.pathText
    /// That name while the row is painted as the one being read, empty otherwise — what a headless run reads off the
    /// list. The rectangle's own `visible`, since reading the condition back would go green with the rectangle unwired.
    readonly property string litKey: selectedBox.visible ? fileRow.walkKey : ""

    /// Stands in for the pointer where headless cannot put one, so a cut-down row's tooltip can be photographed
    /// (PG_AUTO_ACT=path-tip). -1 points at no row.
    property int pointedTipRow: -1
    /// **`>= 0` first**: a delegate the view has put back in its reuse pool reports `index` -1, and -1 is also "the
    /// pointer is on no row" — without the guard every pooled row claims the shared tooltip, and the one row actually
    /// pointed at never gets it (the instance is one per window).
    readonly property bool tipPointedAt: fileRow.pointedTipRow >= 0
                                         && fileRow.pointedTipRow === fileRow.model.index
    /// A right-click menu of the page's is standing over this list.
    property bool menuStanding: false

    signal activated(string bucket, string path, string origPath)
    signal folderToggled(string key)

    width: listWidth
    height: Theme.rowHeight

    // The row whose diff is on screen, lit the way every other list lights the row it is standing on (デザイン規約 §色「選択行」).
    // Under the hover wash, which is an overlay colour: the two read together on the row the pointer is already on.
    Rectangle {
        id: selectedBox
        anchors.fill: parent
        color: Theme.bgSelected
        visible: fileRow.selected
    }
    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: fileMouse.containsMouse
    }

    // The mark and the name are the part both file lists say the same way (`NameCell`), down to the arrow a rename puts
    // between its two names. The whole of this row is that part; what a click means is the only thing it keeps to
    // itself.
    NameCell {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceSm + (fileRow.model.depth ?? 0) * Theme.spaceMd
        anchors.rightMargin: Theme.spaceSm
        folder: fileRow.isFolder
        change: fileRow.changeText
        name: fileRow.nameText
        origPath: fileRow.origNameText
    }
    MouseArea {
        id: fileMouse
        anchors.fill: parent
        hoverEnabled: true
        onClicked: {
            if (fileRow.isFolder)
                fileRow.folderToggled(fileRow.pathText)
            else
                fileRow.activated("", fileRow.pathText, fileRow.origPathText)
        }
    }
    // Hover says the path, whatever the row shows and however wide the pane is (デザイン規約 §hover のツールチップ). Asking
    // whether the row had already said it — tree leaf against paths view, and either against what the pane elided —
    // bought a repeat avoided at the price of a condition nobody could read off the screen.
    // Not behind a standing menu: the pointer is in the menu, and a tip that comes out now is drawn over the rows the
    // hand is reading (デザイン規約 §メニュー).
    ToolTip.visible: (fileMouse.containsMouse || fileRow.tipPointedAt) && !fileRow.menuStanding
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: fileRow.pathText
}
