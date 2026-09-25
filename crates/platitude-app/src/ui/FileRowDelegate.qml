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
    /// The whole path — what a diff is asked for by, what the hover says, what the walk finds a row by. **Its own
    /// property**, as `NameCell.folded` is: a commit's changed files spell it `path` (`models::details::FileItem`,
    /// the answer below), a working copy's `full` (`models::nav::Role`), so that pane says so. A role the model does
    /// not answer reads back `undefined` silently — every row then holds the same empty string and lights together.
    property string pathText: model.path ?? ""
    /// Which side of the index this row's bytes are on, empty for a commit's changed files. Half of what addresses
    /// the file (`Bucket::routing_of`): the page opens the diff by the pair.
    property string bucket: ""
    /// What a folder row folds by — its own key: a commit's files keep one field for both (the answer here), a
    /// working copy's model folds by `<run>:<path>`, so that pane says which is which.
    property string foldKey: fileRow.pathText
    readonly property string origPathText: model.orig_path ?? ""
    /// The source written the way this row writes names — what the row shows; `origPathText` addresses the diff.
    readonly property string origNameText: model.orig_name ?? ""
    readonly property bool isFolder: (model.folder ?? false) === true
    /// A shut folder row — a field of its own here; the sidebar's model packs it into the change code, so that pane
    /// says so (`NameCell.folded`).
    property bool isFolded: (model.collapsed ?? false) === true
    /// The path the middle pane is reading, handed down by the pane. Never a folder: a folder has no diff.
    property string readPath: ""
    readonly property bool selected: !fileRow.isFolder && fileRow.pathText === fileRow.readPath
    /// What the walk over this list calls this row, empty on a folder (`FileRowWalk`).
    readonly property string walkKey: fileRow.isFolder ? "" : fileRow.pathText
    /// Automation: that name while the row is painted as read, off the rectangle's own `visible` — reading the
    /// condition back would go green with the rectangle unwired.
    readonly property string litKey: selectedBox.visible ? fileRow.walkKey : ""
    /// Automation: the same rectangle as a bare answer, for counting (`FileRowWalk.litRows`).
    readonly property bool litNow: selectedBox.visible
    /// Automation: the turn this row's fold arrow is drawn at, -1 on a file row (`NameCell.foldTurn`).
    readonly property real foldTurn: nameCell.foldTurn

    /// Stands in for the pointer where headless cannot put one (PGG_AUTO_ACT=path-tip). -1 points at no row.
    property int pointedTipRow: -1
    /// **`>= 0` first**: a pooled delegate reports `index` -1 (rules-refs/app-ui.md「プールへ戻された delegate は」).
    readonly property bool tipPointedAt: fileRow.pointedTipRow >= 0
                                         && fileRow.pointedTipRow === fileRow.model.index
    /// A right-click menu of the page's is standing over this list.
    property bool menuStanding: false

    signal activated(string bucket, string path, string origPath)
    signal folderToggled(string key)

    /// A press on this row — a folder folds, a file is read. The handler below and a headless run both enter here
    /// (verify-ui §壊れない動詞の実装と反復).
    function press() {
        if (fileRow.isFolder)
            fileRow.folderToggled(fileRow.foldKey)
        else
            fileRow.activated(fileRow.bucket, fileRow.pathText, fileRow.origPathText)
    }

    width: listWidth
    height: Theme.rowHeight

    // The row whose diff is on screen (デザイン規約 §diff のファイル一覧), under the hover wash, which is an overlay colour.
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

    NameCell {
        id: nameCell
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceXs + (fileRow.model.depth ?? 0) * Theme.spaceMd
        // The pane's scroll bar gutter — anything short stands an elided name's last glyph under the bar
        // (デザイン規約 §余白).
        anchors.rightMargin: Theme.navBarGutter
        folder: fileRow.isFolder
        folded: fileRow.isFolded
        change: fileRow.changeText
        name: fileRow.nameText
        origPath: fileRow.origNameText
    }
    MouseArea {
        id: fileMouse
        anchors.fill: parent
        hoverEnabled: true
        // Or a click with a tremor hands the grab to the list (rules-refs/app-ui.md「行は渡されたドラッグを手放さない」).
        preventStealing: true
        onClicked: fileRow.press()
    }
    // Hover always says the path (デザイン規約 §hover のツールチップ), except behind a standing menu (デザイン規約 §メニュー).
    ToolTip.visible: (fileMouse.containsMouse || fileRow.tipPointedAt) && !fileRow.menuStanding
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: fileRow.pathText
}
