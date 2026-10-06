pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Right pane, another working copy's uncommitted work: whose it is and what it is holding. Built on the commit pane,
// not the WIP pane — it can only be read (デザイン規約 §別の作業コピーを読む).
//
// One list, one row per path (`models::nav::Bucket::Whole`): the staged / unstaged split is that copy's index, which
// nothing here moves.
ColumnLayout {
    id: carriedPane

    /// The copy's name — the folder's last segment, the same one the row's chip carries.
    required property string copyName
    /// Its changed files, one row per path (`NavSectionModel` on the `whole` run).
    required property var files
    /// Which of them the middle pane is reading. Both halves: the walk matches a row by the pair, and the rows still
    /// carry the side their bytes are on (`Bucket::routing_of`).
    property string readBucket: ""
    property string readPath: ""
    /// A right-click menu of the page's is standing over this pane.
    property bool menuStanding: false

    /// A file row was opened. `bucket` is the side the row's bytes are on, which the row works out itself
    /// (`Bucket::routing_of`).
    signal fileActivated(string bucket, string path, string origPath)
    signal fileWalked(string bucket, string path, string origPath)
    /// Tree or flat paths; the page applies it (`RepoPage.setWipTreeView`).
    signal treeViewChosen(bool tree)

    /// Automation only: the list itself and the walk over it.
    readonly property alias view: fileList
    readonly property alias filesWalk: fileWalk
    /// Automation: whether the name was cut, and whether its hover is out (`carried-read <長い名前>`).
    readonly property alias nameCut: nameLabel.cutting
    readonly property bool nameTipShown: nameSeat.ToolTip.visible
    /// Stands in for the pointer on the name, which headless cannot inject (verify-ui).
    property bool namePointedAt: false
    /// The same stand-in for a file row's hover (`FileRowDelegate.pointedTipRow`); -1 points at no row. This list
    /// needs its own: a folder row here carries a fold key beside its path, which are one field in the commit list.
    property int pointedTipRow: -1
    /// How much of the bottom edge is left bare for the page's corner text. This pane lends it, as the commit pane's
    /// does (`DetailsPane.bottomRoom`): nothing is pinned to its foot.
    readonly property real bottomRoom: carriedPane.height - fileList.y
        - Math.max(0, Math.min(fileList.height, fileList.originY + fileList.contentHeight - fileList.contentY))

    spacing: 0

    FileRowWalk {
        id: fileWalk
        sides: [{ view: fileList, model: carriedPane.files }]
        readBucket: carriedPane.readBucket
        readPath: carriedPane.readPath
        onLanded: (bucket, path, origPath) => carriedPane.fileWalked(bucket, path, origPath)
    }

    // Whose copy this is, and nothing else; the band under it says what it holds.
    Rectangle {
        Layout.fillWidth: true
        implicitHeight: Theme.headerHeight
        color: Theme.bgElevated
        BandRule {}
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Theme.spaceXs
            anchors.rightMargin: Theme.spaceXs
            spacing: Theme.spaceXs
            Label {
                text: qsTr("WORKTREE")
                font.pixelSize: Theme.fontMd
                font.weight: Theme.fontWeightStrong
                color: Theme.textSecondary
            }
            // A dash, not `on` (規約 §別の作業コピーを読む), and as a character (規約 §寸法「字で出ていた記号は 5 種」).
            Label {
                text: "—"
                font.pixelSize: Theme.fontMd
                color: Theme.textSecondary
            }
            // The mark and the name are one word (規約 §別の作業コピーを読む): the seat is the mark's own ink, so its
            // box's spare air does not read as a gap.
            RowLayout {
                Layout.fillWidth: true
                spacing: 0
                Item {
                    Layout.preferredWidth: nameMark.inkWidth
                    Layout.preferredHeight: Theme.iconSm
                    Layout.alignment: Qt.AlignVCenter
                    NavIcon {
                        id: nameMark
                        anchors.centerIn: parent
                        kind: "tree"
                        tint: Theme.textSecondary
                        width: Theme.iconSm
                        height: Theme.iconSm
                    }
                }
                // Cut in the middle, whole on the hover (規約 §別の作業コピーを読む) — the only place the whole name
                // is, so the seat takes a pointer.
                Item {
                    id: nameSeat
                    Layout.fillWidth: true
                    Layout.preferredHeight: Theme.rowHeight
                    CutName {
                        id: nameLabel
                        anchors.fill: parent
                        text: carriedPane.copyName
                        pixelSize: Theme.fontMd
                        weight: Theme.fontWeightStrong
                        // The caption's colour: the band is one phrase (規約 §別の作業コピーを読む).
                        color: Theme.textSecondary
                    }
                    HoverHandler { id: nameHover }
                    ToolTip.visible: (nameHover.hovered || carriedPane.namePointedAt) && nameLabel.cutting
                    ToolTip.delay: Metrics.tipDelayMs
                    ToolTip.text: carriedPane.copyName
                }
            }
        }
    }

    // What that copy is holding, in the words this window's own pane uses.
    DetailsChangesBand {
        Layout.fillWidth: true
        caption: qsTr("UNCOMMITTED CHANGES")
        count: carriedPane.files.total
        treeView: carriedPane.files.treeView
        onChosen: tree => carriedPane.treeViewChosen(tree)
    }

    AppListView {
        id: fileList
        Layout.fillWidth: true
        Layout.fillHeight: true
        model: carriedPane.files
        verticalBar: PaneScrollBar {}
        // The arrows are answered by the walk above: Qt's own key navigation moves `currentIndex` and tells nobody.
        keyNavigationEnabled: false
        Keys.onUpPressed: event => event.accepted = fileWalk.stepFile(-1, event.isAutoRepeat)
        Keys.onDownPressed: event => event.accepted = fileWalk.stepFile(1, event.isAutoRepeat)
        // The commit pane's row: it opens a diff and has no seat for a mark that stages.
        delegate: FileRowDelegate {
            listWidth: fileList.width
            menuStanding: carriedPane.menuStanding
            pointedTipRow: carriedPane.pointedTipRow
            readPath: carriedPane.readPath
            // The names a `NavSectionModel` answers to (`models::nav::Role`) — a role it does not answer comes back
            // empty without a word. A folder row's fold key is `full` (`<run>:<path>`) and its path is `orig_path`
            // (`NavItemDelegate.hoverText` reads the pair the same way).
            pathText: model.folder === true ? model.orig_path : model.full
            foldKey: model.full
            isFolded: model.change === "FOLDED"
            bucket: model.bucket
            onActivated: (bucket, path, origPath) => {
                fileList.forceActiveFocus()
                carriedPane.fileActivated(bucket, path, origPath)
            }
            onFolderToggled: key => carriedPane.files.toggleFolder(key)
        }
    }
}
