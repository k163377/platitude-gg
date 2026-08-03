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
    // The stretching section absorbs the sidebar's leftover height;
    // the others stay content-sized.
    property bool stretch: false
    // "↑a ↓b" of the current branch (branches section only).
    property string headTrack: ""
    signal refActivated(string oidHex)
    signal fileActivated(string bucket, string path, string origPath)
    signal refMenuRequested(string name, string oidHex)

    visible: expanded
    Layout.fillWidth: true
    Layout.fillHeight: expanded
    Layout.maximumHeight: !expanded ? 0
                          : stretch ? Number.POSITIVE_INFINITY
                          : count * Theme.rowHeight + Theme.spaceXs
    clip: true
    model: sectionModel
    reuseItems: true
    ScrollBar.vertical: AutoScrollBar {}
    delegate: NavItemDelegate {
        listWidth: navList.width
        kindHint: navList.kindHint
        headTrack: navList.headTrack
        onRefClicked: oidHex => navList.refActivated(oidHex)
        onFileClicked: (bucket, path, origPath) => navList.fileActivated(bucket, path, origPath)
        onFolderClicked: key => navList.sectionModel.toggleFolder(key)
        onRefMenuRequested: (name, oidHex) => navList.refMenuRequested(name, oidHex)
    }
}
