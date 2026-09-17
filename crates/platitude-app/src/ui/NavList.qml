pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import platitude.ui

// One sidebar section's scrolling list under its fixed NavHeader.
AppListView {
    id: navList
    property var sectionModel
    property bool expanded: true
    property string kindHint: "branch"
    /// REMOTES only: the configured remotes as the model packs them, and which of them this repository sends pushes
    /// to. Unpacked once here — the rows are recycled, so neither answer can be theirs to keep, and one list is one
    /// split (デザイン規約 §その他の操作).
    property string remotesPacked: ""
    property string markedRemote: ""
    readonly property var remoteNames:
        navList.remotesPacked === "" ? [] : navList.remotesPacked.split(String.fromCharCode(31))
    /// This list holds whatever height the panel it is in has over its rows. Only the one section the folded rail
    /// opens beside itself wants that — it is alone in its panel, so the spare height has nowhere else to go. In the
    /// sidebar proper the ground at the foot of the column takes it, and no section stretches
    /// (SidebarPane).
    property bool stretch: false
    /// The sidebar, which owns the row gestures: which row was clicked last and which is being typed into outlive both
    /// the delegates and this list, and only one row at a time is either, whichever section it sits in. A list with
    /// none (the WIP file list) simply has no gestures — every call below is skipped.
    property var gestures: null
    /// Stands in for the pointer where headless cannot put one, so a row's tooltip — or the absence of one — can be
    /// photographed (PGG_AUTO_ACT=nav-tip). -1 points at no row. The file lists carry the same property on their own
    /// panes (`WipPane` / `DetailsPane`).
    property int pointedTipRow: -1
    /// Where a row's open name box is drawn — outside this list, which clips, because the box is allowed past the
    /// pane's edge when what is in it does not fit (`NavItemDelegate`).
    ///
    /// **Two out.** One out is the column the sections are laid out in, and a layout lays out whatever is parented
    /// into it — a box put there is given the column's own next row (measured: it landed at the foot of the pane).
    /// Two out is what that column fills, which lays nothing out and clips nothing.
    ///
    /// A list whose rows cannot be typed into is left with none: the working tree's file lists have no gestures, so
    /// no row of theirs ever opens a box, and naming a layer for them would only say where a box that never comes
    /// would have gone. Settable, for a surface where the box is better off staying in its seat.
    property Item boxLayer:
        navList.gestures && navList.parent ? navList.parent.parent : null
    /// Where this list's rows begin in that layer: its own place in the column, and the column's in the layer.
    readonly property real boxRowsX: (navList.parent ? navList.parent.x : 0) + navList.x
    readonly property real boxRowsTop: (navList.parent ? navList.parent.y : 0) + navList.y

    signal refActivated(string oidHex)
    signal fileActivated(string bucket, string path, string origPath)
    signal refMenuRequested(string kind, string name, string full, string oidHex)
    /// Right-click on the row a remote itself stands on.
    signal remoteMenuRequested(string name)

    /// What identifies a row across sections and rebuilds. A colon cannot appear in a ref name, and the section prefix
    /// keeps two sections' equal names apart.
    function keyOf(full, name) {
        return navList.kindHint + ":" + (full !== "" ? full : name)
    }

    /// The box has opened on a row: if it is one of this section's, bring it into view. A row can be typed into
    /// without having been clicked — the current branch's sticky row raises the same menu while the real row is
    /// scrolled off (`HeadPinRow`) — and a name changing itself somewhere off screen is a name nobody agreed to.
    /// Watched: only the gestures know when a box opens, and every list they reach is one of these.
    Connections {
        target: navList.gestures
        function onEditKeyChanged() {
            const key = navList.gestures.editKey
            const head = navList.kindHint + ":"
            if (!key.startsWith(head))
                return
            // What is on show, so a row behind a filter or a closed folder answers -1.
            const row = navList.sectionModel.rowOfName(key.substring(head.length))
            if (row >= 0)
                navList.positionViewAtIndex(row, ListView.Contain)
        }
    }

    /// Smoke hooks (PGG_AUTO_ACT=nav-reclick): a left click on one row, and what that row made of it. Clicks cannot be
    /// injected (verify-ui スキル), so they go in at the row's own answer. `clickRow` says false when the view has not
    /// built that row yet — the delegate arrives on the layout after the model got the rows, and a run that counted
    /// the miss as a press would wait for a gesture nobody made (app-ui.md §UI 自動化の因果性).
    function clickRow(index) {
        const row = navList.itemAtIndex(index)
        if (!row)
            return false
        // Nothing was held down: a run with no pointer has no press to time (`ReclickGesture.click`).
        row.leftClick(Qt.NoModifier, 0)
        return true
    }
    function rowArmed(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.renameArmed : false
    }
    function rowGuarded(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.clickGuarded : false
    }
    function rowFocused(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.editFocused : false
    }
    /// The box on one row, as drawn and as the question on it wants to be drawn (PGG_AUTO_ACT=nav-branch-box). Neither
    /// is anything a picture answers: an elided placeholder frames like a shorter question. A row the view has not
    /// built answers 0 — the same miss `clickRow` reports as false.
    function rowBoxWidth(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.editBoxWidth : 0
    }
    function rowBoxWhole(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.editBoxWhole : 0
    }
    function rowBoxSeat(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.editBoxSeat : 0
    }
    function rowBoxShown(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.editBoxShown : false
    }
    function rowBoxAt(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.editBoxAt : ""
    }
    /// Whether the shared tooltip is standing on that row's box (PGG_AUTO_ACT=rename-tag-box). A row the view has not
    /// built answers false — the same miss the rest of these report.
    function rowTipShown(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.editTipShown : false
    }
    /// What the pointed row itself would say, and what it is called (PGG_AUTO_ACT=nav-tip). The row decides and the
    /// shared instance shows, so the two are read apart: a row with nothing to say never reaches the instance. Read
    /// off `hoverText` — **the attached `ToolTip.visible` reads back the instance's own state**, so during the delay
    /// a row that does speak answers false. An empty name says the view has not built that row yet — the same miss
    /// `clickRow` reports as false.
    function rowTipWords(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.hoverText : ""
    }
    function rowNameAt(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.name : ""
    }
    /// What commit that row names, read off the row itself (PGG_AUTO_ACT=nav-jump): where a click on it leads is the
    /// row's own answer, and a run that worked the commit out from the model instead would be asking the question the
    /// click is supposed to answer. Empty on a row the view has not built — and on the one row here that names no
    /// commit, a bare worktree entry.
    function rowOidAt(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.oid_hex : ""
    }
    /// Where the list has actually scrolled to, and how a run puts a row out of sight to begin with. Read off
    /// `contentY`: a row scrolled away has no delegate, and "there is no delegate" is also what a list that has not
    /// been built yet says.
    function rowInView(index) {
        const top = index * Theme.rowHeight
        return top >= navList.contentY && top + Theme.rowHeight <= navList.contentY + navList.height
    }
    function scrollToEnd() {
        navList.contentY = Math.max(0, navList.contentHeight - navList.height)
    }
    /// A few rows on, which is how a run puts a row just out of sight (PGG_AUTO_ACT=nav-branch-box:away). A few,
    /// because past the view's own cache the delegate is gone, and a box that went with it proves nothing about the
    /// one rule being read — that a box whose row has left the list goes with it.
    function scrollRows(rows) {
        navList.contentY = Math.min(Math.max(0, navList.contentHeight - navList.height),
                                    navList.contentY + rows * Theme.rowHeight)
    }

    visible: expanded
    Layout.fillWidth: true
    Layout.fillHeight: expanded
    // A section closes on a hairline of ground — just enough to keep its last row off the next header band. Anything
    // thicker reads as a blank row belonging to the section. A top margin is a seat given to something standing over
    // the rows, so the section asks for it on top of them (`NavSections` / `HeadPinRow`); every other list here has
    // none and adds nothing.
    Layout.maximumHeight: !expanded ? 0
                          : stretch ? Number.POSITIVE_INFINITY
                          : count * Theme.rowHeight + navList.topMargin + Theme.borderWidth
    model: sectionModel
    // The pane's own bar, in place of the style's one that `AppListView` hands the graph, the diff and the log.
    verticalBar: PaneScrollBar {}
    delegate: NavItemDelegate {
        id: row
        listWidth: navList.width
        // Both margins of the panel are the one the bar asks for at the right edge, so the rows sit between equal
        // sides; the folds step in by that same value
        // (`NavItemDelegate.rowInset` / デザイン規約 §余白 の左メニューの行の項).
        rowInset: Theme.spaceSm
        nestStep: Theme.spaceSm
        // The list's own place in that layer, so a scroll carries the box along with the row it belongs to.
        boxLayer: navList.boxLayer
        boxRowsX: navList.boxRowsX
        boxRowsY: navList.boxRowsTop - navList.contentY
        boxRowsTop: navList.boxRowsTop
        boxRowsHeight: navList.height
        kindHint: navList.kindHint
        remoteNames: navList.remoteNames
        markedRemote: navList.markedRemote
        pointedTipRow: navList.pointedTipRow
        rowKey: navList.gestures ? navList.keyOf(full, name) : ""
        activeKey: navList.gestures ? navList.gestures.activeKey : ""
        // The gesture itself: the wait a second click opens has to outlive this row, and a list whose rows cannot be
        // typed into has none (`ReclickGesture`).
        reclick: navList.gestures ? navList.gestures.reclick : null
        editKey: navList.gestures ? navList.gestures.editKey : ""
        menuStanding: navList.gestures ? navList.gestures.menuOpen : false
        editMode: navList.gestures ? navList.gestures.editMode : ""
        editText: navList.gestures ? navList.gestures.editText : ""
        editRefused: navList.gestures ? navList.gestures.editRefused : false
        editRefusedWhy: navList.gestures ? navList.gestures.editRefusedWhy : ""
        onRefClicked: oidHex => navList.refActivated(oidHex)
        onFileClicked: (bucket, path, origPath) => navList.fileActivated(bucket, path, origPath)
        onFolderClicked: key => navList.sectionModel.toggleFolder(key)
        onRefMenuRequested: (name, full, oidHex) => navList.refMenuRequested(navList.kindHint, name, full, oidHex)
        onRemoteMenuRequested: name => navList.remoteMenuRequested(name)
        onRowClicked: {
            if (navList.gestures)
                navList.gestures.noteClick(row.rowKey)
        }
        onActivateRequested: {
            if (navList.gestures)
                navList.gestures.activateRow(navList.kindHint, row.name, row.full, row.oid_hex)
        }
        onEditTyped: text => {
            if (navList.gestures)
                navList.gestures.editText = text
        }
        onEditAccepted: text => {
            if (navList.gestures)
                navList.gestures.submitEdit(text)
        }
        onEditCancelled: {
            if (navList.gestures)
                navList.gestures.stopEdit()
        }
    }
}
