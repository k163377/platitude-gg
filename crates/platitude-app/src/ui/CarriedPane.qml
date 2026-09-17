pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Right pane, another working copy's uncommitted work: whose it is, what it is holding, and the one door into it.
//
// **Built on the commit pane** (デザイン規約 §別の作業コピーを読む). What a reader can do with a
// commit and with somebody else's changes is the same thing — read them — so the shape is the same: a heading, a
// block saying whose, `CHANGES (n)`, and one flat list. The WIP pane's own furniture is all about the next commit
// *here* (three buckets for an index this window cannot move, a message, a button that records it), and a copy of it
// with every control switched off is a pane that describes a thing nobody can do.
//
// **One list.** The split into conflicted / unstaged / staged is the index's, and the index is that
// copy's. Told apart here, a file edited and then edited again stands twice under two headings that name two halves
// of an act nobody reading can take part in (`models::nav::Bucket::Whole` folds them to one row per path).
ColumnLayout {
    id: carriedPane

    /// The copy this pane is about — its own name, the folder's last segment, the same one the row's chip carries.
    required property string copyName
    /// Its changed files, folded to one row per path (`NavSectionModel` on the `whole` run).
    required property var files
    /// Which of them the middle pane is reading, handed down by the page — one copy here.
    ///
    /// **Both halves, because a row is found by the pair.** The rows of this one list still carry the side their
    /// bytes are on (`Bucket::routing_of`), which is what the walk matches a row by — so the bucket the page opened
    /// the file with is what has to come down here.
    property string readBucket: ""
    property string readPath: ""
    /// A right-click menu of the page's is standing over this pane.
    property bool menuStanding: false

    /// A file row was opened. `bucket` is the side the row's bytes are on, which the row works out for itself
    /// (`Bucket::routing_of`) — the reader never chose it.
    signal fileActivated(string bucket, string path, string origPath)
    signal fileWalked(string bucket, string path, string origPath)
    /// Tree or flat paths. The page applies it, because the choice is the pane's
    /// (`RepoPage.setWipTreeView`).
    signal treeViewChosen(bool tree)

    /// Automation only: the list itself and the walk over it, the same kind of exposure `GraphPane.view` is.
    readonly property alias view: fileList
    readonly property alias filesWalk: fileWalk
    /// Automation: whether the name had to be cut, and the words the hover behind it carries — the output side of
    /// the one thing a long name is judged on (`carried-read <長い名前>`).
    readonly property alias nameCut: nameLabel.cutting
    readonly property bool nameTipShown: nameSeat.ToolTip.visible
    /// Stands in for the pointer on the name, which headless cannot inject (verify-ui).
    property bool namePointedAt: false
    /// The same stand-in for a file row's own hover, which says the whole path (`FileRowDelegate.pointedTipRow`).
    /// -1 points at no row. **This list needs one of its own**: the pointer cannot be injected, and the path a row
    /// names is the one thing in it the row works out — a folder row here is handed a fold key
    /// beside its path, and the two are one field in the list this row was written for.
    property int pointedTipRow: -1
    /// How much of this pane's bottom edge is left bare for the corner text the page hangs there. **This pane lends
    /// the seat**, the same measurement and for the same reason the commit pane's does (`DetailsPane.bottomRoom`):
    /// nothing is pinned to its foot — what a reader can do with another copy's work is read it, so there is no
    /// button down there to be drawn across. The working tree's pane is the one that never lends it.
    readonly property real bottomRoom: carriedPane.height - fileList.y
        - Math.max(0, Math.min(fileList.height, fileList.originY + fileList.contentHeight - fileList.contentY))

    spacing: 0

    // The arrows walk the one list, so the walk is handed one side (`FileRowWalk` crosses buckets where there are
    // several — here there is one).
    FileRowWalk {
        id: fileWalk
        sides: [{ view: fileList, model: carriedPane.files }]
        readBucket: carriedPane.readBucket
        readPath: carriedPane.readPath
        onLanded: (bucket, path, origPath) => carriedPane.fileWalked(bucket, path, origPath)
    }

    // Whose copy this is, and nothing else. **The caption names the copy**: the band under it
    // says what they are, and a reader arriving at two bands has to be told which tree they belong to before being
    // told how many there are.
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
                text: qsTr("WORKING COPY")
                font.pixelSize: Theme.fontMd
                font.weight: Font.DemiBold
                color: Theme.textSecondary
            }
            // **A dot**, which is the device this window already separates a caption from
            // its value with (`DiffPaneHeader`). `on` is git's own word for the branch a tree is on (`On branch
            // main`), and what follows here is the copy's own folder name — a reader told `WORKING COPY ON topic`
            // would look for a branch called topic. The copy *is* that name, which is apposition
            // and takes no preposition at all.
            DotMark { tint: Theme.textSecondary }
            // The same mark the copy's chip and its sidebar row wear — one印 for one idea (規約 §ref の種別).
            NavIcon {
                kind: "tree"
                tint: Theme.textSecondary
                Layout.preferredWidth: Theme.iconSm
                Layout.preferredHeight: Theme.iconSm
                Layout.alignment: Qt.AlignVCenter
            }
            // **Cut in the middle, whole on the hover** (規約 §hover のツールチップ): a copy can be called anything and
            // the right pane's floor is 300px, so a long name loses its middle and keeps the ends it is told apart
            // by. The hover is the only place the whole of it is, which is why the seat under it takes a pointer.
            Item {
                id: nameSeat
                Layout.fillWidth: true
                Layout.preferredHeight: Theme.rowHeight
                CutName {
                    id: nameLabel
                    anchors.fill: parent
                    text: carriedPane.copyName
                    pixelSize: Theme.fontMd
                    weight: Font.DemiBold
                    color: Theme.textPrimary
                }
                HoverHandler { id: nameHover }
                ToolTip.visible: (nameHover.hovered || carriedPane.namePointedAt) && nameLabel.cutting
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: carriedPane.copyName
            }
        }
    }

    // What that copy is holding. **The same words this window's own pane heads its files with** — they are the same
    // kind of thing, and the band above is what says whose.
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
        // The pane's own bar (`DetailsPane` does the same).
        verticalBar: PaneScrollBar {}
        // The arrows are answered by the walk above: Qt's own key navigation moves `currentIndex` and tells nobody.
        keyNavigationEnabled: false
        Keys.onUpPressed: event => event.accepted = fileWalk.stepFile(-1, event.isAutoRepeat)
        Keys.onDownPressed: event => event.accepted = fileWalk.stepFile(1, event.isAutoRepeat)
        // The commit pane's row, and for the commit pane's reason: it draws a change and opens a diff, and has no
        // seat for a mark that stages.
        delegate: FileRowDelegate {
            listWidth: fileList.width
            menuStanding: carriedPane.menuStanding
            pointedTipRow: carriedPane.pointedTipRow
            readPath: carriedPane.readPath
            // **The names this model answers to.** These rows are a
            // `NavSectionModel`'s, which spells the whole path `full`, packs a folder's fold into the change code,
            // and is the only one of the two that carries a bucket at all (`models::nav::Role`) — and a role asked
            // for under a name the model does not answer comes back empty without a word said about it.
            //
            // **A folder row is the one that needs both**: this model folds by `<run>:<path>` and keeps the path
            // itself in the slot a rename would use, so the fold key and the name the hover says are two different
            // strings here (`NavItemDelegate.hoverText` reads the pair the same way for the working tree's list).
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
