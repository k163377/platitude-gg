pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import platitude
import platitude.ui

// One bucket of the worktree as a list of its own: the heading pinned on top and the files scrolling under it,
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
    required property var worktree
    /// This bucket's rows (`NavSectionModel` attached to this run).
    required property var model
    /// Automation's stand-in pointer row (PGG_AUTO_ACT=path-tip): the pane's row less the rows above this bucket, so
    /// out of range in every other bucket.
    required property int tipRow
    /// The hand over the pane's file rows (`WipPane`), which the rows read by geometry (`NavItemDelegate.syncHand`).
    property HandWatch paneHand: null

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
    /// The row the hand was on as the model began to deal the rows again (its `fullName`), "" for none, and the
    /// hand's move count then (`HandWatch.moves`) — weighed once they are dealt (`weighDeal`).
    property string handBefore: ""
    property int movesBefore: -1

    /// Automation: press this bucket's whole-bucket button where a hand presses it (`WipBucketHeader.moveAll`).
    function moveAll() {
        return bucketHead.moveAll()
    }

    /// The built row under the pane's hand, or null: the hand off this list, unheard, or on no row of it.
    function rowUnderHand() {
        const hand = bucketPane.paneHand
        if (hand === null || !hand.known || !hand.over(bucketList))
            return null
        const p = bucketList.contentItem.mapFromItem(null, hand.at.x, hand.at.y)
        return bucketList.itemAt(p.x, p.y)
    }
    /// A built row of this list the hand lights (`NavItemDelegate.handOn`) — read before the rows' places are weighed
    /// again, so it is the row the hand was on.
    function handRow() {
        const rows = bucketList.contentItem.children
        for (let i = 0; i < rows.length; i++) {
            if (rows[i].handOn === true)
                return rows[i]
        }
        return null
    }
    /// The rows here are the hand's: under it now, lit by it before the move, or hung with the line-ending card.
    function holdsHand() {
        return bucketPane.rowUnderHand() !== null || bucketPane.handRow() !== null
            || bucketPane.pane.eolCardIn(bucketPane.section)
    }

    /// **Every status read that brings news deals the rows again** (`NavSectionModel` resets; so does a folder
    /// opened or shut): the delegates are new, and none of them has heard the hand. **The same row dealt under the
    /// hand takes it back** — nothing moved under it, so the hand on it is still the hand on it: the light, the stage
    /// button, the asks. **Another row dealt there arrived under a still hand** — the rows moved under it
    /// (`HandWatch.rowsMoved`: the row there lights, nothing opens until the hand moves, what hung off the rows goes);
    /// the row that had the hand is gone from under it, and its ask for the card with it — the delegate that asked
    /// went without sending the leave. A hand that moved meanwhile has been read by the new rows already.
    function weighDeal() {
        const was = bucketPane.handBefore
        bucketPane.handBefore = ""
        const hand = bucketPane.paneHand
        if (was === "" || hand === null || hand.moves !== bucketPane.movesBefore)
            return
        const row = bucketPane.rowUnderHand()
        if (row !== null && row.fullName === was) {
            row.syncHand(false)
            return
        }
        bucketPane.pane.unpointEolIn(bucketPane.section, was)
        hand.rowsMoved()
    }

    spacing: 0
    // The bucket stepping down or up the pane — the bucket above it grew or shrank with a status read — moves its rows
    // under the hand as a scroll does, where they are the hand's.
    onYChanged: {
        if (bucketPane.paneHand !== null && bucketPane.holdsHand())
            bucketPane.paneHand.rowsMoved()
    }

    Connections {
        target: bucketPane.model
        function onModelAboutToBeReset() {
            const row = bucketPane.handRow()
            bucketPane.handBefore = row !== null ? row.fullName : ""
            bucketPane.movesBefore = bucketPane.paneHand !== null ? bucketPane.paneHand.moves : -1
        }
        // A turn later: the view deals the new rows in its own answer to the reset.
        function onModelReset() { Qt.callLater(bucketPane.weighDeal) }
    }

    WipBucketHeader {
        id: bucketHead
        Layout.fillWidth: true
        section: bucketPane.section
        repoTab: bucketPane.repoTab
        worktree: bucketPane.worktree
        bucketModel: bucketPane.model
    }
    AppListView {
        id: bucketList
        Layout.fillWidth: true
        Layout.fillHeight: true
        model: bucketPane.model
        // The pane's hand for the tips the rows put out, as for their light (`TipAsk.hand`).
        rowsHand: bucketPane.paneHand
        // The pane's bar, not the style's (`AppListView`).
        verticalBar: PaneScrollBar {}
        // Qt's own key navigation moves `currentIndex` and tells nobody; the arrows are answered by the pane's walk,
        // which crosses from this bucket's list into the next one's (規約 §diff のファイル一覧).
        keyNavigationEnabled: false
        Keys.onUpPressed: event => event.accepted = bucketPane.pane.filesWalk.stepFile(-1, event.isAutoRepeat)
        Keys.onDownPressed: event => event.accepted = bucketPane.pane.filesWalk.stepFile(1, event.isAutoRepeat)
        // The menu key; Windows' Shift+F10 comes through the window (`Main.keyMenuAsked`).
        Keys.onMenuPressed: event => event.accepted = bucketPane.pane.menuFromKeys()
        // The rows moved under the hand (`HandWatch.rowsMoved`): the reader's wheel and bar, and the keyboard's walk
        // bringing a row on (`FileRowWalk`) — nothing of this list's own moves its rows (there is no row that grows).
        onContentYChanged: {
            if (bucketPane.paneHand !== null)
                bucketPane.paneHand.rowsMoved()
        }
        delegate: NavItemDelegate {
            listWidth: bucketList.width
            kindHint: "file"
            showStage: true
            chosen: bucketPane.pane.isChosen(bucket, fullName)
            sideOurs: bucketPane.worktree.sideOurs
            sideTheirs: bucketPane.worktree.sideTheirs
            pointedTipRow: bucketPane.tipRow
            // This bucket's own model names the row the card is out for: the same path in another bucket is another
            // row (`WipPane.pointEolIn`).
            pointedEolPath: bucketPane.model.pointedEolPath
            paneHand: bucketPane.paneHand
            // The target is this list's row: the section names the list, not the change (an untracked file's own
            // bucket is `untracked`, shown in the unstaged list).
            onEolPointed: (path, on) =>
                on ? bucketPane.pane.pointEolIn(bucketPane.section, path)
                   : bucketPane.pane.unpointEolIn(bucketPane.section, path)
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
