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

    /// This row is one of the commits being held (デザイン規約 §複数のコミットを選ぶ). **The choice is the whole of the
    /// highlight wherever there is one** — the current item is only fallen back on where the choice is empty, which is
    /// the working tree's row: that row is not a commit, so it is never in a choice, and it is where a person lands
    /// with nothing chosen.
    readonly property bool selected: rowItem.ListView.view && rowItem.ListView.view.chosenCount > 0
        ? rowItem.ListView.view.chosenOids[rowItem.oid_hex] === true
        : rowItem.ListView.isCurrentItem
    // The top row's highlight and hit area bleed over the list's top margin: hovering or selecting the first commit
    // shows one unbroken band level with the neighbouring header bands instead of leaving a dark sliver above the row.
    readonly property real topBleed: index === 0 && ListView.view ? ListView.view.topMargin : 0
    // The all-zero id marks the synthetic uncommitted-changes row (the sentinel is core's — `Oid::zero_like`).
    readonly property bool isWip: GitFacts.wipOid(oid_hex)
    // **Whose uncommitted row this is.** Every one of them carries the all-zero id — git's own "there is no object
    // here", which is true of all of them — so what tells two apart is the working copy each is about, and the one
    // this window is open on is the one with nothing here. Empty on every commit and on this window's own row.
    //
    // **`carriedRevision` is touched on purpose** (both of these): what answers is a slot call, and a slot call is
    // not made again because the map behind it was rewritten. Without the dependency a row spliced into a rebuilt
    // graph keeps the answer it was given for whatever stood at its index before — observed, this window's own row
    // wearing another copy's tallies.
    readonly property string carriedName: {
        const view = rowItem.ListView.view
        if (!rowItem.isWip || !view || !view.model)
            return ""
        view.model.carriedRevision
        return view.model.carriedName(rowItem.index)
    }
    // A row about a copy this window is not open on. **Read-only**: it opens no pane, takes no selection, and offers
    // no gesture — the pane that stages and commits is opened by a WIP row's selection, so a row that cannot be
    // selected never puts it in front of a tree it does not belong to (P3-確認事項 §別 worktree の未コミット行).
    readonly property bool carried: rowItem.carriedName !== ""
    // Its six tallies, packed by `GraphModel::carried_tally` in the order below.
    readonly property var carriedTally: {
        if (!rowItem.carried)
            return null
        rowItem.ListView.view.model.carriedRevision
        const packed = rowItem.ListView.view.model.carriedTally(rowItem.index)
        return packed === "" ? null : packed.split(",").map(n => parseInt(n, 10))
    }
    // The six an uncommitted row says, whosever it is: another copy's come off its row, and this window's off the
    // view — which holds one set, and it is this window's tree's. Ordered added, modified, deleted, renamed, copied,
    // conflicted, the order both sides are written in.
    readonly property var shownTally: {
        if (rowItem.carriedTally)
            return rowItem.carriedTally
        const view = rowItem.ListView.view
        if (!view)
            return [0, 0, 0, 0, 0, 0]
        return [view.wipAdded, view.wipModified, view.wipDeleted,
                view.wipRenamed, view.wipCopied, view.wipConflicted]
    }
    // Chip records are separated by U+001F (see encode.rs), and arrive in the order the chip reads them out: HEAD →
    // local → remote → tag. One card for the row, so a commit that is both a branch tip and a release shows the branch
    // — the tag is a sheet behind it (`RefChipStack`), read whole in the hover card. A chip the window has already
    // said is gone is left
    // out (`encode::labels_shown` applies the set): the row still carries it, because the ref only leaves the model
    // when the walk that follows the delete lands (デザイン規約 §消す操作は先に画面から消す). Read off the model rather than
    // mirrored onto the view — the same list that hands out `labels` says which names the window stands in for.
    readonly property string goneChips:
        rowItem.ListView.view && rowItem.ListView.view.model ? rowItem.ListView.view.model.goneChips : ""
    readonly property string shownLabels: GitFacts.labelsShown(labels, rowItem.goneChips)
    readonly property var labelRecords:
        rowItem.shownLabels === "" ? [] : rowItem.shownLabels.split(String.fromCharCode(31))
    // The same reading `RepoPage.branchRecordAt` uses, so the two doors to a row's branch cannot disagree: a tag is
    // not a branch, and neither is the detached-HEAD marker.
    readonly property var branchRecords: labelRecords.filter(r => {
        const kind = GitFacts.recordKind(r)
        return kind === "branch" || kind === "remote"
    })
    // Whether this row is somewhere HEAD could stand: the working-tree row is not a commit, and a stash sits on no
    // branch's history.
    readonly property bool movable: !rowItem.isWip && rowItem.stash_ref === ""
    // The commit the working tree is standing on. Asked of the row number the model settled rather than of the chips
    // this row carries: a detached HEAD is the same answer and says so with a different kind (`models::graph::head`).
    readonly property bool isHead: rowItem.ListView.view ? rowItem.ListView.view.headRow === rowItem.index : false
    // Where a double-click on this row goes: the branch chip's own first record, so what is on screen is what is moved
    // to. Empty means the row shows no branch, which is the offer to put one there.
    readonly property string primaryRecord:
        rowItem.movable && rowItem.branchRecords.length > 0 ? rowItem.branchRecords[0] : ""
    // **The name this row draws**: the chip's own first record, whatever kind it is. What the spaced second click
    // changes, and what a right-click aims the menu's cards at — one answer, so the row cannot rename one name and
    // offer another. **Not `primaryRecord`** — that one answers "where does this row lead", and a tag leads nowhere
    // while still being a name that can be changed (デザイン規約 §左メニューの所作). The two markers name no ref — the
    // detached HEAD and a working copy standing here — so they answer `""`.
    readonly property string renameRecord:
        rowItem.labelRecords.length > 0 && GitFacts.recordKind(rowItem.labelRecords[0]) !== ""
            ? rowItem.labelRecords[0] : ""
    // The chip itself — what a stacked one is unstacked under.
    readonly property alias chipItem: chipColumn.chipItem
    // The mark the chip wears while a second click waits out its window, as drawn (PGG_AUTO_ACT=graph-reclick-mark).
    readonly property alias chipWaiting: chipColumn.chipWaiting
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
        visible: rowItem.lit && !rowItem.selected
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
    // last used for: what it holds is the picture it was left with, and the graph may have grown a lane meanwhile.
    // The attached `ListView` only exists on the delegate's root, so the asking is done here, not in the cell.
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
            waiting: rowItem.renameArmed
            listOpen: rowItem.listOnThisChip
            naming: rowItem.naming
            namingMode: rowItem.ListView.view ? rowItem.ListView.view.namingMode : "branch"
            namingKind: rowItem.ListView.view ? rowItem.ListView.view.namingKind : ""
            // Only the row holding the box can be refused: the answer is about what is typed, and one row is typed in.
            namingRefused: rowItem.naming && rowItem.ListView.view
                           ? rowItem.ListView.view.namingRefused : false
            namingRefusedWhy: rowItem.ListView.view ? rowItem.ListView.view.namingRefusedWhy : ""
            onNamingSubmitted: name => rowItem.ListView.view.namingSubmitted(
                rowItem.oid_hex, name, rowItem.ListView.view.namingMode)
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
            // Where the message stops: the pane's own inset, the same the chip column keeps on the other side
            // (デザイン規約 §余白). Without it every row would end in ink against the frame beside it. **Not the bar's
            // gutter** — the thumb reaches twice as far in and is drawn thin enough to read the tail through.
            Layout.rightMargin: Theme.spaceXs
            spacing: Theme.spaceXs
            // The tick goes with the message, not with the lanes: it stands in the subject column, this row's alone.
            opacity: rowItem.dimmed ? Metrics.dimFade : 1
            // Short colored tick before the message: separates rows visually (deliberately not a continuous line) and
            // echoes the commit's chain color. These three steps — this margin, this width, the layout's spacing —
            // are what `GraphPane.subjectTextX` adds up, since that is where the find bar's cap is measured from.
            Rectangle {
                Layout.leftMargin: Theme.spaceSm
                implicitWidth: 2 * Theme.borderWidth
                implicitHeight: Theme.iconMd
                radius: Theme.borderWidth
                color: Theme.graphLane[rowItem.node_color % Theme.graphLane.length]
            }
            // The message, cut at its end and the mark held against the column's right edge, so every row's `…` stands
            // at one x (デザイン規約 §タイポグラフィ). **The stand-in cuts its own at the same x** (`GraphHeadPin`): one
            // commit's message may not change length when its row steps aside for the stand-in.
            CutName {
                // The uncommitted row's words are a fixed length, and the counts read as part of the same sentence: it
                // keeps its own width so they sit right after it rather than out at the pane's far edge.
                Layout.fillWidth: !rowItem.isWip
                cutAt: "end"
                // No total: the tallies beside it add up to exactly that number, and the pane's own heading says it as
                // well (規約 §未コミット行が名乗るもの).
                text: rowItem.isWip ? qsTr("Uncommitted changes") : rowItem.subject
                pixelSize: Theme.fontMd
                // The commit the working tree is standing on writes its message in the branch's own blue — the same
                // `textLink` the chip on it uses, and the same the stand-in uses while this row is scrolled off
                // (規約 §グラフの中で HEAD を見失わない). One rule for the one commit, so nothing changes as it steps aside.
                color: rowItem.isWip ? Theme.textSecondary
                       : rowItem.isHead ? Theme.textLink
                       : Theme.textPrimary
            }
            // **Only on the row they belong to.** A nested layout defaults to `Layout.fillWidth: true`, so left up on
            // every commit row this takes a share of the free space even with all six counts at zero and nothing drawn
            // (67px of a 400px row, measured), which is that much of the message cut off for a column holding nothing.
            //
            // **Built on the uncommitted row alone.** A delegate is built per row on screen, and the tallies are six
            // marks — a canvas each — with a number beside every one; on a commit's row they were built and hidden,
            // which is heap the graph's rows are measured by (rules-refs/app-ui.md, the Loader rule). Invisible while
            // inactive as well: a layout skips an invisible item, and an empty loader would still take the spacing.
            Loader {
                active: rowItem.isWip
                visible: rowItem.isWip
                Layout.leftMargin: Theme.spaceSm
                sourceComponent: WipTallyRow {
                    added: rowItem.shownTally[0]
                    modified: rowItem.shownTally[1]
                    deleted: rowItem.shownTally[2]
                    renamed: rowItem.shownTally[3]
                    copied: rowItem.shownTally[4]
                    conflicted: rowItem.shownTally[5]
                }
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
    // the chip, on the other side the commit. The chip's frame is the wrong line to divide on — it is drawn eighteen
    // pixels tall in a row of twenty-eight and stops where the name stops, so the margin around it, plainly part of
    // "the branch" to anyone looking, would answer with the commit's card. **The column's edge is a line that is
    // actually drawn** (the divider), which is what makes it a boundary a reader can hold, and the chip is alone in
    // that column — the exception デザイン規約 §hover のツールチップ「開けるのは的、保つのはその的が属する区画」 names.

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
    /// The card out is of this row's own commit — read back from the page the same way. **By the commit, not by the
    /// delegate**: delegates travel, and a row that scrolled into this one's place is not the row the card is of.
    readonly property bool cardOnThisRow: rowItem.ListView.view && rowItem.oid_hex !== ""
                                          ? rowItem.ListView.view.rowCardOid === rowItem.oid_hex : false
    /// The row wears the hover band. **The pointer being on it is only one of the ways** — what the pointer opened on
    /// this row holds it up too, for as long as that stands: both popups are drawn over or off the row and take the
    /// pointer off it at once (`RowHoverHost`), and a row gone dark under its own card says nothing about which it is.
    readonly property bool lit: rowMouse.containsMouse || rowItem.cardOnThisRow || rowItem.listOnThisChip

    // Both open on a rest, and neither on landing: a hand crossing the graph passes over every row on the way, and
    // opening where it lands flashes one card out and back per row (規約 §hover のツールチップ). **Only one of the two is
    // ever out** (規約: 1 つのポインタが開けるものは 1 つ). Whatever the pointer has left goes now rather than in a beat's
    // time — the beat is for walking into what is open, and what is being left is not it.
    onPointedPartChanged: rowItem.settlePointed()
    /// A second click is waiting out its window somewhere in this graph. **Nothing hover opens or closes while it
    /// runs**: the reader has clicked and is waiting for the box, and a card that came or went in that beat is a
    /// change they did not ask for. What the pointer did meanwhile is settled the moment the wait ends.
    readonly property bool renameWaiting: rowItem.ListView.view ? rowItem.ListView.view.renameWaiting : false
    onRenameWaitingChanged: if (!rowItem.renameWaiting) rowItem.settlePointed()
    function settlePointed() {
        const view = rowItem.ListView.view
        if (!view || rowItem.renameWaiting)
            return
        restDelay.stop()
        if (rowItem.pointedPart !== "chip")
            view.chipCollapseRequested()
        if (rowItem.pointedPart !== "row")
            view.rowHoverRequested(rowItem, false)
        if (rowItem.pointedPart === "chip" && rowItem.listOnThisChip) {
            // Already out, and the hand walked down into it and came back. The rest asks "did you mean to point at
            // this", and it has been answered — re-hold now, or it closes at `hoverKeepMs` and reopens at
            // `tipDelayMs`, which reads as a blink (規約「戻る手は待たせない」).
            rowItem.openPointed()
        } else if (rowItem.pointedPart !== "") {
            restDelay.restart()
        }
    }
    function openPointed() {
        const view = rowItem.ListView.view
        // The rest that was already running when the second click landed is part of the same beat: it opens what the
        // reader did not ask for, half a second into a wait they are watching (see `renameWaiting`).
        if (!view || rowItem.renameWaiting)
            return
        // Another copy's row has nothing behind it: the card would come up empty, with a stamp of `1970-01-01`
        // (photographed). An empty card is worse than none (P3-確認事項 §別 worktree の未コミット行).
        if (rowItem.carried)
            return
        if (rowItem.pointedPart === "chip")
            view.chipExpandRequested(rowItem.oid_hex, rowItem.index,
                                     chipColumn.chipItem.records, chipColumn.chipItem)
        else if (rowItem.pointedPart === "row")
            view.rowHoverRequested(rowItem, true)
    }
    Timer {
        id: restDelay
        interval: Metrics.tipDelayMs
        onTriggered: rowItem.openPointed()
    }

    /// Whether this row is holding the wait the name box opens after, and whether a click landing now would still be
    /// counted as the other half of a double-click — what a headless run reads to put its second click in as a second
    /// (app-ui.md §UI 自動化の因果性, PGG_AUTO_ACT=graph-reclick). Both are the list's answer: the gesture lives there,
    /// because this delegate is pooled the moment its row scrolls off (`GraphList`).
    readonly property bool renameArmed:
        rowItem.ListView.view ? rowItem.ListView.view.renameArmed(rowItem.renameRecord) : false
    readonly property bool clickGuarded: rowItem.ListView.view ? rowItem.ListView.view.clickGuarded : false
    /// A left click, as this row answers one. `held` is how long the button was down, which is what the gesture takes
    /// off the wait it has left (`ReclickGesture.click`). Named so that a run with no pointer to press with puts its
    /// click in at the row itself rather than at a copy of what the row would have decided.
    function leftClick(held, modifiers) {
        // Another copy's row answers to nothing: no selection, so the pane that stages and commits is never opened
        // in front of a tree this window is not on (P3-確認事項 §別 worktree の未コミット行).
        if (rowItem.carried)
            return
        const mods = modifiers === undefined ? Qt.NoModifier : modifiers
        // **A click that is building a choice is not a click on a name.** The gesture that opens the name box is two
        // plain clicks spaced apart (`ReclickGesture`); a held Ctrl or Shift says the hand is picking commits, and
        // arming the box off it would put a name box on the row that a range just swept through.
        if (mods & (Qt.ControlModifier | Qt.ShiftModifier)) {
            // And it ends one already waiting: the hand went off to pick commits, so coming back to this row later is
            // a first click again, not the second half of a gesture it walked away from.
            rowItem.ListView.view.dropRename()
            rowItem.claimRow(mods)
            return
        }
        // The second click of a double-click is not a click of its own: the first one already did what a click does,
        // and the gesture is the double.
        if (!rowItem.ListView.view.noteClick(rowItem.oid_hex, rowItem.renameRecord, held))
            return
        rowItem.claimRow(Qt.NoModifier)
    }
    /// A double-click, as this row answers one. Named for the same reason `leftClick` is: **the lane column has a
    /// strip of its own over the list** (`GraphLanePan`, up wherever the lanes overflow their column), and where a row
    /// leads has to be decided in one place, or the part under that strip answers differently from the rest.
    /// Answers whether it led anywhere — **the row's own decision, read back**. What is on the other side of it is a
    /// write that lands ticks later, so nothing on screen says at the moment of the press whether the row took the
    /// gesture or turned it down.
    function doubleClick(modifiers) {
        // **Another copy's row opens that copy**, in a tab of its own — the same door the WORKTREES row is
        // (`SidebarRowGestures.activateRow`). The changes are read where they live: this window's panes read this
        // window's tree, and a copy's own tab is the only place its files can be staged and committed as well as
        // read (P3-確認事項 §別 worktree の未コミット行).
        if (rowItem.carried) {
            const path = rowItem.ListView.view.model.carriedPath(rowItem.index)
            if (path !== "")
                rowItem.ListView.view.carriedOpenRequested(path)
            return true
        }
        const mods = modifiers === undefined ? Qt.NoModifier : modifiers
        // **A held double-click is two selection presses, not a double-click** (デザイン規約 §複数のコミットを選ぶ).
        // Qt hands the pair over as a double whatever the hand was holding, and the modifier with it — measured,
        // `tst_moddblclick` — so this is the only place the two can be told apart. What is on the other side of the
        // plain one is `switch`, which moves the working tree and takes uncommitted changes with it: toggling a row
        // out of a choice and back in, quickly, must not be a way to reach it.
        if (mods & (Qt.ControlModifier | Qt.ShiftModifier))
            return false
        // The second click came inside the window after all, so the gesture was the double-click and not the name.
        // Dropped whatever the row leads to — a row that leads nowhere still has to take the box off the wait.
        rowItem.ListView.view.dropRename()
        if (!rowItem.movable)
            return false
        rowItem.ListView.view.rowSwitchRequested(rowItem.oid_hex, rowItem.primaryRecord)
        return true
    }
    /// A press here says where the keyboard is working, so the arrows walk the history from the row that was just
    /// picked (規約 §矢印で履歴を辿る). Taken by the list, not by this row: the delegate is recycled when the row scrolls
    /// off, and either button is the same claim.
    /// **The keyboard comes here even where the read stays put.** A Ctrl click moves only the choice, but the row it
    /// landed on is where the arrows resume and where the next Shift click measures its range from, so the current
    /// item follows every press.
    function claimRow(modifiers) {
        rowItem.ListView.view.takeKeyboard()
        rowItem.ListView.view.currentIndex = rowItem.index
        rowItem.ListView.view.rowSelected(rowItem.oid_hex, rowItem.index,
                                          modifiers === undefined ? Qt.NoModifier : modifiers)
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
        /// When the button went down, so the gesture can take the time it was held off the wait it has left — Qt
        /// measures a double-click press to press, and `clicked` arrives at the release.
        property real pressAt: 0
        onPressed: mouse => {
            if (mouse.button === Qt.LeftButton)
                rowMouse.pressAt = Date.now()
        }
        onClicked: mouse => {
            if (mouse.button !== Qt.RightButton) {
                rowItem.leftClick(Date.now() - rowMouse.pressAt, mouse.modifiers)
                return
            }
            // **A right-click takes the choice down to this row**, whatever was held before it. Every row of the menu
            // below is about one commit, and leaving several highlighted while the menu acts on one is the one way
            // this pane could offer to `cherry-pick` three and pick one (デザイン規約 §複数のコミットを選ぶ).
            rowItem.claimRow(Qt.NoModifier)
            // The synthetic WIP row is not a commit, so nothing in the commit menu applies to it.
            //
            // **The row is not divided here.** Wherever along it the press landed, one menu comes up: its rows are
            // about this commit, and the cards at its foot are about the name the chip is drawing — which is what
            // `renameRecord` already is, the first record the chip reads out (デザイン規約 §グラフ行の右クリック).
            // The hover still reads the division (`partAt`), because the two things a rest opens are two different
            // things; a right-click opens one, so it has nothing to divide. The names behind the card aim the same
            // menu at one of themselves, through the right-click on the list the chip unfolds into.
            if (!rowItem.isWip)
                rowItem.ListView.view.rowMenuRequested(rowItem.oid_hex, rowItem.renameRecord)
        }
        // Where the row leads: the chip it shows, or — with no branch on it — the offer to put one there. The page
        // decides which.
        onDoubleClicked: mouse => {
            if (mouse.button !== Qt.LeftButton)
                return
            rowItem.doubleClick(mouse.modifiers)
        }
        onPositionChanged: mouse => rowItem.pointerRowX = mouse.x
        onContainsMouseChanged: {
            if (rowMouse.containsMouse) {
                rowItem.pointerRowX = rowMouse.mouseX
                return
            }
            // The list opens *on* the chip, so this area loses the pointer the instant it is drawn — a row under a
            // popup sees no hover at all. That leave says nothing about where the hand went, and answering it takes
            // the list down under the hand that asked for it. **Only the row the list is standing on skips it**, and
            // only while it stands: a hand walking off elsewhere lands on another row, which reports a real point.
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
