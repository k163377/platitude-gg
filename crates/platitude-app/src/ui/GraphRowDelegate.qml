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
    // Where a double-click on this row goes: the branch chip's own first record, so what is on screen is what is moved
    // to. Empty means the row shows no branch, which is the offer to put one there.
    readonly property string primaryRecord:
        rowItem.movable && rowItem.branchRecords.length > 0 ? rowItem.branchRecords[0] : ""
    // The chip itself — what a stacked one is unstacked under.
    readonly property alias chipItem: chipColumn.chipItem
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
                color: rowItem.isWip ? Theme.textSecondary : Theme.textPrimary
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

    // Which stacked chip the pointer is over, if it is over one that has something to unstack. Worked out from the
    // row's own coordinates rather than a hover area inside the chip: this one is on top, so it is the one that hears
    // about the pointer at all.
    property Item hoveredChip: null
    /// This row's chip is the one with the list open under it.
    property bool chipHeld: false
    function chipUnder(px, py) {
        const p = rowItem.mapToItem(chipColumn, px, py)
        return chipColumn.chipAt(p.x, p.y, rowItem.chipHeld)
    }
    // Opens on a rest, the way the row's own card does, and closes with the pointer. Not on landing: a pointer crossing
    // the chip column on its way somewhere passes over every stacked chip on the way, and opening on landing flashes
    // each one's list out and back (2026-08-09 報告).
    //
    // The row's own card gives way to it: the two open off the same pointer and land in the same place, and the chip is
    // the more particular thing to be standing on (デザイン規約 §hover のツール チップ). Stepping off the chip onto the rest of the
    // row offers the card again from the beginning.
    function noteChipHover(px, py) {
        const chip = rowItem.naming ? null : rowItem.chipUnder(px, py)
        if (chip === rowItem.hoveredChip || !rowItem.ListView.view)
            return
        rowItem.hoveredChip = chip
        if (chip) {
            hoverDelay.stop()
            rowItem.ListView.view.rowHoverRequested(rowItem, false)
            if (rowItem.ListView.view.chipListAnchor === chip) {
                // The list this chip opened is still out: the hand walked down into it and came back up. The rest is a
                // question about opening, and nothing is being opened — re-hold now, or the settle closes the list at
                // `hoverKeepMs` and the rest reopens it at `tipDelayMs`, which reads as a blink (規約 §hover
                // のツールチップ「戻る手は待たせない」).
                chipDelay.stop()
                rowItem.chipHeld = true
                rowItem.ListView.view.chipExpandRequested(chip.records, chip)
            } else {
                chipDelay.restart()
            }
        } else {
            chipDelay.stop()
            rowItem.chipHeld = false
            rowItem.ListView.view.chipCollapseRequested()
            if (rowMouse.containsMouse && !rowItem.isWip)
                hoverDelay.restart()
        }
    }
    Timer {
        id: chipDelay
        interval: Metrics.tipDelayMs
        onTriggered: {
            if (!rowItem.hoveredChip || !rowItem.ListView.view)
                return
            rowItem.chipHeld = true
            rowItem.ListView.view.chipExpandRequested(rowItem.hoveredChip.records, rowItem.hoveredChip)
        }
    }

    MouseArea {
        id: rowMouse
        anchors.fill: parent
        anchors.topMargin: -rowItem.topBleed
        // While the box is open the chip column belongs to it: this area is painted over everything in the row, so
        // anything under it would never see a click of its own.
        anchors.leftMargin: rowItem.naming ? rowItem.labelsW : 0
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
                // On the chip the menu is the named ref's — what the name on screen names — and everywhere else the
                // row's. The stacked names under +N take the same right-click on the list the chip unfolds into.
                const chip = chipColumn.chipItem
                const p = rowItem.mapToItem(chip, mouse.x, mouse.y)
                if (chip.visible && chip.contains(Qt.point(p.x, p.y)))
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
        onPositionChanged: mouse => rowItem.noteChipHover(mouse.x, mouse.y)
        onContainsMouseChanged: {
            if (!rowMouse.containsMouse)
                rowItem.noteChipHover(-1, -1)
        }
    }
    /// Where the pointer is along the row, so the card can open under it rather than at the row's left edge — a row is
    /// the width of the pane, and its left edge is nowhere near the pointer.
    readonly property real pointerX: rowMouse.mouseX

    // Hover details: who wrote it, when, and whoever they credited. The row reports; the page decides, because the card
    // outlives this delegate (it is recycled the moment the row scrolls off).
    //
    // The WIP row opens nothing: it has no commit behind it, and its count is already in its own label (規約 §hover
    // のツールチップ).
    Timer {
        id: hoverDelay
        interval: Metrics.tipDelayMs
        onTriggered: {
            if (rowMouse.containsMouse && rowItem.ListView.view)
                rowItem.ListView.view.rowHoverRequested(rowItem, true)
        }
    }
    onIsWipChanged: hoverDelay.stop()
    // A pooled row is under no pointer, and the row it comes back as has its own chips: anything this one was holding
    // goes with it.
    ListView.onPooled: {
        hoverDelay.stop()
        chipDelay.stop()
        rowItem.hoveredChip = null
        rowItem.chipHeld = false
    }
    Connections {
        target: rowMouse
        function onContainsMouseChanged() {
            if (rowItem.isWip)
                return
            if (rowMouse.containsMouse) {
                hoverDelay.restart()
            } else {
                hoverDelay.stop()
                if (rowItem.ListView.view)
                    rowItem.ListView.view.rowHoverRequested(rowItem, false)
            }
        }
    }
}
