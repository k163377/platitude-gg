pragma ComponentBehavior: Bound

import QtQuick

/// What a headless run does to the sidebar, and what it reads back off it.
///
/// Every one of these goes in where a hand goes in — the filter field itself, the rail's own `enterAt` /
/// `leaveAt` / `tapAt`, the header band's `tap` — so what answers is the pane's wiring rather than a second copy
/// of it (verify-ui スキル §注入はハンドラ本体そのものへ入れる). Hover is the input that cannot be injected, so
/// resting on something is always "write the one property a real hover writes".
///
/// Here rather than on `SidebarPane`: the pane's part in this is the three children it hands over (`autoRail` /
/// `autoSections` / `autoPeek`) and the one piece of its own state a peek moves (`peekEntered`). Composing them into
/// what a verb wants is the harness's.
QtObject {
    id: probe

    /// The pane itself, for the one state a peek moves.
    required property var sidebar

    readonly property var rail: probe.sidebar.autoRail
    readonly property var sections: probe.sidebar.autoSections
    readonly property var peek: probe.sidebar.autoPeek

    /// PG_AUTO_ACT=nav-filter: type into the filter band. Written into the field itself, so what the sections are
    /// asked is what a typist asks them.
    function typeFilter(text) {
        probe.sections.filterText = text
        return probe.sections.filterText
    }

    /// PG_AUTO_ACT=nav-peek: rest on one section's cell. Named rather than hovered — hover cannot be injected
    /// (verify-ui スキル). Whether that opens anything is the cell's answer: an empty section answers no.
    function peekAt(kind) {
        probe.rail.enterAt(kind, probe.rail.topOf(kind))
    }
    /// PG_AUTO_ACT=nav-peek-away / nav-peek-shut: walk the pointer off the cell that opened it, and click that cell.
    /// All of these go in at the rail, so what the cells decide and report is part of what is being tested
    /// (`NavRail.enterAt`).
    function peekAway(kind) {
        probe.rail.leaveAt(kind)
    }
    function peekTap(kind) {
        probe.rail.tapAt(kind)
    }
    /// PG_AUTO_ACT=nav-peek-into / nav-peek-out: the pointer walked off the cell down into the open section, and then
    /// out of the section the other way (into the diff or the graph) instead of back over the cell. They write the
    /// same `peekEntered` the popup's own hover writes — the leaving and the being-inside are one state, and a
    /// headless run that cannot say "inside" cannot tell the exit that closes it from the one that must not.
    function peekInto(kind) {
        probe.rail.leaveAt(kind)
        probe.sidebar.peekEntered = true
    }
    function peekOut() {
        probe.sidebar.peekEntered = false
    }

    /// PG_AUTO_ACT=nav-close: close one section by putting a click in where the header band takes one — a hook that
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
    /// PG_AUTO_ACT=nav-add-remote: press the `+` at the end of the REMOTES band. It goes in at the band's own signal,
    /// so what answers is the page's wiring and not a second way in.
    function tapAddRemote() {
        probe.headOf("remote").addRemoteRequested()
    }
    /// PG_AUTO_ACT=tags-eye: press the eye at the end of the TAGS band, at the button's own press
    /// (`NavHeader.tapTags`).
    function tapTagEye() {
        probe.headOf("tag").tapTags()
    }
    /// Where a section's header band has come to rest, and where the ground under the last section begins — what
    /// PG_AUTO_ACT=nav-close reads to see the sections packed against the top.
    function headerTopOf(kind) {
        const head = probe.headOf(kind)
        return head ? head.y : -1
    }
    readonly property real groundTop: probe.sections.groundTop
    /// Where the section the folded rail has open begins and ends (PG_AUTO_ACT=nav-peek). The panel is a popup, so a
    /// headless run reads its placement here rather than off the picture: it starts at the top edge of the cell that
    /// opened it and stops inside the pane, whatever the section's row count.
    readonly property real peekY: probe.peek.y
    readonly property real peekBottom: probe.peek.y + probe.peek.height

    /// PG_AUTO_ACT=nav-tip: rest the pointer on one section's row, and read back what that row answers about a
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
    function tipWordsAt(kind, row) {
        const list = probe.listOf(kind)
        return list ? list.rowTipWords(row) : ""
    }
    function tipNameAt(kind, row) {
        const list = probe.listOf(kind)
        return list ? list.rowNameAt(row) : ""
    }
    /// The current branch's sticky stand-in, under the same pointer: it rides the edge its own row went out of, so
    /// resting on that row is resting on this (`HeadPinRow`). What puts the pointer there is the pane's own
    /// `headPinPointed` — the row it stands for has no place in the list at all while a filter hides it, which is
    /// one of the two ways the stand-in is on screen.
    readonly property bool headPinLit: probe.sections.headPinLit
    readonly property string headPinWords: probe.sections.headPinWords

    /// PG_SCROLL_TO=nav-bottom: jump the branches list to its end. The current branch's sticky row only changes edges
    /// under scroll, which a headless run cannot produce otherwise.
    function scrollBranchesToEnd() {
        probe.sections.scrollBranchesToEnd()
    }
}
