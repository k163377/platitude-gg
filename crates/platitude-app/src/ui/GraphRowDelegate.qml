pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Commit-graph row: [branch/tag chips][lanes + identicon node][subject] — fixed-width label and graph columns keep
// subjects aligned.
Item {
    id: rowItem
    required property int index
    required property string oid_hex
    required property string author
    required property double atime
    required property string subject
    /// Everything after the subject, minus the co-author trailers. Only the hover card reads it — but the role has to
    /// be declared here or it arrives as `undefined` and whoever touches it stops mid-way.
    required property string body
    required property int node_lane
    required property int node_color
    required property int avatar
    /// A picture this author was given, or empty for the generated pattern. Resolved in Rust onto the row
    /// (models::graph).
    required property string avatar_url
    /// Packed `Co-authored-by` records; the first one badges the node (GraphLaneCell), and the hover card names the
    /// rest.
    required property string co_authors
    required property string geometry
    required property string labels
    required property string stash_ref
    /// The find bar's line is somewhere in this row (platitude-core::find decides; the model marks it). Only ever true
    /// while a search is on.
    required property bool matched
    // Dimmed because the search passed this row over — not because it is in any way unavailable (デザイン規約 §暗く落とした段).
    readonly property bool dimmed: rowItem.ListView.view ? rowItem.ListView.view.findOn && !rowItem.matched : false

    width: ListView.view.width
    height: Theme.graphRowHeight

    readonly property bool selected: ListView.isCurrentItem
    // The top row's highlight and hit area bleed over the list's top margin: hovering or selecting the first commit
    // shows one unbroken band level with the neighbouring header bands instead of leaving a dark sliver above the row.
    readonly property real topBleed: index === 0 && ListView.view ? ListView.view.topMargin : 0
    // The all-zero id marks the synthetic uncommitted-changes row.
    readonly property bool isWip: oid_hex !== "" && !/[^0]/.test(oid_hex)
    // Chip records are separated by U+001F (see encode.rs), and arrive in the order the chip reads them out: HEAD →
    // local → remote → tag. One chip for the row, so a commit that is both a branch tip and a release shows the branch
    // — the tag is behind the "+N", where the hover card has it.
    readonly property var labelRecords: labels === "" ? [] : labels.split(String.fromCharCode(31))
    readonly property var branchRecords: labelRecords.filter(r => r[0] !== "T")
    // Whether this row is somewhere HEAD could stand: the working-tree row is not a commit, and a stash sits on no
    // branch's history.
    readonly property bool movable: !rowItem.isWip && rowItem.stash_ref === ""
    // The commit the working tree is standing on. Asked of the row number the model settled rather than of the chips
    // this row carries: a detached HEAD is the same answer and the chip that says so is a different kind
    // (`models::graph::head`).
    readonly property bool isHead: rowItem.ListView.view ? rowItem.ListView.view.headRow === rowItem.index : false
    // Where a double-click on this row goes: the branch chip's own first record, so what is on screen is what is moved
    // to. Empty means the row shows no branch, which is the offer to put one there.
    readonly property string primaryRecord:
        rowItem.movable && rowItem.branchRecords.length > 0 ? rowItem.branchRecords[0] : ""
    // The chip itself — what a stacked one is unstacked under.
    readonly property alias chipItem: chipColumn.chipItem
    // What the name box on this row came out to (see the column — a headless run reads it off here).
    readonly property alias nameBoxWidth: chipColumn.nameBoxWidth
    // This row's chip column is a branch-name box right now.
    readonly property bool naming: rowItem.ListView.view ? rowItem.ListView.view.namingOid === rowItem.oid_hex : false
    // The standing question is about this row. Its words are on the bar above the graph; the row answers "which one"
    // and nothing else.
    readonly property bool marked: rowItem.ListView.view ? rowItem.ListView.view.askOid === rowItem.oid_hex : false

    Rectangle {
        anchors.fill: parent
        anchors.topMargin: -rowItem.topBleed
        color: Theme.bgSelected
        visible: rowItem.selected
    }
    Rectangle {
        anchors.fill: parent
        anchors.topMargin: -rowItem.topBleed
        color: Theme.bgHover
        visible: rowMouse.containsMouse && !rowItem.selected
    }
    // The row the standing question is about: the tone of the bar, run down the edge the rows begin at.
    Rectangle {
        anchors.fill: parent
        anchors.topMargin: -rowItem.topBleed
        color: Theme.bgHover
        visible: rowItem.marked
        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: Metrics.laneStroke
            color: rowItem.ListView.view && rowItem.ListView.view.askDanger ? Theme.danger : Theme.warning
        }
    }

    // A row coming back out of the reuse pool has to redraw even when none of its roles differs from the row it was
    // last used for: what it holds is the picture it was left with, and the graph may have grown a lane in the meantime
    // — see GraphLaneCell's canvases. The attached `ListView` only exists on the delegate's root, so the asking is done
    // from here rather than in the cell.
    ListView.onReused: {
        laneCell.loadFace()
        laneCell.repaintLanes()
    }

    readonly property real labelsW: ListView.view ? ListView.view.labelWidth : Metrics.labelColW

    RowLayout {
        anchors.fill: parent
        spacing: 0

        GraphRowChips {
            id: chipColumn
            Layout.preferredWidth: rowItem.labelsW
            Layout.fillHeight: true
            opacity: rowItem.dimmed ? Metrics.dimFade : 1
            records: rowItem.labelRecords
            columnWidth: rowItem.labelsW
            naming: rowItem.naming
            onNamingSubmitted: name => rowItem.ListView.view.namingSubmitted(rowItem.oid_hex, name)
            onNamingEdited: text => rowItem.ListView.view.namingText = text
            onNamingCancelled: rowItem.ListView.view.namingCancelled()
        }

        GraphLaneCell {
            id: laneCell
            Layout.preferredWidth: rowItem.ListView.view ? rowItem.ListView.view.graphColWidth : 0
            Layout.fillHeight: true
            xOffset: rowItem.ListView.view ? rowItem.ListView.view.graphXOffset : 0
            fullWidth: rowItem.ListView.view ? rowItem.ListView.view.graphFullWidth : 0
            geometry: rowItem.geometry
            nodeLane: rowItem.node_lane
            coAuthors: rowItem.co_authors
            avatar: rowItem.avatar
            avatarUrl: rowItem.avatar_url
            isWip: rowItem.isWip
            stashRef: rowItem.stash_ref
            dimmed: rowItem.dimmed
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: Theme.spaceXs
            // The tick goes with the message, not with the lanes: it stands in the subject column and belongs to this
            // row alone.
            opacity: rowItem.dimmed ? Metrics.dimFade : 1
            // Short colored tick before the message: separates rows visually (deliberately not a continuous line) and
            // echoes the commit's chain color.
            //
            // These three steps — this margin, this width, the layout's spacing — are what `GraphPane.subjectTextX`
            // adds up, since that is where the find bar's cap is measured from.
            Rectangle {
                Layout.leftMargin: Theme.spaceSm
                implicitWidth: 2 * Theme.borderWidth
                implicitHeight: Theme.iconMd
                radius: Theme.borderWidth
                color: Theme.graphLane[rowItem.node_color % Theme.graphLane.length]
            }
            Label {
                // The uncommitted row's words are a fixed length, and the counts read as part of the same sentence: it
                // keeps its own width so they sit right after it rather than out at the pane's far edge.
                Layout.fillWidth: !rowItem.isWip
                // No total: the tallies beside it add up to exactly that number, and the pane's own heading says it as
                // well (規約 §未コミット行が名乗るもの).
                text: rowItem.isWip ? qsTr("Uncommitted changes") : rowItem.subject
                elide: Text.ElideRight
                font.pixelSize: Theme.fontMd
                // The commit the working tree is standing on writes its message in the branch's own blue — the same
                // `textLink` the chip on it uses, and the same the stand-in uses while this row is scrolled off
                // (規約 §グラフの中で HEAD を見失わない). One rule for the one commit, so nothing changes under the reader when
                // they go there and the stand-in steps aside (2026-08-22 ユーザー判断).
                color: rowItem.isWip ? Theme.textSecondary
                       : rowItem.isHead ? Theme.textLink
                       : Theme.textPrimary
                rightPadding: rowItem.isWip ? 0 : Theme.spaceSm
            }
            WipTallyRow {
                Layout.leftMargin: Theme.spaceSm
                Layout.rightMargin: Theme.spaceSm
                conflicted: rowItem.isWip && rowItem.ListView.view ? rowItem.ListView.view.wipConflicted : 0
                added: rowItem.isWip && rowItem.ListView.view ? rowItem.ListView.view.wipAdded : 0
                modified: rowItem.isWip && rowItem.ListView.view ? rowItem.ListView.view.wipModified : 0
                deleted: rowItem.isWip && rowItem.ListView.view ? rowItem.ListView.view.wipDeleted : 0
                renamed: rowItem.isWip && rowItem.ListView.view ? rowItem.ListView.view.wipRenamed : 0
                copied: rowItem.isWip && rowItem.ListView.view ? rowItem.ListView.view.wipCopied : 0
            }
            Item {
                visible: rowItem.isWip
                Layout.fillWidth: true
            }
        }
    }

    // The box carries on from wherever the last delegate to hold it left off — including a fresh one, when the row is
    // scrolled back into view mid-name.
    onNamingChanged: rowItem.takeNamingFocus()
    Component.onCompleted: rowItem.takeNamingFocus()
    function takeNamingFocus() {
        if (!rowItem.naming || !rowItem.ListView.view)
            return
        chipColumn.takeNamingFocus(rowItem.ListView.view.namingText)
    }

    // ---- what the pointer is on ------------------------------------------------------------------------------------
    //
    // **The row is divided once, and everything the pointer does on it reads that one answer** — the two things a rest
    // opens, and the two menus a right-click opens. The division is the chip column's own edge: on that side of it is
    // the chip, on the other side the commit.
    //
    // It used to be the chip's frame, and the frame is the wrong line to divide on. It is drawn eighteen pixels tall in
    // a row of twenty-eight and stops where the name stops, so the margin around it — inside the column, plainly part
    // of "the branch" to anyone looking — answered with the commit's card instead (2026-08-21 ユーザー報告: ギリギリの余白に
    // 重ねるとコミットが出てしまう). **The column's edge is a line that is actually drawn** (the divider), which is what makes
    // it a boundary a reader can hold. The chip is alone in that column, so opening from the whole of it costs nothing
    // — the exception デザイン規約 §hover のツールチップ「開けるのは的、保つのはその的が属する区画」 now names.

    /// Where along the row the pointer is, or -1 for "not on this row". Written by the area below.
    property real pointerRowX: -1
    /// Which half of the row a point along it falls in: `chip`, `row`, or `""` for neither. **One place**, so a rest
    /// and a right-click cannot disagree about where the boundary was.
    function partAt(px) {
        if (px < 0)
            return ""
        if (chipColumn.hasChip && px < rowItem.labelsW)
            return "chip"
        // The WIP row is not a commit: it has no card and no menu (規約 §hover のツールチップ / §グラフ行の右クリック).
        return rowItem.isWip ? "" : "row"
    }
    readonly property string pointedPart: rowItem.partAt(rowItem.pointerRowX)
    /// The open list is this row's chip's — read back from the page, which owns the card.
    readonly property bool listOnThisChip:
        rowItem.ListView.view ? rowItem.ListView.view.chipListAnchor === chipColumn.chipItem : false
    /// What the row skips while the list stands on it (see the area below) it takes back the moment the list goes, or
    /// the pointer is left where it was last seen and a hand coming back is taken for one that never left.
    onListOnThisChipChanged: {
        if (!rowItem.listOnThisChip && !rowMouse.containsMouse)
            rowItem.pointerRowX = -1
    }

    // Both open on a rest, and neither on landing: a hand crossing the graph passes over every row on the way, and
    // opening where it lands flashes one card out and back per row (2026-08-09 報告. 規約 §hover のツールチップ).
    //
    // **Only one of the two is ever out** (規約: 1 つのポインタが開けるものは 1 つ). Whatever the pointer has left goes now
    // rather than in a beat's time — the beat is for walking into what is open, and what is being left is not it.
    onPointedPartChanged: rowItem.settlePointed()
    function settlePointed() {
        const view = rowItem.ListView.view
        if (!view)
            return
        restDelay.stop()
        if (rowItem.pointedPart !== "chip")
            view.chipCollapseRequested()
        if (rowItem.pointedPart !== "row")
            view.rowHoverRequested(rowItem, false)
        if (rowItem.pointedPart === "chip" && rowItem.listOnThisChip) {
            // Already out, and the hand walked down into it and came back. The rest is the question "did you mean to
            // point at this", and it has been answered — re-hold now, or the settle closes the list at `hoverKeepMs`
            // and the rest opens it again at `tipDelayMs`, which reads as a blink (規約「戻る手は待たせない」).
            rowItem.openPointed()
        } else if (rowItem.pointedPart !== "") {
            restDelay.restart()
        }
    }
    function openPointed() {
        const view = rowItem.ListView.view
        if (!view)
            return
        if (rowItem.pointedPart === "chip")
            view.chipExpandRequested(chipColumn.chipItem.records, chipColumn.chipItem)
        else if (rowItem.pointedPart === "row")
            view.rowHoverRequested(rowItem, true)
    }
    Timer {
        id: restDelay
        interval: Metrics.tipDelayMs
        onTriggered: rowItem.openPointed()
    }

    MouseArea {
        id: rowMouse
        anchors.fill: parent
        anchors.topMargin: -rowItem.topBleed
        // While the box is open the ground it stands on belongs to it: this area is painted over everything in the
        // row, so anything under it would never see a click of its own. **The box's own edge, not the column's** — a
        // box wider than its column reaches into the lanes, and the part of it out there takes presses like the rest
        // of it (the column's edge is still a divider a hand can drag, which is the other way out of a narrow one).
        anchors.leftMargin: rowItem.naming ? Theme.spaceXs + chipColumn.nameBoxWidth : 0
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: mouse => {
            // A press here says where the keyboard is working, so the arrows walk the history from the row that was
            // just picked (規約 §矢印で履歴を辿る). Taken by the list rather than by this row: the delegate is recycled the
            // moment the row scrolls off, and either button is the same claim.
            rowItem.ListView.view.takeKeyboard()
            rowItem.ListView.view.currentIndex = rowItem.index
            rowItem.ListView.view.rowSelected(rowItem.oid_hex)
            // The synthetic WIP row is not a commit, so nothing in the commit menu applies to it.
            if (mouse.button === Qt.RightButton && !rowItem.isWip) {
                // In the chip's half the menu is the named ref's — what the name on screen names — and in the other
                // half the row's. **The same division the hover uses** (`partAt`): one boundary, so the button and the
                // rest cannot answer a point differently. The stacked names under +N take the same right-click on the
                // list the chip unfolds into.
                if (rowItem.partAt(mouse.x) === "chip")
                    rowItem.ListView.view.chipMenuRequested(rowItem.oid_hex, rowItem.labelRecords[0])
                else
                    rowItem.ListView.view.rowMenuRequested(rowItem.oid_hex)
            }
        }
        // Where the row leads: the chip it shows, or — with no branch on it — the offer to put one there. The page
        // decides which.
        onDoubleClicked: mouse => {
            if (mouse.button !== Qt.LeftButton || !rowItem.movable)
                return
            rowItem.ListView.view.rowSwitchRequested(rowItem.oid_hex, rowItem.primaryRecord)
        }
        onPositionChanged: mouse => rowItem.pointerRowX = mouse.x
        onContainsMouseChanged: {
            if (rowMouse.containsMouse) {
                rowItem.pointerRowX = rowMouse.mouseX
                return
            }
            // The list opens *on* the chip, so this area loses the pointer the instant it is drawn — a row under a
            // popup sees no hover at all. That leave says nothing about where the hand went, and answering it takes the
            // list down under the hand that asked for it. **Only the row the list is standing on skips it**, and only
            // while it stands: the hand walking off anywhere else lands on another row, which reports a real point and
            // puts the list away, and the card holds itself once it has the pointer (`RowHoverHost`).
            if (rowItem.listOnThisChip)
                return
            rowItem.pointerRowX = -1
        }
    }
    /// Where the pointer is along the row, so the card can open under it rather than at the row's left edge — a row is
    /// the width of the pane, and its left edge is nowhere near the pointer.
    readonly property real pointerX: rowMouse.mouseX

    // A pooled row is under no pointer, and the row it comes back as has its own names: what this one was pointed at
    // goes with it. Clearing where the pointer was takes the rest and whatever was open down with it (`settlePointed`).
    ListView.onPooled: rowItem.pointerRowX = -1
}
