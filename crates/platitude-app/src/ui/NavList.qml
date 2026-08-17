pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One sidebar section's scrolling list under its fixed NavHeader.
AppListView {
    id: navList
    property var sectionModel
    property bool expanded: true
    property string kindHint: "branch"
    /// This list holds whatever height the panel it is in has over its rows, instead of staying content-sized. Only the
    /// one section the folded rail opens beside itself wants that — it is alone in its panel, so the spare height has
    /// nowhere else to go. In the sidebar proper the ground at the foot of the column takes it, and no section
    /// stretches (SidebarPane).
    property bool stretch: false
    /// The current branch's ahead / behind (branches section only). Two numbers and whether it has an upstream at all:
    /// the arrows are drawn by the row, not spelled here.
    property bool headTracks: false
    property int headAhead: 0
    property int headBehind: 0
    /// The sidebar, which owns the row gestures: which row was clicked last and which is being typed into outlive both
    /// the delegates and this list, and only one row at a time is either, whichever section it sits in. A list with
    /// none (the WIP file list) simply has no gestures — every call below is skipped.
    property var gestures: null

    signal refActivated(string oidHex)
    signal fileActivated(string bucket, string path, string origPath)
    signal refMenuRequested(string kind, string name, string full, string oidHex)

    /// What identifies a row across sections and rebuilds. A colon cannot appear in a ref name, and the section prefix
    /// keeps two sections' equal names apart.
    function keyOf(full, name) {
        return navList.kindHint + ":" + (full !== "" ? full : name)
    }

    /// The box has opened on a row: if it is one of this section's, bring it into view. A row can be typed into
    /// without having been clicked — the current branch's sticky row raises the same menu while the real row is
    /// scrolled off (`HeadPinRow`) — and a name changing itself somewhere off screen is a name nobody agreed to.
    /// Watched rather than told: only the gestures know when a box opens, and every list they reach is one of these.
    Connections {
        target: navList.gestures
        function onEditKeyChanged() {
            const key = navList.gestures.editKey
            const head = navList.kindHint + ":"
            if (!key.startsWith(head))
                return
            // What is on show, so a row behind a filter or a closed folder answers -1 rather than a stranger's place.
            const row = navList.sectionModel.rowOfName(key.substring(head.length))
            if (row >= 0)
                navList.positionViewAtIndex(row, ListView.Contain)
        }
    }

    /// Smoke hooks (PG_AUTO_ACT=nav-reclick): a left click on one row, and what that row made of it. Clicks cannot be
    /// injected (verify-ui スキル), so they go in at the row's own answer. `clickRow` says false when the view has not
    /// built that row yet — the delegate arrives on the layout after the model got the rows, and a run that counted
    /// the miss as a press would wait for a gesture nobody made (app-ui.md §UI 自動化の因果性).
    function clickRow(index) {
        const row = navList.itemAtIndex(index)
        if (!row)
            return false
        row.leftClick(Qt.NoModifier)
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
    /// Where the list has actually scrolled to, and how a run puts a row out of sight to begin with. Read off
    /// `contentY` rather than off a delegate: a row scrolled away has none, and "there is no delegate" is also what a
    /// list that has not been built yet says.
    function rowInView(index) {
        const top = index * Theme.rowHeight
        return top >= navList.contentY && top + Theme.rowHeight <= navList.contentY + navList.height
    }
    function scrollToEnd() {
        navList.contentY = Math.max(0, navList.contentHeight - navList.height)
    }

    visible: expanded
    Layout.fillWidth: true
    Layout.fillHeight: expanded
    // A section closes on a hairline of ground — just enough to keep its last row off the next header band. Anything
    // thicker reads as a blank row belonging to the section.
    Layout.maximumHeight: !expanded ? 0
                          : stretch ? Number.POSITIVE_INFINITY
                          : count * Theme.rowHeight + Theme.borderWidth
    model: sectionModel
    delegate: NavItemDelegate {
        id: row
        listWidth: navList.width
        kindHint: navList.kindHint
        headTracks: navList.headTracks
        headAhead: navList.headAhead
        headBehind: navList.headBehind
        rowKey: navList.gestures ? navList.keyOf(full, name) : ""
        activeKey: navList.gestures ? navList.gestures.activeKey : ""
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
        onRowClicked: {
            if (navList.gestures)
                navList.gestures.noteClick(row.rowKey)
        }
        onActivateRequested: {
            if (navList.gestures)
                navList.gestures.activateRow(navList.kindHint, row.name, row.full, row.oid_hex)
        }
        onRenameRequested: {
            if (!navList.gestures)
                return
            // A stash is named by its message and known to git by its selector; everything else answers to the name it
            // shows. A remote branch is typed without the remote it is on — `origin/` is where the branch lives, not
            // part of its name.
            const id = row.full !== "" ? row.full : row.name
            navList.gestures.startEdit(
                navList.kindHint, row.rowKey, "rename", id, row.oid_hex,
                navList.kindHint === "stash" ? row.name
                : navList.kindHint === "remote"
                  ? id.substring(id.indexOf("/") + 1) : id)
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
