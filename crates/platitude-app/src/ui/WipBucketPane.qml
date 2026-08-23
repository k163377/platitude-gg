pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

// One bucket of the working tree, as a list of its own: the heading pinned to the top and the files under it,
// scrolling inside whatever share of the pane the bucket was given (デザイン規約 §その他の操作).
//
// **The heading stands whether or not the bucket has rows**, because it is part of the frame rather than something the
// rows put there: an emptied bucket goes on saying where its files went and where the next one will land, and nothing
// grows in under the hand at the next `+`. That is also why it does not scroll away with the files — `Stage all` is
// about the whole bucket, and a bucket you have scrolled into is exactly when you want it.
ColumnLayout {
    id: bucketPane

    /// Which bucket this is: `conflicts` / `unstaged` / `staged`.
    required property string section
    /// The pane this is one bucket of — where the choice, the stage marks and the signals live, all of which are the
    /// whole pane's rather than one bucket's (a Ctrl-click reaches across the buckets, and so does a walk).
    required property var pane
    required property var repoTab
    required property var workTree
    /// This bucket's rows (`NavSectionModel` attached to this run).
    required property var model
    /// Stands in for the pointer on a row of this bucket, so a cut-down paths-view row's tooltip can be photographed
    /// (PG_AUTO_ACT=path-tip). The pane hands down its own row number less the rows above this bucket, so a number
    /// below 0 or past the end points at no row — which is every bucket but the one being photographed.
    required property int tipRow

    /// Whether this bucket's heading is on screen. Read off the heading itself: an emptied bucket keeps its heading,
    /// and the whole of that claim is that the thing is there (app-ui.md §UI 自動化の因果性).
    readonly property bool headed: bucketPane.visible && bucketHead.visible && bucketHead.height > 0
    /// The list itself — the pane walks the buckets through this (choice, automation, the arrow keys).
    readonly property alias list: bucketList
    /// How many rows this bucket is showing, and the room they would take with nothing in their way. What the pane
    /// shares the space out by (`WipPane.roomFor`).
    readonly property int rows: bucketPane.model.shownRows
    readonly property real wants: bucketPane.rows * Theme.rowHeight
    /// The heading's own height, which is not the bucket's to give: it stands whatever the bucket was given. The token
    /// rather than the heading's own measure — the pane works out from this how much room there is to hand out, and
    /// measuring an item whose height comes from that answer would be reading the answer back out of its question.
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
        // Qt's own key navigation moves `currentIndex` and tells nobody; the arrows are answered by the pane's walk,
        // which crosses from this bucket's list into the next one's (規約 §diff のファイル一覧).
        keyNavigationEnabled: false
        Keys.onUpPressed: event => event.accepted = bucketPane.pane.filesWalk.stepFile(-1)
        Keys.onDownPressed: event => event.accepted = bucketPane.pane.filesWalk.stepFile(1)
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
                // Choosing rows is not reading one: only a plain click moves the diff. Either way the press landed in
                // this list, so this is where the keyboard is (規約 §diff のファイル一覧).
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
            // One press, every highlighted row that can go the same way — and one git command for the lot, whatever the
            // count (デザイン規約 §その他の操作).
            onStageClicked: (bucket, path) => bucketPane.pane.moveStage(bucket, path)
        }
    }
}
