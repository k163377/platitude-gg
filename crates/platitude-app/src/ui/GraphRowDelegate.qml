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
    /// Everything after the subject, minus the co-author trailers. Only the hover card reads it, but an undeclared role
    /// arrives as `undefined` and stops whatever reads it mid-way.
    required property string body
    required property int node_lane
    required property int node_color
    required property int avatar
    /// A picture this author was given, or empty for the generated pattern. Resolved in Rust onto the row
    /// (models::graph).
    required property string avatar_url
    /// The `Co-authored-by` records (`{name, email, face}`); the first one badges the node (GraphLaneCell), and the
    /// hover card names the rest.
    required property var co_authors
    /// The lane segments the cell draws (`{kind, lane, color, dashed}` — `encode::Lanes`).
    required property var geometry
    /// The chips the row carries (`{kind, name, isHead, hasRemote, hasPr, here, held, locked, remote, key}` —
    /// `encode::Chips`), in the order the front card reads them: HEAD → local → remote → tag.
    required property var labels
    required property string stash_ref
    /// The find bar's line is somewhere in this row (platitude-core::find decides; the model marks it). Only ever true
    /// while a search is on.
    required property bool matched
    // Dimmed because the search passed this row over, and still as available as any other (デザイン規約 §暗く落とした段).
    readonly property bool dimmed: rowItem.ListView.view ? rowItem.ListView.view.findOn && !rowItem.matched : false

    width: ListView.view.width
    height: Theme.graphRowHeight

    /// This row is one of the commits being held (デザイン規約 §複数のコミットを選ぶ). **A non-empty choice is the whole
    /// highlight**; with nothing held, the current item — the working tree's row, where a person lands — is.
    readonly property bool selected: rowItem.ListView.view && rowItem.ListView.view.chosenCount > 0
        ? rowItem.ListView.view.chosenOids[rowItem.oid_hex] === true
        : rowItem.ListView.isCurrentItem
    // The top row's highlight and hit area bleed over the list's top margin: hovering or selecting the first commit
    // shows one unbroken band level with the neighbouring header bands.
    readonly property real topBleed: index === 0 && ListView.view ? ListView.view.topMargin : 0
    // The all-zero id marks the synthetic uncommitted-changes row (the sentinel is core's — `Oid::zero_like`).
    readonly property bool isWip: GitFacts.wipOid(oid_hex)
    // **Whose uncommitted row this is** — every such row carries the same all-zero id. Empty on every commit and on
    // this window's own row.
    //
    // **`carriedRevision` is touched on purpose** (here and in `carriedTally`): a slot call is not made again when the
    // map behind it is rewritten, so without the dependency a row spliced into a rebuilt graph keeps the answer for
    // whatever stood at its index before.
    readonly property string carriedName: {
        const view = rowItem.ListView.view
        if (!rowItem.isWip || !view || !view.model)
            return ""
        view.model.carriedRevision
        return view.model.carriedName(rowItem.index)
    }
    // A row about a copy this window is not open on. **It selects like any other row and opens that copy read-only**
    // (`leftClick`) — the only tree this window can write is its own (`RepoPage.wipWritable`); the copy's writes are
    // in its own tab, which the double-click opens (デザイン規約 §別の作業コピーを読む).
    readonly property bool carried: rowItem.carriedName !== ""
    // Its six tallies (`GraphModel::carried_tally` — `{added, modified, deleted, renamed, copied, conflicted}`), and
    // nothing on every other row.
    readonly property var carriedTally: {
        if (!rowItem.carried)
            return undefined
        rowItem.ListView.view.model.carriedRevision
        return rowItem.ListView.view.model.carriedTally(rowItem.index)
    }
    // The six counts an uncommitted row shows: another copy's off its row, this window's off the view.
    readonly property var shownTally: {
        if (rowItem.carriedTally)
            return rowItem.carriedTally
        const view = rowItem.ListView.view
        if (!view)
            return { "added": 0, "modified": 0, "deleted": 0, "renamed": 0, "copied": 0, "conflicted": 0 }
        return { "added": view.wipAdded, "modified": view.wipModified, "deleted": view.wipDeleted,
                 "renamed": view.wipRenamed, "copied": view.wipCopied, "conflicted": view.wipConflicted }
    }
    // A chip the window has already said is gone is left out (`encode::chips_shown`): the row still carries it until
    // the walk that follows the delete lands (デザイン規約 §消す操作は先に画面から消す).
    readonly property var goneChips:
        rowItem.ListView.view && rowItem.ListView.view.model ? rowItem.ListView.view.model.goneChips : []
    readonly property var labelRecords: GitFacts.chipsShown(labels, rowItem.goneChips)
    // The chips a double-click can lead to (`primaryChip`).
    readonly property var branchRecords: labelRecords.filter(chip => chip.kind === "branch" || chip.kind === "remote")
    // Whether this row is somewhere HEAD could stand: the working-tree row is not a commit, and a stash sits on no
    // branch's history.
    readonly property bool movable: !rowItem.isWip && rowItem.stash_ref === ""
    // The commit the working tree is standing on, detached or not — the row number the model settled
    // (`models::graph::head`).
    readonly property bool isHead: rowItem.ListView.view ? rowItem.ListView.view.headRow === rowItem.index : false
    // Where a double-click on this row goes: the branch chip's own first record, so what is on screen is what is moved
    // to. Null means the row shows no branch, which is the offer to put one there.
    readonly property var primaryChip:
        rowItem.movable && rowItem.branchRecords.length > 0 ? rowItem.branchRecords[0] : null
    // **The name this row draws**: the chip's first record, whatever kind. The spaced second click renames it and a
    // right-click aims the menu's cards at it — one answer, so the row cannot rename one name and offer another.
    // **Not `primaryChip`**: a tag leads nowhere but can still be renamed (デザイン規約 §左メニューの所作). The two
    // markers (the detached HEAD, a working copy standing here) name no ref, so they answer null.
    readonly property var renameChip:
        rowItem.labelRecords.length > 0 && GitFacts.refKind(rowItem.labelRecords[0].kind) !== ""
            ? rowItem.labelRecords[0] : null
    // The chip, sheets and all — what the card it unfolds into stands on (`GraphRowChips.chipItem`).
    readonly property alias chipItem: chipColumn.chipItem
    // The mark the chip wears while a second click waits out its window, as drawn (PGG_AUTO_ACT=graph-reclick-mark).
    readonly property alias chipWaiting: chipColumn.chipWaiting
    // What the name box on this row came out to (see the column — a headless run reads it off here).
    readonly property alias nameBoxWidth: chipColumn.nameBoxWidth
    // This row's chip column is a name box right now.
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

    // A row back from the reuse pool redraws even when no role differs: it holds the picture it was left with, and the
    // graph may have grown a lane meanwhile. The attached `ListView` exists only on the delegate's root, hence here.
    ListView.onReused: {
        laneCell.loadFace()
        laneCell.repaintLanes()
    }

    readonly property real labelsW: ListView.view ? ListView.view.labelWidth : Metrics.labelColW
    /// What a copy's uncommitted row may spend on its words and name (only that row reads it). **Measured off the row
    /// and the view's column widths, not off the message column**: a layout handed more than its cell holds grows past
    /// the pane, so a ceiling read off that column moves out of the way of what it caps. **Every term of the message
    /// column is in it**, as in `GraphPane.subjectTextX`: move one of those and this moves too.
    readonly property real wipRoom:
        rowItem.width - rowItem.labelsW - (ListView.view ? ListView.view.graphColWidth : 0)
        - 2 * Theme.spaceSm - 2 * Theme.borderWidth - 5 * Theme.spaceXs - tallySeat.implicitWidth
    /// What is left of that for the copy's name. **The dash and the mark are measured where they are drawn**
    /// (`furnitureW`), never written down: the dash's width is the font's, the mark's seat its ink (規約 §余白).
    /// **The words never cut**, so the whole overrun is the name's (規約 §未コミット行が名乗るもの).
    readonly property real carriedNameMax:
        rowItem.wipRoom - wordsCut.implicitWidth
        - (carriedSeat.item ? carriedSeat.item.furnitureW : 0)

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
            // Where the message stops: the pane's own inset (デザイン規約 §余白); the bar's thumb reaches further in,
            // thin enough to read the tail through.
            Layout.rightMargin: Theme.spaceXs
            spacing: Theme.spaceXs
            // The tick goes with the message: it stands in the subject column, this row's alone.
            opacity: rowItem.dimmed ? Metrics.dimFade : 1
            // The tick in the commit's lane color. Its margin, its width and the layout's spacing are what
            // `GraphPane.subjectTextX` adds up (the find bar's cap is measured from there): change one, change both.
            Rectangle {
                Layout.leftMargin: Theme.spaceSm
                implicitWidth: 2 * Theme.borderWidth
                implicitHeight: Theme.iconMd
                radius: Theme.borderWidth
                color: Theme.graphLane[rowItem.node_color % Theme.graphLane.length]
            }
            // The message, cut at its end with the `…` held to the column's right edge (デザイン規約 §寸法). **The
            // stand-in cuts at the same x** (`GraphHeadPin`), so a message keeps its length when its row steps aside.
            CutName {
                id: wordsCut
                // The uncommitted row's words keep their own width, so the counts sit right after them and read as
                // part of the same sentence (規約 §未コミット行が名乗るもの).
                Layout.fillWidth: !rowItem.isWip
                cutAt: "end"
                // No total — the tallies add up to it (規約 §未コミット行が名乗るもの). The same words whosever tree it
                // is; whose comes after them, in the seat below.
                text: rowItem.isWip ? qsTr("Uncommitted changes") : rowItem.subject
                pixelSize: Theme.fontMd
                // HEAD's commit writes its message in `textLink`, as the stand-in does while this row is scrolled off
                // (規約 §グラフの中で HEAD を見失わない) — one rule, so nothing changes as it steps aside.
                color: rowItem.isWip ? Theme.textSecondary
                       : rowItem.isHead ? Theme.textLink
                       : Theme.textPrimary
            }
            // Whose tree the words are about, on the rows about somebody else's: a dash, the tree mark, the name
            // (デザイン規約 §未コミット行が名乗るもの).
            //
            // Built only on the rows that wear it: a canvas per graph row is heap the rows are measured by
            // (rules-refs/app-ui.md「行のデリゲートが見せない部品は消す」).
            Loader {
                id: carriedSeat
                active: rowItem.carried
                visible: rowItem.carried
                sourceComponent: RowLayout {
                    spacing: Theme.spaceXs
                    /// The dash and the mark, for `carriedNameMax`.
                    readonly property real furnitureW:
                        carriedDash.implicitWidth + Theme.spaceXs + carriedMark.inkWidth
                    // The dash stays a character — it draws the same length in the same place on both OSes
                    // (規約 §寸法「字で出ていた記号は 5 種」).
                    Label {
                        id: carriedDash
                        text: "—"
                        font.pixelSize: Theme.fontMd
                        color: Theme.textSecondary
                        Layout.alignment: Qt.AlignVCenter
                    }
                    // The mark and the name are one word, seated on the mark's ink
                    // (規約 §余白「印が自分で持っている余白は、隣の詰めに数える」). A nested layout fills by default;
                    // this pair is only as wide as what it holds.
                    RowLayout {
                        Layout.fillWidth: false
                        spacing: 0
                        Item {
                            Layout.preferredWidth: carriedMark.inkWidth
                            Layout.preferredHeight: Theme.iconSm
                            Layout.alignment: Qt.AlignVCenter
                            NavIcon {
                                id: carriedMark
                                anchors.centerIn: parent
                                kind: "tree"
                                // The ink every mark takes: what a mark says is its shape (規約 §ref の種別).
                                tint: Theme.textSecondary
                                width: Theme.iconSm
                                height: Theme.iconSm
                            }
                        }
                        // Cut in the middle, as this copy's name is everywhere (規約 §別の作業コピーを読む).
                        CutName {
                            Layout.maximumWidth: rowItem.carriedNameMax
                            text: rowItem.carriedName
                            pixelSize: Theme.fontMd
                            color: Theme.textSecondary
                        }
                    }
                }
            }
            // **Built on the uncommitted row alone**, for the reason `carriedSeat` is. Invisible while inactive too: a
            // layout skips only invisible items, and an empty loader would still take the spacing
            // (rules-refs の `WipTallyRow` の行).
            Loader {
                id: tallySeat
                active: rowItem.isWip
                visible: rowItem.isWip
                Layout.leftMargin: Theme.spaceSm
                sourceComponent: WipTallyRow {
                    added: rowItem.shownTally.added
                    modified: rowItem.shownTally.modified
                    deleted: rowItem.shownTally.deleted
                    renamed: rowItem.shownTally.renamed
                    copied: rowItem.shownTally.copied
                    conflicted: rowItem.shownTally.conflicted
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
    // **The row is divided once, at the chip column's edge**: the chip on one side, the commit on the other. Not at the
    // chip's frame, which stops where the name stops, so the margin a reader takes for "the branch" would answer with
    // the commit's card (デザイン規約 §hover のツールチップ「開けるのは的、保つのはその的が属する区画」).

    /// Where along the row the pointer is, or -1 for "not on this row". Written by the area below.
    property real pointerRowX: -1
    /// Which half of the row a point along it falls in: `chip`, `row`, or `""` for neither.
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
    /// The card out is of this row's own commit — read back from the page the same way. **By the commit**:
    /// delegates travel, and a row that scrolled into this one's place is not the row the card is of.
    readonly property bool cardOnThisRow: rowItem.ListView.view && rowItem.oid_hex !== ""
                                          ? rowItem.ListView.view.rowCardOid === rowItem.oid_hex : false
    /// The row wears the hover band while the pointer is on it and while what it opened stands — both popups take the
    /// pointer off the row (`RowHoverHost`). **Not for a row the graph brought under a still hand** (`handHeld`).
    readonly property bool lit: (rowMouse.containsMouse && !rowItem.handHeld) || rowItem.cardOnThisRow
                                || rowItem.listOnThisChip

    // **The chip's list opens at once; the row's card opens on a rest** (規約 §hover のツールチップ「展開は即時に開く」
    // 「補足は待ってから開く」), and only one of the two is ever out (「1 つのポインタが開けるものは 1 つ」). Whatever the
    // pointer has left goes now — the beat is for walking into what is open, not out of it.
    onPointedPartChanged: rowItem.settlePointed()
    /// A second click is waiting out its window somewhere in this graph. **Hover is held still while it runs** (規約
    /// §グラフ行のダブルクリック); what the pointer did meanwhile settles when the wait ends.
    readonly property bool renameWaiting: rowItem.ListView.view ? rowItem.ListView.view.renameWaiting : false
    onRenameWaitingChanged: if (!rowItem.renameWaiting) rowItem.settlePointed()
    /// The graph moved under a hand that stayed where it was (`GraphPane.settleUnderHand`): **this row came to the
    /// hand, the hand did not come to it**, so it opens nothing until the hand moves — and then settles what it is
    /// under by then.
    readonly property bool handHeld: rowItem.ListView.view ? rowItem.ListView.view.handHeld : false
    onHandHeldChanged: if (!rowItem.handHeld) rowItem.settlePointed()
    function settlePointed() {
        const view = rowItem.ListView.view
        if (!view || rowItem.renameWaiting)
            return
        restDelay.stop()
        if (rowItem.pointedPart !== "chip")
            view.chipCollapseRequested()
        if (rowItem.pointedPart !== "row")
            view.rowHoverRequested(rowItem, false)
        // What has to go goes all the same; what would open waits for the hand.
        if (rowItem.handHeld)
            return
        if (rowItem.pointedPart === "chip") {
            // Unfolding needs no rest (規約「展開は即時に開く」). A hand coming back out of the list takes this door
            // too, and must: waiting here would let the list close at `hoverKeepMs` under it.
            rowItem.openPointed()
        } else if (rowItem.pointedPart === "row" && rowItem.cardOnThisRow) {
            // The card already out on this commit re-opens at once too: `hoverKeepMs` is shorter than `tipDelayMs`,
            // so waiting would take it down and bring it back wherever the pointer is by then
            // (`RowHoverHost.openRowCard`).
            rowItem.openPointed()
        } else if (rowItem.pointedPart !== "") {
            restDelay.restart()
        }
    }
    function openPointed() {
        const view = rowItem.ListView.view
        // A rest already running when the second click landed opens nothing (`renameWaiting`), nor one on a row the
        // graph brought under a still hand (`handHeld`).
        if (!view || rowItem.renameWaiting || rowItem.handHeld)
            return
        // Another copy's row has no commit behind it: its card would come up empty, stamped `1970-01-01`
        // (規約 §hover のツールチップ「ツールチップが立つのは、足すものが在る的だけ」).
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
    /// counted as the other half of a double-click — what a headless run reads to time its second click
    /// (PGG_AUTO_ACT=graph-reclick). Both are the list's answer: this delegate is pooled when its row scrolls off.
    readonly property bool renameArmed:
        rowItem.ListView.view ? rowItem.ListView.view.renameArmed(rowItem.renameChip) : false
    readonly property bool clickGuarded: rowItem.ListView.view ? rowItem.ListView.view.clickGuarded : false
    /// A left press, as this row answers one — **at the press, not at the release**: answered at the release, the row
    /// sits unlit while the button is down. Named so that a run with no pointer can press the row itself.
    function leftClick(modifiers) {
        // Another copy's row: the page is handed the row number, since every uncommitted row carries the same all-zero
        // id (`RepoPage.openWipFor`). A held modifier chooses nothing — these rows are not commits.
        if (rowItem.carried) {
            rowItem.claimRow(Qt.NoModifier)
            return
        }
        const mods = modifiers === undefined ? Qt.NoModifier : modifiers
        // **A click that is building a choice leaves the name alone**: the name box opens on two plain clicks spaced
        // apart (`ReclickGesture`), and a held Ctrl or Shift is picking commits.
        if (mods & (Qt.ControlModifier | Qt.ShiftModifier)) {
            // And it ends one already waiting: coming back to this row later is a first click again.
            rowItem.ListView.view.dropRename()
            rowItem.claimRow(mods)
            return
        }
        // The second press of a double-click belongs to the gesture: the first one already did what a press does.
        if (!rowItem.ListView.view.noteClick(rowItem.oid_hex, rowItem.renameChip))
            return
        rowItem.claimRow(Qt.NoModifier)
    }
    /// A double-click, as this row answers one — also called from the lane column's own strip over the list
    /// (`GraphLanePan`), so where a row leads is decided in one place. Returns whether it led anywhere: the write
    /// behind it lands ticks later, so nothing on screen says at the press whether the row took the gesture.
    function doubleClick(modifiers) {
        // The second half of a double-click begun on the card that stood here: it moves nothing
        // (`ReclickGesture.hush`).
        if (rowItem.ListView.view.clicksHushed)
            return false
        // **Another copy's row opens that copy in a tab of its own** — the same door as the WORKTREES row
        // (`SidebarRowGestures.activateRow`), and the only place its files can be staged and committed.
        if (rowItem.carried) {
            const path = rowItem.ListView.view.model.carriedPath(rowItem.index)
            if (path !== "")
                rowItem.ListView.view.carriedOpenRequested(path)
            return true
        }
        const mods = modifiers === undefined ? Qt.NoModifier : modifiers
        // **A held double-click is two selection presses** (デザイン規約 §複数のコミットを選ぶ). Qt reports the pair as
        // a double whatever is held, modifier included (`tst_moddblclick`), so this is where they are told apart —
        // toggling a row out of a choice and back must not `switch` the working tree.
        if (mods & (Qt.ControlModifier | Qt.ShiftModifier))
            return false
        // The gesture was the double-click: the name box comes off its wait even on a row that leads nowhere.
        rowItem.ListView.view.dropRename()
        if (!rowItem.movable)
            return false
        rowItem.ListView.view.rowSwitchRequested(rowItem.oid_hex, rowItem.primaryChip)
        return true
    }
    /// A press here says where the keyboard is working (規約 §矢印で履歴を辿る); the list takes it, since the delegate
    /// is recycled when the row scrolls off. **The current item follows every press**, a Ctrl click's included: it is
    /// where the arrows resume and where the next Shift click measures its range from.
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
        // While the name box is open this area starts at the box's own edge: it lies over everything in the row, and a
        // box wider than its column reaches into the lanes and takes presses there too.
        anchors.leftMargin: rowItem.naming ? Theme.spaceXs + chipColumn.nameBoxWidth : 0
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        // **The row keeps the drag it is handed** (`tst_pressorder`): otherwise the list grabs at the drag distance and
        // cancels the press, so a click with a tremor chooses nothing and slides the history.
        preventStealing: true
        // **The left button is answered here**, at the press. The right one waits for its release, which is where
        // every platform opens a context menu.
        onPressed: mouse => {
            if (mouse.button === Qt.LeftButton)
                rowItem.leftClick(mouse.modifiers)
        }
        onClicked: mouse => {
            if (mouse.button !== Qt.RightButton)
                return
            // **A right-click takes the choice down to this row**: the menu acts on one commit, and several left
            // highlighted would offer to act on all of them (デザイン規約 §複数のコミットを選ぶ).
            rowItem.claimRow(Qt.NoModifier)
            // The WIP row is not a commit, so it has no menu. **One menu wherever along the row the press landed**:
            // its rows are about this commit, the cards at its foot about `renameChip` (デザイン規約 §グラフ行の右クリック)
            // — unlike a rest, a right-click has nothing to divide (`partAt`).
            if (!rowItem.isWip)
                rowItem.ListView.view.rowMenuRequested(rowItem.oid_hex, rowItem.renameChip)
        }
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
            // The list opens *on* the chip, so this area loses the pointer the instant it is drawn; answering that
            // leave would take the list down under the hand that asked for it. **Only the row the list stands on skips
            // it**, and only while it stands — a hand walking off lands on another row, which reports a real point.
            if (rowItem.listOnThisChip)
                return
            rowItem.pointerRowX = -1
        }
    }
    /// Where the pointer is along the row, so the card opens under it rather than at the row's left edge.
    readonly property real pointerX: rowMouse.mouseX

    // A pooled row is under no pointer: clearing it takes the rest and whatever was open down with it
    // (`settlePointed`).
    ListView.onPooled: rowItem.pointerRowX = -1
}
