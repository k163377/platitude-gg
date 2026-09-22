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
    /// The whole path — what a diff is asked for by, what the hover says, and the name the walk over this list finds a
    /// row by. **Asked as its own question**, the way `NameCell.folded` is and for the same reason: the two models
    /// that hang these rows do not spell it alike. A commit's changed files spell it `path`
    /// (`models::details::FileItem`), which is the answer below; a working copy's spell it `full`
    /// (`models::nav::Role`), so the pane showing one says so.
    ///
    /// **A role this model does not answer reads back `undefined`** — silently, leaving every row holding the same
    /// empty string. Two of those match, so the whole list lights up the moment
    /// nothing is being read, and none of it lights while something is (observed on the carried pane).
    property string pathText: model.path ?? ""
    /// Which side of the index this row's bytes are on, empty for a commit's changed files — those sit in no bucket.
    /// A working copy's rows carry one (`Bucket::routing_of`), and it is half of what addresses the file: the page
    /// opens the diff by the pair.
    property string bucket: ""
    /// What a folder row folds by. **A key of its own**: a commit's changed files keep one field for both
    /// (`models::details::FileItem.path`), which is the answer here; a working copy's model folds by `<run>:<path>`
    /// and keeps the path itself beside it, so a pane on that one says which is which. A file row never folds.
    property string foldKey: fileRow.pathText
    readonly property string origPathText: model.orig_path ?? ""
    /// The same source written the way this row writes names — what the row shows. The one above stays whole: that one
    /// addresses a diff.
    readonly property string origNameText: model.orig_name ?? ""
    readonly property bool isFolder: (model.folder ?? false) === true
    /// A shut folder row. The commit's list keeps the answer in a field of its own, which is the answer below; the
    /// sidebar's model packs it into the change code, so a pane on that one says so (`NameCell.folded`, which is
    /// asked the same way and for the same reason).
    property bool isFolded: (model.collapsed ?? false) === true
    /// The path the middle pane is reading, handed down by the pane — one copy for the whole list. A folder
    /// is never it: a folder has no diff, and its own path is the fold key.
    property string readPath: ""
    readonly property bool selected: !fileRow.isFolder && fileRow.pathText === fileRow.readPath
    /// What the walk over this list calls this row, empty on a folder — the one name both file lists' rows answer to
    /// (`FileRowWalk`).
    readonly property string walkKey: fileRow.isFolder ? "" : fileRow.pathText
    /// That name while the row is painted as the one being read, empty otherwise — what a headless run reads off the
    /// list. The rectangle's own `visible`, since reading the condition back would go green with the rectangle unwired.
    readonly property string litKey: selectedBox.visible ? fileRow.walkKey : ""
    /// The same rectangle as a bare answer. **Counted on this one** — the name above carries the row's own path,
    /// which is the very thing a list lighting every row it has failed to read, so a count off it comes to none
    /// exactly where it is worth taking (`FileRowWalk.litRows`).
    readonly property bool litNow: selectedBox.visible
    /// The turn this row's fold arrow is drawn at, -1 on a file row — what a headless run reads, since the flag
    /// behind it would go green with the arrow unwired (`NameCell.foldTurn`).
    readonly property real foldTurn: nameCell.foldTurn

    /// Stands in for the pointer where headless cannot put one, so a cut-down row's tooltip can be photographed
    /// (PGG_AUTO_ACT=path-tip). -1 points at no row.
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

    /// What a press on this row does — a folder folds, a file is read. **The handler below is one line and this is the
    /// whole of its body**, because this is also the door a headless run comes in by: a run handed the path by the
    /// model beside the row would go green with the row itself reading nothing at all (verify-ui §壊れない動詞).
    function press() {
        if (fileRow.isFolder)
            fileRow.folderToggled(fileRow.foldKey)
        else
            fileRow.activated(fileRow.bucket, fileRow.pathText, fileRow.origPathText)
    }

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
        id: nameCell
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceXs + (fileRow.model.depth ?? 0) * Theme.spaceMd
        // The gutter the list's own scroll bar is drawn in. This list is the details pane's, so the bar
        // is the pane's own slab — 5px of ink against the edge and 3 of ground behind it — and anything short of this
        // stands the last glyph of an elided name under it (デザイン規約 §余白).
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
        // The row keeps the drag it is handed, as every row in this window does (`GraphRowDelegate`, measured in
        // `tst_pressorder`): the list takes the grab at the platform's drag distance otherwise, and a click with a
        // tremor in it opens nothing while the files slide under the hand that made it.
        preventStealing: true
        onClicked: fileRow.press()
    }
    // Hover says the path, whatever the row shows and however wide the pane is (デザイン規約 §hover のツールチップ). Asking
    // whether the row had already said it — tree leaf against paths view, and either against what the pane elided —
    // bought a repeat avoided at the price of a condition nobody could read off the screen.
    // Gone behind a standing menu: the pointer is in the menu, and a tip coming out now is drawn over the rows the
    // hand is reading (デザイン規約 §メニュー).
    ToolTip.visible: (fileMouse.containsMouse || fileRow.tipPointedAt) && !fileRow.menuStanding
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: fileRow.pathText
}
