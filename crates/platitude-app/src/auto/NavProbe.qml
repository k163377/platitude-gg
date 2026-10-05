pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

/// What a headless run does to the sidebar, and what it reads back off it.
///
/// Every input goes in where a hand goes in (verify-ui implement.md「注入はハンドラ本体そのものへ入れる」). Hover
/// cannot be injected, so resting on something writes the one property a real hover writes.
///
/// The pane hands over three children (`autoRail` / `autoSections` / `autoPeek`); composing them into what a verb
/// wants is the harness's.
QtObject {
    id: probe

    required property var sidebar

    readonly property var rail: probe.sidebar.autoRail
    readonly property var sections: probe.sidebar.autoSections
    readonly property var peek: probe.sidebar.autoPeek

    /// PGG_AUTO_ACT=nav-filter: type into the filter field itself.
    function typeFilter(text) {
        probe.sections.filterText = text
        return probe.sections.filterText
    }

    /// PGG_AUTO_ACT=nav-peek: rest on one section's cell. Whether that opens anything is the cell's answer (an empty
    /// section answers no).
    function peekAt(kind) {
        probe.rail.enterAt(kind, probe.rail.topOf(kind))
    }
    /// PGG_AUTO_ACT=nav-peek-away / nav-peek-shut: walk the pointer off the cell that opened it, and click that cell.
    function peekAway(kind) {
        probe.rail.leaveAt(kind)
    }
    function peekTap(kind) {
        probe.rail.tapAt(kind)
    }
    /// PGG_AUTO_ACT=nav-peek-into / nav-peek-out: the pointer walked off the cell down into the open section, and then
    /// out of it the other way. They write the `peekEntered` the popup's own hover writes: a run that cannot say
    /// "inside" cannot tell the exit that closes the peek from the one that keeps it open.
    function peekInto(kind) {
        probe.rail.leaveAt(kind)
        probe.sidebar.peekEntered = true
    }
    function peekOut() {
        probe.sidebar.peekEntered = false
    }

    /// PGG_AUTO_ACT=nav-close: close one section at the header band's own click (`NavHeader.tap`) — setting
    /// `expTags` itself would open a section the band refuses to.
    function closeSection(kind) {
        const head = probe.headOf(kind)
        if (head)
            head.tap()
    }
    function headOf(kind) {
        return probe.sections.headOf(kind)
    }
    /// PGG_AUTO_ACT=nav-add-remote: press the `+` at the end of the REMOTES band, at the band's own signal.
    function tapAddRemote() {
        probe.headOf("remote").addRemoteRequested()
    }
    /// PGG_AUTO_ACT=tags-eye: press the eye at the end of the TAGS band (`NavHeader.tapTags`).
    function tapTagEye() {
        probe.headOf("tag").tapTags()
    }
    /// PGG_AUTO_ACT=nav-close: where a section's header band came to rest, and where the ground under the last
    /// section begins.
    function headerTopOf(kind) {
        const head = probe.headOf(kind)
        return head ? head.y : -1
    }
    readonly property real groundTop: probe.sections.groundTop
    /// PGG_AUTO_ACT=nav-peek: where the open peek begins and ends — at the top edge of the cell that opened it, and
    /// inside the pane whatever the row count.
    readonly property real peekY: probe.peek.y
    readonly property real peekBottom: probe.peek.y + probe.peek.height

    /// `countsInk`'s ruler, given each count's font in turn.
    property FontMetrics countFace: FontMetrics {}
    /// PGG_AUTO_ACT=nav-fold: whether every count on the rail has its ink under its mark's box, centred in the room down
    /// to half a step over the cell's foot (デザイン規約 §左メニューを畳む), with the span the counts' ink takes in their
    /// cells. Read off each laid-out label and the ink of what it says — not the digits the rail seats them by — since
    /// where a line's ink falls is the face's answer. Centred: the room over the ink and the room under it part by no
    /// more than the odd pixel the room gives above, and half a pixel for the label's whole-pixel place.
    function countsInk() {
        const cells = probe.rail.autoCells
        const floor = probe.rail.cellHeight - Theme.spaceXs / 2
        const span = { clear: cells.count > 0, top: Infinity, bottom: -Infinity }
        for (let i = 0; i < cells.count; i++) {
            const cell = cells.itemAt(i)
            if (cell === null) {
                span.clear = false
                continue
            }
            const count = cell.autoCount
            probe.countFace.font = count.font
            const ink = probe.countFace.tightBoundingRect(count.text)
            const top = count.y + count.baselineOffset + ink.y
            const mark = cell.autoMark
            const over = top - (mark.y + mark.height)
            const under = floor - (top + ink.height)
            span.clear = span.clear && over >= 0 && Math.abs(over - under) <= 1.5
            span.top = Math.min(span.top, top)
            span.bottom = Math.max(span.bottom, top + ink.height)
        }
        return span
    }

    /// PGG_AUTO_ACT=nav-tip: rest the pointer on one section's row, through the same `pointedTipRow` the file lists
    /// carry.
    function listOf(kind) {
        return probe.sections.listOf(kind)
    }
    function pointTipAt(kind, row) {
        const list = probe.listOf(kind)
        if (list)
            list.pointedTipRow = row
    }
    /// PGG_AUTO_ACT=nav-open-foot / nav-open-then: where one section's list stands. A run reads it before the hand
    /// arrives, so the give-back is judged against its own reading and not the list's note of it.
    function listContentY(kind) {
        const list = probe.listOf(kind)
        return list ? list.contentY : 0
    }
    /// PGG_AUTO_ACT=nav-open: whether a row has its facts open, and what they say — read off the row itself
    /// (`SidebarPane.rowFactsWords`).
    readonly property bool rowFactsOpen: probe.sidebar.rowFactsOpen
    function rowFactsWords() {
        return probe.sidebar.rowFactsWords()
    }
    /// PGG_AUTO_ACT=nav-open-foot: whether the open row is showing whole, its lines included.
    function rowFactsShown() {
        return probe.sidebar.rowFactsShown()
    }
    /// PGG_AUTO_ACT=nav-open-foot bar: whether the open row had closed the moment the list's bar was taken — read
    /// then, before the run asks for the row again under the held bar.
    property bool shutByBar: false
    /// PGG_AUTO_ACT=nav-open-then / nav-open-foot: what the hand does once the row is open.
    function afterOpen(what, kind, row) {
        if (what === "bar") {
            // The list's own bar taken — the one property a press on it writes (`AutoScrollBar`); the hand stays on
            // the row, as a real one on the bar over it does. Then the row is asked for again under the held bar,
            // which must open nothing (`SidebarRowGestures.openFacts`).
            Hand.heldBar = probe.listOf(kind).ScrollBar.vertical
            probe.shutByBar = !probe.rowFactsOpen
            probe.pointTipAt(kind, -1)
            probe.pointTipAt(kind, row)
            return
        }
        const lines = probe.sidebar.openFactsItem()
        if (what === "rightclick") {
            lines.handClicked(Qt.RightButton, Qt.NoModifier)
            return
        }
        if (what === "rightclick-name") {
            // The row's own line, through the three handlers a right press reaches there in order
            // (`NavItemDelegate.itemMouse`): the press and release answer nothing for it, the click raises the menu.
            const open = probe.listOf(kind).itemAtIndex(row)
            open.linePressed(Qt.RightButton, open.width / 2, open.lineHeight / 2)
            open.lineReleased()
            open.lineClicked(Qt.RightButton, Qt.NoModifier)
            return
        }
        if (what === "sweep" || what === "tap") {
            // The sweep runs the first line from its head: one started mid-word takes half a word. The tap goes in at
            // the foot, under the last line's band: a line that goes somewhere takes clicks end to end
            // (`NavRowFacts.bandOf`), so the row's own click is only what no band covers.
            const line = what === "tap" ? lines.height - 1 : lines.height / 4
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
    /// PGG_AUTO_ACT=nav-drag-open: a press on a **closed** row's own line that starts to move. False while the view
    /// has not built that row.
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
    /// Whether the click on the open lines landed, and what the hand came away with (`SweepPad`).
    function factsCaret() {
        const lines = probe.sidebar.openFactsItem()
        return lines !== null && lines.caretLanded
    }
    function factsTook() {
        const lines = probe.sidebar.openFactsItem()
        return lines === null ? "" : lines.sweptText()
    }
    /// Whether the row is still painted as the one under the hand (an open row wears the wash over all of itself).
    function rowWashLit(kind, row) {
        const list = probe.listOf(kind)
        return !!list && list.rowWashLit(row)
    }
    /// PGG_AUTO_ACT=nav-open / nav-open-tip: the supplement the open row is asking for — a working copy's path
    /// (`NavRowFacts.says`). The ask, not the box: the box follows a rest later, so a run holds the shared instance
    /// against this before reading its line. Empty where the row keeps nothing or no row is open.
    function factsSays() {
        const lines = probe.sidebar.openFactsItem()
        return lines === null ? "" : lines.says
    }
    /// PGG_AUTO_ACT=nav-open-tag `:tip`: the same rest on **one** of those lines, where a tag's row keeps why a
    /// carrier wears a warning (`NavRowFacts.pointLineTip`). False until that line is built.
    function pointFactsLine(row, on) {
        const lines = probe.sidebar.openFactsItem()
        return lines !== null && lines.pointLineTip(row, on)
    }
    /// PGG_AUTO_ACT=tag-line-menu: a right-click on one line of the open row, in at the hand's own handlers
    /// (`NavRowFacts.handPressed` / `handClicked`) at the middle of that line's words — so the line the menu acts on
    /// is the one the press found, as for a real hand. False until the lines are built.
    function factsLineMenu(row) {
        const lines = probe.sidebar.openFactsItem()
        if (lines === null || row >= lines.lines.length)
            return false
        const at = lines.lineWordsMiddle(row)
        if (at.x < 0)
            return false
        lines.handPressed(Qt.RightButton, at.x, at.y)
        lines.handClicked(Qt.RightButton, Qt.NoModifier)
        return true
    }
    /// PGG_AUTO_ACT=nav-open-tag `:name`: the rest on the open row's own name, where a tag's row keeps why that name
    /// wears the warning (`NavRowFacts.pointNameTip`). False until the lines are built.
    function pointFactsName(on) {
        const lines = probe.sidebar.openFactsItem()
        return lines !== null && lines.pointNameTip(on)
    }

    /// PGG_AUTO_ACT=nav-follow: the first of the open lines whose words go somewhere (-1 for none), and where.
    function factsFirstGoing() {
        const lines = probe.sidebar.openFactsItem()
        return lines === null ? -1 : lines.firstGoing()
    }
    function factsGoesTo(line) {
        const lines = probe.sidebar.openFactsItem()
        return lines === null ? null : lines.lineGoesTo(line)
    }
    /// A press that never moved on the middle of that line's words, at the lines' own handlers
    /// (`NavRowFacts.handPressed` …).
    function followFactsLine(line) {
        const lines = probe.sidebar.openFactsItem()
        if (lines === null)
            return false
        const at = lines.lineWordsMiddle(line)
        if (at.x < 0)
            return false
        lines.handPressed(Qt.LeftButton, at.x, at.y)
        lines.handReleased()
        lines.handClicked(Qt.LeftButton, Qt.NoModifier)
        return true
    }
    /// PGG_AUTO_ACT=nav-follow-lit: the pointer resting on that line, and what the block made of it — the band drawn
    /// under the line, and the hand cursor.
    function pointFactsWords(line, on) {
        const lines = probe.sidebar.openFactsItem()
        return lines !== null && lines.pointLineWords(line, on)
    }
    function factsLineAimed(line) {
        const lines = probe.sidebar.openFactsItem()
        return lines !== null && lines.lineAimed(line)
    }
    function factsHand() {
        const lines = probe.sidebar.openFactsItem()
        return lines !== null && lines.handShown
    }

    /// PGG_AUTO_ACT=nav-peek-open: the same rest on a row of the peek — the row that grows is inside a popup, which
    /// has to make room for it without closing.
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
    /// Whether one section's row is on screen, so a run can say the row left before it says what took its place.
    function rowInView(kind, row) {
        const list = probe.listOf(kind)
        return !!list && list.rowInView(row)
    }
    /// The current branch's sticky stand-in under the same pointer (`HeadPinRow`), pointed through the pane's own
    /// `headPinPointed`: while a filter hides its row, that row has no place in the list to point at.
    readonly property bool headPinLit: probe.sections.headPinLit
    readonly property string headPinWords: probe.sections.headPinWords
    /// Whether it is standing, which edge it took, and where the layout put it (PGG_AUTO_ACT=nav-pin-edge).
    readonly property bool headPinShown: probe.sections.headPinShown
    readonly property bool headPinAbove: probe.sections.headPinAbove
    readonly property real headPinY: probe.sections.headPinY
    /// PGG_AUTO_ACT=nav-open `head:<filter>:<row>`: the folded row it sits under, and where that row's seat came
    /// out — read off the row's own geometry, so the placement is not judged by the count it placed itself by.
    readonly property int headPinUnder: probe.sections.headPinUnder
    function headPinSeatY() {
        return probe.sections.headPinSeatY()
    }
    /// PGG_AUTO_ACT=nav-open `head:<filter>:<row>`: click one row of a section (on a folder row, the fold closes) at
    /// the row's own click (`NavList.clickRow`). False while the view has not built that row — a section's rows
    /// arrive on a read of their own.
    function clickRow(kind, row) {
        const list = probe.listOf(kind)
        return !!list && list.clickRow(row)
    }

    /// PGG_AUTO_ACT=nav-pin-edge: jump the branches list to its end, scrolling the current branch's row off — the
    /// other way the stand-in comes on screen (the filter's is `nav-open head:<filter>`). Answers where the list
    /// came to rest (`NavSections.scrollBranchesToEnd`).
    function scrollBranchesToEnd() {
        return probe.sections.scrollBranchesToEnd()
    }
}
