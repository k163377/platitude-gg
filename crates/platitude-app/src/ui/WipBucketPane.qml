pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

// One bucket of the working tree as a list of its own: the heading pinned on top and the files scrolling under it,
// in whatever share of the pane the bucket was given. The heading stands when the bucket is empty and does not
// scroll (デザイン規約 §バケツごとの一覧と、ペインの分け方).
ColumnLayout {
    id: bucketPane

    /// Which bucket this is: `conflicts` / `unstaged` / `staged`.
    required property string section
    /// The pane this is one bucket of. The choice, the stage marks and the signals are the pane's: a Ctrl-click or a
    /// walk crosses buckets.
    required property var pane
    required property var repoTab
    required property var workTree
    /// This bucket's rows (`NavSectionModel` attached to this run).
    required property var model
    /// Automation's stand-in pointer row (PGG_AUTO_ACT=path-tip): the pane's row less the rows above this bucket, so
    /// out of range in every other bucket.
    required property int tipRow

    /// Whether this bucket's heading is on screen, read off the heading itself (app-ui.md §UI 自動化).
    readonly property bool headed: bucketPane.visible && bucketHead.visible && bucketHead.height > 0
    /// The list itself — the pane walks the buckets through this (choice, automation, the arrow keys).
    readonly property alias list: bucketList
    /// How many rows this bucket shows, and the room they would take — what the pane shares out by
    /// (`WipPane.roomFor`).
    readonly property int rows: bucketPane.model.shownRows
    readonly property real wants: bucketPane.rows * Theme.rowHeight
    /// The heading's height as the token, not measured: the pane shares room out from it, and measuring an item
    /// sized by that answer would be a binding loop.
    readonly property real headHeight: Theme.rowHeight

    /// Automation: press this bucket's whole-bucket button where a hand presses it (`WipBucketHeader.moveAll`).
    function moveAll() {
        return bucketHead.moveAll()
    }

    spacing: 0

    WipBucketHeader {
        id: bucketHead
        Layout.fillWidth: true
        section: bucketPane.section
        repoTab: bucketPane.repoTab
        workTree: bucketPane.workTree
        bucketModel: bucketPane.model
    }
    AppListView {
        id: bucketList
        Layout.fillWidth: true
        Layout.fillHeight: true
        model: bucketPane.model
        // The pane's bar, not the style's (`AppListView`).
        verticalBar: PaneScrollBar {}
        // Qt's own key navigation moves `currentIndex` and tells nobody; the arrows are answered by the pane's walk,
        // which crosses from this bucket's list into the next one's (規約 §diff のファイル一覧).
        keyNavigationEnabled: false
        Keys.onUpPressed: event => event.accepted = bucketPane.pane.filesWalk.stepFile(-1, event.isAutoRepeat)
        Keys.onDownPressed: event => event.accepted = bucketPane.pane.filesWalk.stepFile(1, event.isAutoRepeat)
        // The menu key; Windows' Shift+F10 comes through the window (`Main.keyMenuAsked`).
        Keys.onMenuPressed: event => event.accepted = bucketPane.pane.menuFromKeys()
        delegate: NavItemDelegate {
            listWidth: bucketList.width
            kindHint: "wt"
            showStage: true
            chosen: bucketPane.pane.isChosen(bucket, fullName)
            sideOurs: bucketPane.workTree.sideOurs
            sideTheirs: bucketPane.workTree.sideTheirs
            pointedTipRow: bucketPane.tipRow
            menuStanding: bucketPane.pane.menuStanding
            pointedEolPath: bucketPane.pane.pointedEolPath
            onEolPointed: (path, on) => bucketPane.pane.pointEol(on ? path : "")
            onFileClicked: (bucket, path, origPath, modifiers) => {
                // Any click puts the keyboard here; only a plain one moves the diff (規約 §diff のファイル一覧).
                bucketList.forceActiveFocus()
                if (bucketPane.pane.applyClick(bucket, path, modifiers))
                    bucketPane.pane.fileActivated(bucket, path, origPath)
            }
            onFileMenuRequested: (bucket, path) => {
                if (!bucketPane.pane.isChosen(bucket, path))
                    bucketPane.pane.chooseOnly(bucket, path)
                bucketPane.pane.fileMenuRequested(bucket, path)
            }
            onFolderClicked: key => bucketPane.model.toggleFolder(key)
            stagePeer: bucketPane.pane.stagePeerOf(bucket, fullName)
            onStageHovered: (bucket, path, on) => {
                if (on)
                    bucketPane.pane.showStageTools(bucket, path)
                else if (bucketPane.pane.stageHotKey === bucket + ":" + path)
                    bucketPane.pane.stageHotKey = ""
            }
            // Every highlighted row that can go the same way, in one git command (デザイン規約 §その他の操作).
            onStageClicked: (bucket, path) => bucketPane.pane.moveStage(bucket, path)
        }
    }
}
