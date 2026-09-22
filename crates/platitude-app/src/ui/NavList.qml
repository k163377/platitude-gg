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
    /// REMOTES only: the configured remote names (`RepoTab.remoteNames`), and which of them this repository sends
    /// pushes to. Held here — the rows are recycled, so neither answer can be theirs to keep
    /// (デザイン規約 §その他の操作).
    property var remoteNames: []
    property string markedRemote: ""
    /// This list holds whatever height the panel it is in has over its rows. Only the one section the folded rail
    /// opens beside itself wants that — it is alone in its panel, so the spare height has nowhere else to go. In the
    /// sidebar proper the ground at the foot of the column takes it, and no section stretches
    /// (SidebarPane).
    property bool stretch: false
    /// The sidebar, which owns the row gestures: which row was clicked last and which is being typed into outlive both
    /// the delegates and this list, and only one row at a time is either, whichever section it sits in. A list with
    /// none (the WIP file list) simply has no gestures — every call below is skipped.
    property var gestures: null
    /// Whether this list's rows open their facts under themselves on a rest (`NavRowFacts`), and the section that
    /// knows which working copy has a branch out (`worktreeHolding`). **The two are asked for separately**: the
    /// working copies' own list opens its rows and has nothing to ask that section, since a row of it *is* a copy.
    /// Off and null in the lists whose rows do not open — the refs a section holds answer by opening, and the
    /// working tree's file lists hold no refs.
    property bool offersFacts: false
    property var worktreesModel: null
    /// The branches' own section, for the rows that open on somebody else's branch: a working copy's row says of the
    /// branch it holds what that branch's own row would (デザイン規約 §左メニューの所作). Null everywhere else.
    property var branchesModel: null
    /// TAGS only: the remote this window's tag rows act on (`RepoTab.defaultRemote`) — **the reading the others
    /// are read against** in what a tag's row opens on, since sending a tag and taking one off a remote both go
    /// there and nowhere else (デザイン規約 §左メニューの所作 の TAGS の段). Empty in every other list.
    property string pushRemote: ""
    /// The row that is holding a seat under itself for something standing over this list, or -1 for none: the row of
    /// the folded folder the current branch is behind, which is where its stand-in belongs (`HeadPinRow.seatedUnder`
    /// / `NavSections`). The row grows by that much at its foot, so the rows below it move down and the stand-in has
    /// the line under the folder that is holding it. Off in every other list here — only BRANCHES has a stand-in.
    property int pinSeatRow: -1
    /// Stands in for the pointer where headless cannot put one, so a row's tooltip — or the absence of one — can be
    /// photographed (PGG_AUTO_ACT=nav-tip). -1 points at no row. The file lists carry the same property on their own
    /// panes (`WipPane` / `DetailsPane`).
    property int pointedTipRow: -1
    /// Where a row's open name box is drawn — outside this list, which clips, because the box is allowed past the
    /// pane's edge when what is in it does not fit (`NavItemDelegate`).
    ///
    /// **Two out.** One out is the column the sections are laid out in, and a layout lays out whatever is parented
    /// into it — a box put there is given the column's own next row (measured: it landed at the foot of the pane).
    /// Two out is what that column fills, which lays nothing out and clips nothing.
    ///
    /// A list whose rows cannot be typed into is left with none: the working tree's file lists have no gestures, so
    /// no row of theirs ever opens a box, and naming a layer for them would only say where a box that never comes
    /// would have gone. Settable, for a surface where the box is better off staying in its seat.
    property Item boxLayer:
        navList.gestures && navList.parent ? navList.parent.parent : null
    /// Where this list's rows begin in that layer: its own place in the column, and the column's in the layer.
    readonly property real boxRowsX: (navList.parent ? navList.parent.x : 0) + navList.x
    readonly property real boxRowsTop: (navList.parent ? navList.parent.y : 0) + navList.y

    signal refActivated(string oidHex)
    signal fileActivated(string bucket, string path, string origPath)
    signal refMenuRequested(string kind, string name, string full, string oidHex)
    /// Right-click on the row a remote itself stands on.
    signal remoteMenuRequested(string name)

    /// What identifies a row across sections and rebuilds. A colon cannot appear in a ref name, and the section prefix
    /// keeps two sections' equal names apart.
    function keyOf(full, name) {
        return navList.kindHint + ":" + (full !== "" ? full : name)
    }

    /// The box has opened on a row: if it is one of this section's, bring it into view. A row can be typed into
    /// without having been clicked — the current branch's sticky row raises the same menu while the real row is
    /// scrolled off (`HeadPinRow`) — and a name changing itself somewhere off screen is a name nobody agreed to.
    /// Watched: only the gestures know when a box opens, and every list they reach is one of these. A row opening
    /// under itself is watched from the same place, for the same reason (`keepOpenRowInView`).
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
            // What is on show, so a row behind a filter or a closed folder answers -1.
            const row = navList.sectionModel.rowOfName(key.substring(head.length))
            if (row >= 0)
                navList.positionViewAtIndex(row, ListView.Contain)
        }
    }

    /// The room the open row grew by, which this section asks for on top of its rows (`Layout.maximumHeight`). Zero
    /// with nothing open — a section is as tall as the rows it holds, and one row of them is taller while it is open.
    readonly property real openRoom: {
        const row = navList.openRow()
        return row === null ? 0 : Math.max(0, row.height - Theme.rowHeight)
    }
    /// Which row that is, -1 with nothing open — what something standing among the rows has to weigh its own place
    /// against, since the room above it is room its place moved by (`NavSections` hands it to `HeadPinRow`). **One
    /// row is open at a time** (`SidebarRowGestures.openKey`), so one number answers for the whole list.
    readonly property int openIndex: {
        const row = navList.openRow()
        return row === null ? -1 : row.index
    }
    /// Where this list was standing when the row opened, and whether it is holding that place. **A row at the foot
    /// opens past the bottom edge**, and what the list gives up to show its lines is the top — the part the reader
    /// has already left behind (デザイン規約 §左メニューの所作). Closing puts it back here, whatever the list did in
    /// between.
    property real openRestY: 0
    property bool openHeld: false
    function keepOpenRowInView() {
        if (navList.gestures === null)
            return
        // Put back what the last open row took, whichever way the key moved: it goes from one row straight to the
        // next when a hand crosses between them, and a place left standing would be given back against a row that
        // never took it.
        if (navList.openHeld)
            navList.contentY = navList.openRestY
        navList.openHeld = false
        if (navList.gestures.openKey === "")
            return
        navList.openRestY = navList.contentY
        navList.openHeld = true
        navList.revealOpenRow()
    }
    /// Stand where the open row's lines are in view — **asked again every time either side of that can have moved**,
    /// because neither is settled when the key arrives: the lines are measured on a layout, so the row grows on the
    /// pass after the one that built them, and the section's own height answers to that growth in turn
    /// (`openRoom`). Written as the place to stand rather than as a scroll to add, so an answer read too early is
    /// corrected rather than kept: a section that was given the room its row grew by gives the scroll back by
    /// itself, and a run of these never adds up to more than one.
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

    /// Smoke hooks (PGG_AUTO_ACT=nav-reclick): a left click on one row, and what that row made of it. Clicks cannot be
    /// injected (verify-ui スキル), so they go in at the row's own answer. `clickRow` says false when the view has not
    /// built that row yet — the delegate arrives on the layout after the model got the rows, and a run that counted
    /// the miss as a press would wait for a gesture nobody made (app-ui.md §UI 自動化の因果性).
    function clickRow(index) {
        const row = navList.itemAtIndex(index)
        if (!row)
            return false
        // Nothing was held down: a run with no pointer has no press to time (`ReclickGesture.click`).
        row.leftClick(Qt.NoModifier)
        return true
    }
    /// Smoke hook (PGG_AUTO_ACT=nav-drag-open): a press on one **closed** row's own line that starts to move — the
    /// gesture that brings the lines out at once and carries straight on into them. It enters the row's own
    /// handlers, so a run cannot go green with that hand-over cut (verify-ui §注入はハンドラ本体そのものへ入れる).
    /// The drag runs along the row's own line to its far side, which is where the name it opened ends. False when
    /// the view has not built that row yet — the same miss `clickRow` reports.
    function dragRow(index) {
        const row = navList.itemAtIndex(index)
        if (!row)
            return false
        const line = Theme.rowHeight / 2
        row.linePressed(Qt.LeftButton, Theme.spaceSm, line)
        row.lineDragged(row.width, line)
        row.lineReleased()
        row.lineClicked(Qt.LeftButton, Qt.NoModifier)
        return true
    }
    /// What a drag over that row's name came away with, and whether the keyboard went with it — read off the field
    /// the open row shows (PGG_AUTO_ACT=nav-drag-open).
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
    /// The box on one row, as drawn and as the question on it wants to be drawn (PGG_AUTO_ACT=nav-branch-box). Neither
    /// is anything a picture answers: an elided placeholder frames like a shorter question. A row the view has not
    /// built answers 0 — the same miss `clickRow` reports as false.
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
    /// Whether the shared tooltip is standing on that row's box (PGG_AUTO_ACT=rename-tag-box). A row the view has not
    /// built answers false — the same miss the rest of these report.
    function rowTipShown(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.editTipShown : false
    }
    /// What the pointed row itself would say, and what it is called (PGG_AUTO_ACT=nav-tip). The row decides and the
    /// shared instance shows, so the two are read apart: a row with nothing to say never reaches the instance. Read
    /// off `hoverText` — **the attached `ToolTip.visible` reads back the instance's own state**, so during the delay
    /// a row that does speak answers false. An empty name says the view has not built that row yet — the same miss
    /// `clickRow` reports as false.
    function rowTipWords(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.hoverText : ""
    }
    function rowNameAt(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.name : ""
    }
    /// What commit that row names, read off the row itself (PGG_AUTO_ACT=nav-jump): where a click on it leads is the
    /// row's own answer, and a run that worked the commit out from the model instead would be asking the question the
    /// click is supposed to answer. Empty on a row the view has not built — and on the one row here that names no
    /// commit, a bare worktree entry.
    function rowOidAt(index) {
        const row = navList.itemAtIndex(index)
        return row ? row.oid_hex : ""
    }
    /// The row that has its facts open in this list, and what they say (PGG_AUTO_ACT=nav-open) — read off the row
    /// itself, so a run cannot go green with the wiring cut. Null and empty where the open row is not in this list.
    function openRow() {
        // A list with no section behind it is one the rail closed (`SectionPeekPopup`): it holds no rows to open.
        if (navList.gestures === null || navList.sectionModel === null)
            return null
        // The key carries the section it was opened in (`keyOf`), so a list asked about another's key answers
        // with nothing rather than with a row of its own that happens to share the name.
        const key = navList.gestures.openKey
        const head = navList.kindHint + ":"
        if (!key.startsWith(head))
            return null
        const row = navList.itemAtIndex(navList.sectionModel.rowOfName(key.substring(head.length)))
        return row && row.factsOpen ? row : null
    }
    function openWords() {
        const row = navList.openRow()
        // **The name the open row shows leads** — the whole of what git knows it by everywhere but WORKTREES, where
        // the row is named by its folder and the path is a line of its own (`path=`). **The two free fields come
        // last**, in this order: what somebody typed when they took a lock, then a path off this machine. A claim is
        // read as one substring (`verify/verbs/nav.rs`), so everything judged has to stand ahead of them.
        // `track=` is the pair the open lines draw, which on a working copy's row is the branch it holds and not
        // the row's own roles (`NavItemDelegate.factsAhead`).
        return row === null ? ""
             : row.factsName + " local=" + row.factsLocal
               + " track=" + (row.factsBranch !== "" ? row.factsAhead : row.ahead)
               + "/" + (row.factsBranch !== "" ? row.factsBehind : row.behind)
               + " held=" + row.factsHeldBy + " up=" + row.factsUpstream + " gone=" + row.factsGone
               + " branch=" + row.factsBranch
               + " state=" + row.factsState
               // The carriers of a tag's name, the reading they are read against and which of them stand apart from
               // it — comma-separated lists, so they sit with the judged fields: a remote is a ref path component
               // and holds neither a space nor a comma (git refuses both).
               + " remotes=" + row.factsRemotes
               + " against=" + row.factsAgainst + " apart=" + row.factsApart
               + " why=" + row.factsWhy + " path=" + row.factsPath
    }
    /// Where the open row and its lines actually landed, for a run that has to see the list make room rather than
    /// take the layout's word for it (PGG_AUTO_ACT=nav-open). `view=` is what the list is showing while they are
    /// there — the pair a row opening at the foot is judged on.
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
    /// Whether the whole of the open row — its own line and the lines under it — is inside what this list shows
    /// (PGG_AUTO_ACT=nav-open-foot). Read off the two geometries rather than off the scroll that was asked for: a
    /// list that recorded a move it never made would answer for itself otherwise. False with nothing open.
    function openShown() {
        const row = navList.openRow()
        if (row === null)
            return false
        return Math.round(row.y) >= Math.round(navList.contentY)
            && Math.round(row.y + row.height) <= Math.round(navList.contentY + navList.height)
    }
    /// The lines themselves, where a sweep takes words from and a press that never moved goes
    /// (PGG_AUTO_ACT=nav-open-then: `NavRowFacts` is what the hand is driven into).
    function openFactsItem() {
        const row = navList.openRow()
        return row === null ? null : row.factsItem
    }
    /// Whether one row is painted as the one under the hand (PGG_AUTO_ACT=nav-open-then): the wash goes out with the
    /// pointer, and covers the whole of a row that has its facts open (`NavItemDelegate.washLit`).
    function rowWashLit(index) {
        const row = navList.itemAtIndex(index)
        return !!row && row.washLit
    }
    /// Where the list has actually scrolled to, and how a run puts a row out of sight to begin with. Read off
    /// `contentY`: a row scrolled away has no delegate, and "there is no delegate" is also what a list that has not
    /// been built yet says.
    function rowInView(index) {
        // The rows above this one, plus the seat any of them is holding for something standing over the list
        // (`pinSeatRow`) — that seat is a row's worth of ground the rows under it begin after.
        const seat = navList.pinSeatRow >= 0 && navList.pinSeatRow < index ? Theme.rowHeight : 0
        const top = index * Theme.rowHeight + seat
        return top >= navList.contentY && top + Theme.rowHeight <= navList.contentY + navList.height
    }
    function scrollToEnd() {
        navList.contentY = Math.max(0, navList.contentHeight - navList.height)
    }
    /// A few rows on, which is how a run puts a row just out of sight (PGG_AUTO_ACT=nav-branch-box:away). A few,
    /// because past the view's own cache the delegate is gone, and a box that went with it proves nothing about the
    /// one rule being read — that a box whose row has left the list goes with it.
    function scrollRows(rows) {
        navList.contentY = Math.min(Math.max(0, navList.contentHeight - navList.height),
                                    navList.contentY + rows * Theme.rowHeight)
    }

    visible: expanded
    Layout.fillWidth: true
    Layout.fillHeight: expanded
    // A section closes on a hairline of ground — just enough to keep its last row off the next header band. Anything
    // thicker reads as a blank row belonging to the section. A top margin is a seat given to something standing over
    // the rows, so the section asks for it on top of them (`NavSections` / `HeadPinRow`); every other list here has
    // none and adds nothing.
    // **A row that opened asks for the room it grew by** — the ceiling is counted in whole rows, so without it the
    // section keeps the height it had and the lines it opened push its own row up under the sticky stand-in
    // (measured: the name went behind it and only the lines showed). A row holding a seat for the stand-in asks for
    // it the same way, and for the same reason: the seat is a row's worth the ceiling does not count.
    Layout.maximumHeight: !expanded ? 0
                          : stretch ? Number.POSITIVE_INFINITY
                          : count * Theme.rowHeight + navList.openRoom
                            + (navList.pinSeatRow >= 0 ? Theme.rowHeight : 0)
                            + navList.topMargin + Theme.borderWidth
    model: sectionModel
    // The pane's own bar, in place of the style's one that `AppListView` hands the graph, the diff and the log.
    verticalBar: PaneScrollBar {}
    delegate: NavItemDelegate {
        id: row
        listWidth: navList.width
        // The card, and the two sections its answers come off.
        opensFacts: navList.offersFacts
        openKey: navList.gestures ? navList.gestures.openKey : ""
        sectionModel: navList.sectionModel
        worktreesModel: navList.worktreesModel
        branchesModel: navList.branchesModel
        pushRemote: navList.pushRemote
        // Both margins of the panel are the one the bar asks for at the right edge, so the rows sit between equal
        // sides; the folds step in by that same value
        // (`NavItemDelegate.rowInset` / デザイン規約 §余白 の左メニューの行の項).
        rowInset: Theme.spaceSm
        nestStep: Theme.spaceSm
        // The seat this row holds under itself for the stand-in, on the one row that holds one.
        // **`>= 0` first**: a delegate the view has put back in its reuse pool reports `index` -1, which is also
        // "no row is holding a seat" — without the guard every pooled row of every list here grows by one
        // (the reading `tipPointedAt` already makes of the same -1).
        pinSeat: navList.pinSeatRow >= 0 && navList.pinSeatRow === row.index ? Theme.rowHeight : 0
        // The list's own place in that layer, so a scroll carries the box along with the row it belongs to.
        boxLayer: navList.boxLayer
        boxRowsX: navList.boxRowsX
        boxRowsY: navList.boxRowsTop - navList.contentY
        boxRowsTop: navList.boxRowsTop
        boxRowsHeight: navList.height
        kindHint: navList.kindHint
        remoteNames: navList.remoteNames
        markedRemote: navList.markedRemote
        pointedTipRow: navList.pointedTipRow
        rowKey: navList.gestures ? navList.keyOf(full, name) : ""
        activeKey: navList.gestures ? navList.gestures.activeKey : ""
        // The gesture itself: the wait a second click opens has to outlive this row, and a list whose rows cannot be
        // typed into has none (`ReclickGesture`).
        reclick: navList.gestures ? navList.gestures.reclick : null
        // How often the hand itself has moved, and whether anybody is counting: these rows grow where they stand, so
        // the light is read again on a hand that moved and never on a layout that did
        // (`NavItemDelegate.syncHover`). The working tree's list hands down neither.
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
        onRefMenuRequested: (name, full, oidHex) => navList.refMenuRequested(navList.kindHint, name, full, oidHex)
        onRemoteMenuRequested: name => navList.remoteMenuRequested(name)
        onRowClicked: {
            if (navList.gestures)
                navList.gestures.noteClick(row.rowKey)
        }
        onFactsAsked: (open, at) => {
            if (!navList.gestures)
                return
            // Named the way a click is remembered (`keyOf`): the section is part of it, or one name carried on
            // both sides of a fetch would open a row in each list.
            if (open)
                navList.gestures.openFacts(row.rowKey, at)
            else
                navList.gestures.closeFacts(row.rowKey)
        }
        onFactsMenuAsked: {
            if (navList.gestures)
                navList.gestures.noteMenuFromFacts()
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
