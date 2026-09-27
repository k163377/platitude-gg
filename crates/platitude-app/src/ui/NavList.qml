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
    /// REMOTES only: the remote names (`RepoTab.remoteNames`), the one pushes go to, and the one both origin keys
    /// name (`RepoTab.markedOrigin`). Held here, since rows are recycled (デザイン規約 §リモートを書き留める).
    property var remoteNames: []
    property string markedRemote: ""
    property string originRemote: ""
    /// Take the panel's spare height. Only the rail's peek wants it, alone in its panel; in the sidebar the ground
    /// at the column's foot takes it (SidebarPane).
    property bool stretch: false
    /// The sidebar's row gestures, which outlive the delegates and span sections. Null (the WIP file list): no
    /// gestures, and every call below is skipped.
    property var gestures: null
    /// Whether rows open their facts under themselves (`NavRowFacts`), and the section that knows which working copy
    /// has a branch out (`worktreeHolding`). Separate, since the WORKTREES list opens rows but has nothing to ask it.
    property bool offersFacts: false
    property var worktreesModel: null
    /// BRANCHES, for rows that speak of another's branch: a working copy's held branch, and the local branch reading
    /// a remote-tracking one (デザイン規約 §左メニューの所作). Null elsewhere.
    property var branchesModel: null
    /// TAGS only: the remote tag rows act on (`RepoTab.defaultRemote`), which an open tag row reads the others
    /// against (デザイン規約 §左メニューの所作 の TAGS の段).
    property string pushRemote: ""
    /// The row holding a seat under itself for the current branch's stand-in — the folded folder it is behind
    /// (`HeadPinRow.seatedUnder`) — or -1. BRANCHES only.
    property int pinSeatRow: -1
    /// The pointer's stand-in for headless (PGG_AUTO_ACT=nav-tip); -1 points at no row.
    property int pointedTipRow: -1
    /// Where a row's open name box is drawn: outside this list, which clips, so the box can pass the pane's edge.
    /// Two out, not one — the sections' column is a layout and would give a box parented into it a row of its own.
    /// None without gestures (no box ever opens); settable to keep the box in its seat.
    property Item boxLayer:
        navList.gestures && navList.parent ? navList.parent.parent : null
    /// Where this list's rows begin in that layer.
    readonly property real boxRowsX: (navList.parent ? navList.parent.x : 0) + navList.x
    readonly property real boxRowsTop: (navList.parent ? navList.parent.y : 0) + navList.y
    /// Where a row's ink begins and how far each fold steps it in, so names begin in the band's mark column
    /// (デザイン規約 §余白 の左メニューの行の項): the band's seat is `iconSm`, a row's `iconXs`. The stand-in reads these too.
    readonly property int rowInset: Theme.spaceXs + Theme.iconSm - Theme.iconXs
    /// A fold steps in to its chevron's middle: the chevron's box starts where the row's seat does
    /// (`NameCell.seatNudge`) and the chevron is drawn about the box's centre, so the mark box one depth down starts
    /// under that middle.
    readonly property int nestStep: Theme.iconSm / 2

    signal refActivated(string oidHex)
    signal fileActivated(string bucket, string path, string origPath)
    signal refMenuRequested(string kind, string name, string full, string oidHex, string aim)
    /// Right-click on the row a remote itself stands on.
    signal remoteMenuRequested(string name)

    /// A row's key across sections and rebuilds: the section keeps equal names apart, and no ref name holds a colon.
    function keyOf(full, name) {
        return navList.kindHint + ":" + (full !== "" ? full : name)
    }

    /// Bring this section's row into view when its box or its facts open: a row can be typed into while scrolled
    /// off (the sticky `HeadPinRow` raises the same menu), and a name changing off screen is one nobody agreed to.
    Connections {
        target: navList.gestures
        function onOpenKeyChanged() {
            navList.keepOpenRowInView()
        }
        function onEditKeyChanged() {
            const key = navList.gestures.editKey
            const head = navList.kindHint + ":"
            if (!key.startsWith(head))
                return
            // Rows on show only: one behind a filter or a closed folder answers -1.
            const row = navList.sectionModel.rowOfName(key.substring(head.length))
            if (row >= 0)
                navList.positionViewAtIndex(row, ListView.Contain)
        }
    }

    /// The room the open row grew by, asked for on top of the rows (`Layout.maximumHeight`); zero with nothing open.
    readonly property real openRoom: {
        const row = navList.openRow()
        return row === null ? 0 : Math.max(0, row.height - Theme.rowHeight)
    }
    /// Which row that is, or -1: `HeadPinRow` counts the room opened above it (via `NavSections`). One row is open at
    /// a time (`SidebarRowGestures.openKey`).
    readonly property int openIndex: {
        const row = navList.openRow()
        return row === null ? -1 : row.index
    }
    /// Where the list stood when the row opened, put back on close: a row at the foot scrolls the list to show its
    /// lines (デザイン規約 §左メニューの所作).
    property real openRestY: 0
    property bool openHeld: false
    function keepOpenRowInView() {
        if (navList.gestures === null)
            return
        // Restore first: the key can move straight from one row to the next.
        if (navList.openHeld)
            navList.contentY = navList.openRestY
        navList.openHeld = false
        if (navList.gestures.openKey === "")
            return
        navList.openRestY = navList.contentY
        navList.openHeld = true
        navList.revealOpenRow()
    }
    /// Scroll the open row's lines into view. Re-asked whenever the row or the section's height moves — neither is
    /// settled when the key arrives — and written as a place, not a delta, so an early answer is corrected rather
    /// than added to.
    function revealOpenRow() {
        if (!navList.openHeld)
            return
        const row = navList.openRow()
        if (row === null)
            return
        // Never below where the reader left it — a row that opened in full view moves nothing.
        navList.contentY = Math.max(navList.openRestY, row.y + row.height - navList.height)
    }
    onOpenRoomChanged: navList.revealOpenRow()
    onHeightChanged: navList.revealOpenRow()

    /// Smoke hooks (PGG_AUTO_ACT=nav-reclick): a left click on one row, and what that row made of it. `clickRow` is
    /// false while the view has not built the row, so a run does not count the miss as a press.
    function clickRow(index) {
        const row = navList.itemAtIndex(index)
        if (!row)
            return false
        // Nothing was held down: a run with no pointer has no press to time (`ReclickGesture.click`).
        row.leftClick(Qt.NoModifier)
        return true
    }
    /// Smoke hook (PGG_AUTO_ACT=worktree-menu): a right-click on a row, through the row's own press
    /// (`NavItemDelegate.rowPressed`), so the menu it opens is routed the way a hand's is. False as `clickRow`.
    function rightClickRow(index) {
        const row = navList.itemAtIndex(index)
        if (!row)
            return false
        row.rowPressed(Qt.RightButton, Qt.NoModifier)
        return true
    }
    /// Smoke hook (PGG_AUTO_ACT=nav-drag-open): press a closed row's line and drag along it to the far side, through
    /// the row's own handlers (verify-ui implement.md「注入はハンドラ本体そのものへ入れる」). False as `clickRow`.
    function dragRow(index) {
        const row = navList.itemAtIndex(index)
        if (!row)
            return false
        const line = Theme.rowHeight / 2
        // x 0 is left of the name in every section, seated or not (`NavRowBody.seated`).
        row.linePressed(Qt.LeftButton, 0, line)
        row.lineDragged(row.width, line)
        row.lineReleased()
        row.lineClicked(Qt.LeftButton, Qt.NoModifier)
        return true
    }
    /// What a drag over the open row's name took, and whether the keyboard went with it (PGG_AUTO_ACT=nav-drag-open).
    function rowNameTook(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.nameTook : ""
    }
    function rowNameCaret(index) {
        const row = navList.itemAtIndex(index)
        return !!row && row.nameCaret
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
    /// The box on one row, as drawn and as its question wants (PGG_AUTO_ACT=nav-branch-box) — a picture cannot tell
    /// an elided placeholder from a shorter question. 0 on an unbuilt row.
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
    /// Whether the shared tooltip stands on that row's box (PGG_AUTO_ACT=rename-tag-box).
    function rowTipShown(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.editTipShown : false
    }
    /// What the pointed row would say, and its name (PGG_AUTO_ACT=nav-tip). Read off `hoverText`: the attached
    /// `ToolTip.visible` reads back the shared instance, false during the delay. An empty name: the row is unbuilt.
    function rowTipWords(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.hoverText : ""
    }
    function rowNameAt(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.name : ""
    }
    /// The commit a click on that row leads to, read off the row rather than the model (PGG_AUTO_ACT=nav-jump).
    /// Empty on an unbuilt row, a folder, and a bare worktree entry.
    function rowOidAt(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.oid_hex : ""
    }
    /// The row with its facts open in this list, and what they say (PGG_AUTO_ACT=nav-open); null / empty when the
    /// open row is elsewhere.
    function openRow() {
        // No section: the rail's peek has closed (`SectionPeekPopup`).
        if (navList.gestures === null || navList.sectionModel === null)
            return null
        // The key names its section (`keyOf`), so another list's key finds nothing here.
        const key = navList.gestures.openKey
        const head = navList.kindHint + ":"
        if (!key.startsWith(head))
            return null
        const row = navList.itemAtIndex(navList.sectionModel.rowOfName(key.substring(head.length)))
        return row && row.factsOpen ? row : null
    }
    function openWords() {
        const row = navList.openRow()
        // The free fields (`why=` lock reason, then `path=`) come last: a claim is read as one substring
        // (`verify/verbs/nav.rs`), so everything judged stands ahead of them. `track=` is what the lines draw — on
        // a working copy's row, the branch it holds (`NavItemDelegate.factsAhead`).
        return row === null ? ""
             : row.factsName + " local=" + row.factsLocal
               + " track=" + (row.factsBranch !== "" ? row.factsAhead : row.ahead)
               + "/" + (row.factsBranch !== "" ? row.factsBehind : row.behind)
               + " held=" + row.factsHeldBy + " up=" + row.factsUpstream + " gone=" + row.factsGone
               + " branch=" + row.factsBranch
               + " state=" + row.factsState
               // A tag's carriers: comma lists, safe among the judged fields since git refuses a space or comma in
               // a remote's name.
               + " remotes=" + row.factsRemotes
               + " against=" + row.factsAgainst + " apart=" + row.factsApart
               // The copy here, weighed by the same test: what the row's own name wears. Then who the right reading
               // belongs to — empty where the remotes disagree and nobody decides.
               + " here_apart=" + row.factsHereApart + " by=" + row.factsBy
               + " why=" + row.factsWhy + " path=" + row.factsPath
    }
    /// Where the open row and its lines landed (PGG_AUTO_ACT=nav-open); `view=` is the list's visible span.
    function openGeom() {
        const row = navList.openRow()
        if (row === null)
            return "none"
        const lines = row.factsItem
        const at = lines ? lines.mapToItem(row, 0, 0) : null
        return Math.round(row.y) + "+" + Math.round(row.height) + " line=" + Math.round(row.lineHeight)
             + " lines=" + (at ? Math.round(at.y) + "+" + Math.round(lines.height) : "none")
             + " view=" + Math.round(navList.contentY) + "+" + Math.round(navList.height)
             + " room=" + Math.round(navList.openRoom)
    }
    /// Whether the whole open row is inside what the list shows (PGG_AUTO_ACT=nav-open-foot) — read off geometry,
    /// not the scroll asked for.
    function openShown() {
        const row = navList.openRow()
        if (row === null)
            return false
        return Math.round(row.y) >= Math.round(navList.contentY)
            && Math.round(row.y + row.height) <= Math.round(navList.contentY + navList.height)
    }
    /// The open lines (`NavRowFacts`) a run drives the hand into (PGG_AUTO_ACT=nav-open-then).
    function openFactsItem() {
        const row = navList.openRow()
        return row === null ? null : row.factsItem
    }
    /// Whether a row wears the hover wash (PGG_AUTO_ACT=nav-open-then; `NavItemDelegate.washLit`).
    function rowWashLit(index) {
        const row = navList.itemAtIndex(index)
        return !!row && row.washLit
    }
    /// Where the list has scrolled to, and how a run puts a row out of sight. Read off `contentY`, not a delegate:
    /// a scrolled-away row and an unbuilt list both have none.
    function rowInView(index) {
        const seat = navList.pinSeatRow >= 0 && navList.pinSeatRow < index ? Theme.rowHeight : 0
        const top = index * Theme.rowHeight + seat
        return top >= navList.contentY && top + Theme.rowHeight <= navList.contentY + navList.height
    }
    function scrollToEnd() {
        navList.contentY = Math.max(0, navList.contentHeight - navList.height)
    }
    /// A few rows on (PGG_AUTO_ACT=nav-branch-box:away) — within the view's cache, so the delegate survives and its
    /// box has to hide by its own rule.
    function scrollRows(rows) {
        navList.contentY = Math.min(Math.max(0, navList.contentHeight - navList.height),
                                    navList.contentY + rows * Theme.rowHeight)
    }

    visible: expanded
    Layout.fillWidth: true
    Layout.fillHeight: expanded
    // The rows plus what they do not count: the open row's growth and the stand-in's seat (without them the lines
    // push the row up under the sticky stand-in), the top margin, and a hairline (thicker reads as a blank row).
    Layout.maximumHeight: !expanded ? 0
                          : stretch ? Number.POSITIVE_INFINITY
                          : count * Theme.rowHeight + navList.openRoom
                            + (navList.pinSeatRow >= 0 ? Theme.rowHeight : 0)
                            + navList.topMargin + Theme.borderWidth
    model: sectionModel
    verticalBar: PaneScrollBar {}
    delegate: NavItemDelegate {
        id: row
        listWidth: navList.width
        opensFacts: navList.offersFacts
        openKey: navList.gestures ? navList.gestures.openKey : ""
        sectionModel: navList.sectionModel
        worktreesModel: navList.worktreesModel
        branchesModel: navList.branchesModel
        pushRemote: navList.pushRemote
        rowInset: navList.rowInset
        nestStep: navList.nestStep
        // `>= 0` first: a pooled delegate reports `index` -1, and every pooled row would grow by a seat.
        pinSeat: navList.pinSeatRow >= 0 && navList.pinSeatRow === row.index ? Theme.rowHeight : 0
        // So a scroll carries the box with its row.
        boxLayer: navList.boxLayer
        boxRowsX: navList.boxRowsX
        boxRowsY: navList.boxRowsTop - navList.contentY
        boxRowsTop: navList.boxRowsTop
        boxRowsHeight: navList.height
        kindHint: navList.kindHint
        remoteNames: navList.remoteNames
        markedRemote: navList.markedRemote
        originRemote: navList.originRemote
        pointedTipRow: navList.pointedTipRow
        rowKey: navList.gestures ? navList.keyOf(full, name) : ""
        activeKey: navList.gestures ? navList.gestures.activeKey : ""
        // The gestures' own: the wait for a second click outlives this row (`ReclickGesture`).
        reclick: navList.gestures ? navList.gestures.reclick : null
        // Hover is re-read when the hand moves, never when the layout does — these rows grow in place
        // (`NavItemDelegate.syncHover`).
        handCounted: navList.gestures !== null
        handMoves: navList.gestures ? navList.gestures.handMoves : 0
        editKey: navList.gestures ? navList.gestures.editKey : ""
        menuStanding: navList.gestures ? navList.gestures.menuOpen : false
        editMode: navList.gestures ? navList.gestures.editMode : ""
        editText: navList.gestures ? navList.gestures.editText : ""
        editRefused: navList.gestures ? navList.gestures.editRefused : false
        editRefusedWhy: navList.gestures ? navList.gestures.editRefusedWhy : ""
        onRefClicked: oidHex => navList.refActivated(oidHex)
        onFileClicked: (bucket, path, origPath) => navList.fileActivated(bucket, path, origPath)
        onFolderClicked: key => navList.sectionModel.toggleFolder(key)
        onRefMenuRequested: (name, full, oidHex, aim) =>
            navList.refMenuRequested(navList.kindHint, name, full, oidHex, aim)
        onRemoteMenuRequested: name => navList.remoteMenuRequested(name)
        onRowClicked: {
            if (navList.gestures)
                navList.gestures.noteClick(row.rowKey)
        }
        onFactsAsked: (open, at) => {
            if (!navList.gestures)
                return
            if (open)
                navList.gestures.openFacts(row.rowKey, at)
            else
                navList.gestures.closeFacts(row.rowKey)
        }
        onFactsMenuAsked: {
            if (navList.gestures)
                navList.gestures.noteMenuFromFacts()
        }
        onFactsFollowed: (key, oidHex) => {
            if (navList.gestures)
                navList.gestures.followLine(key, oidHex)
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
