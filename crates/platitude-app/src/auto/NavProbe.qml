pragma ComponentBehavior: Bound

import QtQuick

/// What a headless run does to the sidebar, and what it reads back off it.
///
/// Every one of these goes in where a hand goes in — the filter field itself, the rail's own `enterAt` /
/// `leaveAt` / `tapAt`, the header band's `tap` — so what answers is the pane's wiring
/// (verify-ui スキル §注入はハンドラ本体そのものへ入れる). Hover is the input that cannot be injected, so
/// resting on something is always "write the one property a real hover writes".
///
/// A file of its own: the pane's part in this is the three children it hands over (`autoRail` /
/// `autoSections` / `autoPeek`) and the one piece of its own state a peek moves (`peekEntered`). Composing them into
/// what a verb wants is the harness's.
QtObject {
    id: probe

    /// The pane itself, for the one state a peek moves.
    required property var sidebar

    readonly property var rail: probe.sidebar.autoRail
    readonly property var sections: probe.sidebar.autoSections
    readonly property var peek: probe.sidebar.autoPeek

    /// PGG_AUTO_ACT=nav-filter: type into the filter band. Written into the field itself, so what the sections are
    /// asked is what a typist asks them.
    function typeFilter(text) {
        probe.sections.filterText = text
        return probe.sections.filterText
    }

    /// PGG_AUTO_ACT=nav-peek: rest on one section's cell. Named — hover cannot be injected
    /// (verify-ui スキル). Whether that opens anything is the cell's answer: an empty section answers no.
    function peekAt(kind) {
        probe.rail.enterAt(kind, probe.rail.topOf(kind))
    }
    /// PGG_AUTO_ACT=nav-peek-away / nav-peek-shut: walk the pointer off the cell that opened it, and click that cell.
    /// All of these go in at the rail, so what the cells decide and report is part of what is being tested
    /// (`NavRail.enterAt`).
    function peekAway(kind) {
        probe.rail.leaveAt(kind)
    }
    function peekTap(kind) {
        probe.rail.tapAt(kind)
    }
    /// PGG_AUTO_ACT=nav-peek-into / nav-peek-out: the pointer walked off the cell down into the open section, and then
    /// out of the section the other way (into the diff or the graph). They write the
    /// same `peekEntered` the popup's own hover writes — leaving and being inside are one state, and a headless
    /// run that cannot say "inside" cannot tell the exit that closes it from the one that keeps it open.
    function peekInto(kind) {
        probe.rail.leaveAt(kind)
        probe.sidebar.peekEntered = true
    }
    function peekOut() {
        probe.sidebar.peekEntered = false
    }

    /// PGG_AUTO_ACT=nav-close: close one section by putting a click in where the header band takes one — a hook that
    /// set `expTags` itself would be a second answer, and would open a section the band itself refuses to
    /// (`NavHeader.tap`).
    function closeSection(kind) {
        const head = probe.headOf(kind)
        if (head)
            head.tap()
    }
    function headOf(kind) {
        return probe.sections.headOf(kind)
    }
    /// PGG_AUTO_ACT=nav-add-remote: press the `+` at the end of the REMOTES band. It goes in at the band's own signal,
    /// so what answers is the page's wiring.
    function tapAddRemote() {
        probe.headOf("remote").addRemoteRequested()
    }
    /// PGG_AUTO_ACT=tags-eye: press the eye at the end of the TAGS band, at the button's own press
    /// (`NavHeader.tapTags`).
    function tapTagEye() {
        probe.headOf("tag").tapTags()
    }
    /// Where a section's header band has come to rest, and where the ground under the last section begins — what
    /// PGG_AUTO_ACT=nav-close reads to see the sections packed against the top.
    function headerTopOf(kind) {
        const head = probe.headOf(kind)
        return head ? head.y : -1
    }
    readonly property real groundTop: probe.sections.groundTop
    /// Where the section the folded rail has open begins and ends (PGG_AUTO_ACT=nav-peek). The panel is a popup, so a
    /// headless run reads its placement here: it starts at the top edge of the cell that
    /// opened it and stops inside the pane, whatever the section's row count.
    readonly property real peekY: probe.peek.y
    readonly property real peekBottom: probe.peek.y + probe.peek.height

    /// PGG_AUTO_ACT=nav-tip: rest the pointer on one section's row, and read back what that row answers about a
    /// tooltip. Hover cannot be injected (verify-ui スキル), so it goes in at the same `pointedTipRow` the file lists
    /// carry, and what comes out is the row's own attached ToolTip and the shared instance.
    function listOf(kind) {
        return probe.sections.listOf(kind)
    }
    function pointTipAt(kind, row) {
        const list = probe.listOf(kind)
        if (list)
            list.pointedTipRow = row
    }
    /// Where one section's list is standing (PGG_AUTO_ACT=nav-open-foot / nav-open-then): a run remembers this
    /// itself before the hand arrives, so what it holds the give-back against is not the list's own note of it.
    function listContentY(kind) {
        const list = probe.listOf(kind)
        return list ? list.contentY : 0
    }
    /// PGG_AUTO_ACT=nav-open: the facts a branch row opens under itself — whether one is open, and what it says.
    /// Read off the row itself (`SidebarPane.rowFactsWords`), so a run cannot go green with the wiring cut.
    readonly property bool rowFactsOpen: probe.sidebar.rowFactsOpen
    function rowFactsWords() {
        return probe.sidebar.rowFactsWords()
    }
    /// PGG_AUTO_ACT=nav-open-foot: whether the open row is showing whole, its lines included.
    function rowFactsShown() {
        return probe.sidebar.rowFactsShown()
    }
    /// PGG_AUTO_ACT=nav-open-then: what the hand does once the row is open. Every one of these goes in where the hand
    /// goes in — the row's own pointer stand-in, the lines' own handlers, the pane's rename, the filter field — so
    /// what answers is the wiring and not a second copy of it.
    function afterOpen(what, kind, row) {
        // The three the hand makes on the lines themselves: a menu asked for from them, a drag that takes words
        // away, and a press that never moved (`NavRowFacts.handPressed`).
        const lines = probe.sidebar.openFactsItem()
        if (what === "rightclick") {
            lines.handClicked(Qt.RightButton, Qt.NoModifier)
            return
        }
        if (what === "sweep" || what === "tap") {
            // Along the first line, from its head to the far side: a drag that starts mid-word comes away with half
            // of one, which says nothing about whether a name can be taken.
            const line = lines.height / 4
            lines.handPressed(Qt.LeftButton, 0, line)
            if (what === "sweep")
                lines.handMoved(lines.width, line)
            lines.handReleased()
            lines.handClicked(Qt.LeftButton, Qt.NoModifier)
            return
        }
        if (what === "away")
            probe.pointTipAt(kind, -1)
        else if (what === "edit")
            probe.sidebar.beginRename(kind, probe.tipNameAt(kind, row), probe.tipNameAt(kind, row))
        else if (what === "menu")
            probe.sidebar.menuOpen = true
        else if (what === "filter")
            probe.typeFilter("zzzz")
    }
    /// PGG_AUTO_ACT=nav-drag-open: a press on a **closed** row's own line that starts to move. It goes in at the
    /// row's own handlers, which is where a hand goes in, and says whether the view had built that row to press.
    function dragRow(kind, row) {
        const list = probe.listOf(kind)
        return !!list && list.dragRow(row)
    }
    /// What that drag came away with, off the field the open row shows its name in (`NameCell.whole`).
    function rowNameTook(kind, row) {
        const list = probe.listOf(kind)
        return list ? list.rowNameTook(row) : ""
    }
    function rowNameCaret(kind, row) {
        const list = probe.listOf(kind)
        return !!list && list.rowNameCaret(row)
    }
    /// What the hand on those lines came away with, and where the click landed — the pad's own answers
    /// (`SweepPad`), read through the lines that are open.
    function factsCaret() {
        const lines = probe.sidebar.openFactsItem()
        return lines !== null && lines.caretLanded
    }
    function factsTook() {
        const lines = probe.sidebar.openFactsItem()
        return lines === null ? "" : lines.sweptText()
    }
    /// Whether the row is still painted as the one under the hand — a row that has its facts open wears the wash
    /// over the whole of itself.
    function rowWashLit(kind, row) {
        const list = probe.listOf(kind)
        return !!list && list.rowWashLit(row)
    }

    /// PGG_AUTO_ACT=nav-peek-open: the same rest, taken on a row of the section the folded rail has open — the case
    /// where the row that grows is inside a popup, which has to make room for it without taking itself down.
    function pointPeekTipAt(row) {
        probe.peek.pointTipAt(row)
    }
    function peekNameAt(row) {
        return probe.peek.rowNameAt(row)
    }
    readonly property bool peekStanding: probe.peek.opened

    function tipWordsAt(kind, row) {
        const list = probe.listOf(kind)
        return list ? list.rowTipWords(row) : ""
    }
    function tipNameAt(kind, row) {
        const list = probe.listOf(kind)
        return list ? list.rowNameAt(row) : ""
    }
    /// Whether one section's row is on screen. Read off the list,
    /// so a run can say the row left before it says what took its place.
    function rowInView(kind, row) {
        const list = probe.listOf(kind)
        return !!list && list.rowInView(row)
    }
    /// The current branch's sticky stand-in, under the same pointer: it rides the edge its own row went out of, so
    /// resting on that row is resting on this (`HeadPinRow`). What puts the pointer there is the pane's own
    /// `headPinPointed` — the row it stands for has no place in the list at all while a filter hides it, which is
    /// one of the two ways the stand-in is on screen.
    readonly property bool headPinLit: probe.sections.headPinLit
    readonly property string headPinWords: probe.sections.headPinWords
    /// Whether it is standing, which edge it took, and where the layout put it (PGG_AUTO_ACT=nav-pin-edge).
    readonly property bool headPinShown: probe.sections.headPinShown
    readonly property bool headPinAbove: probe.sections.headPinAbove
    readonly property real headPinY: probe.sections.headPinY

    /// PGG_AUTO_ACT=nav-pin-edge: jump the branches list to its end, which is the other of the two ways the stand-in
    /// comes on screen — its row scrolled off (the filter's way is `nav-tip head`). Only
    /// scroll changes which edge it rides, and a headless run has no other way to produce one. The scroll answers
    /// where the list came to rest (`NavSections.scrollBranchesToEnd`).
    function scrollBranchesToEnd() {
        return probe.sections.scrollBranchesToEnd()
    }
}
