pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One sidebar section's scrolling list under its fixed NavHeader.
ListView {
    id: navList
    property var sectionModel
    property bool expanded: true
    property string kindHint: "branch"
    /// This list holds whatever height the panel it is in has over its
    /// rows, instead of staying content-sized. Only the one section the
    /// folded rail opens beside itself wants that — it is alone in its
    /// panel, so the spare height has nowhere else to go. In the sidebar
    /// proper the ground at the foot of the column takes it, and no
    /// section stretches (SidebarPane).
    property bool stretch: false
    // "↑a ↓b" of the current branch (branches section only).
    property string headTrack: ""
    /// The sidebar, which owns the row gestures: which row was clicked
    /// last and which is being typed into outlive both the delegates and
    /// this list, and only one row at a time is either, whichever section
    /// it sits in. A list with none (the WIP file list) simply has no
    /// gestures — every call below is skipped.
    property var gestures: null

    signal refActivated(string oidHex)
    signal fileActivated(string bucket, string path, string origPath)
    signal refMenuRequested(string kind, string name, string full, string oidHex)

    /// What identifies a row across sections and rebuilds. A colon cannot
    /// appear in a ref name, and the section prefix keeps two sections'
    /// equal names apart.
    function keyOf(full, name) {
        return navList.kindHint + ":" + (full !== "" ? full : name)
    }

    visible: expanded
    Layout.fillWidth: true
    Layout.fillHeight: expanded
    // A section closes on a hairline of ground — just enough to keep its
    // last row off the next header band. Anything thicker reads as a
    // blank row belonging to the section.
    Layout.maximumHeight: !expanded ? 0
                          : stretch ? Number.POSITIVE_INFINITY
                          : count * Theme.rowHeight + Theme.borderWidth
    clip: true
    model: sectionModel
    reuseItems: true
    boundsBehavior: Flickable.StopAtBounds
    ScrollBar.vertical: AutoScrollBar {}
    delegate: NavItemDelegate {
        id: row
        listWidth: navList.width
        kindHint: navList.kindHint
        headTrack: navList.headTrack
        rowKey: navList.gestures ? navList.keyOf(full, name) : ""
        activeKey: navList.gestures ? navList.gestures.activeKey : ""
        editKey: navList.gestures ? navList.gestures.editKey : ""
        editMode: navList.gestures ? navList.gestures.editMode : ""
        editText: navList.gestures ? navList.gestures.editText : ""
        editRefused: navList.gestures ? navList.gestures.editRefused : false
        editRefusedWhy: navList.gestures ? navList.gestures.editRefusedWhy : ""
        onRefClicked: oidHex => navList.refActivated(oidHex)
        onFileClicked: (bucket, path, origPath) => navList.fileActivated(bucket, path, origPath)
        onFolderClicked: key => navList.sectionModel.toggleFolder(key)
        onRefMenuRequested: (name, full, oidHex) =>
            navList.refMenuRequested(navList.kindHint, name, full, oidHex)
        onRowClicked: {
            if (navList.gestures)
                navList.gestures.noteClick(row.rowKey)
        }
        onActivateRequested: {
            if (navList.gestures)
                navList.gestures.activateRow(navList.kindHint, row.name,
                                             row.full, row.oid_hex)
        }
        onRenameRequested: {
            if (!navList.gestures)
                return
            // A stash is named by its message and known to git by its
            // selector; everything else answers to the name it shows.
            // A remote branch is typed without the remote it is on —
            // `origin/` is where the branch lives, not part of its name.
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
