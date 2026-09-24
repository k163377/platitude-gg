pragma ComponentBehavior: Bound

import QtQuick
// For the attached ToolTip alone (rules-refs/app-ui.md carries what an unimported one answers).
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What a row of the left panel opens under itself: one fact to a line, behind the mark that stands for it
// (デザイン規約 §左メニューの所作). **This part draws what it is handed and nothing else** — which lines a section
// opens, and where their words come from, is one table for the three of them (`NavFacts`), so BRANCHES, REMOTES and
// WORKTREES cannot drift apart in the drawing.
//
// **What these lines have of their own is said on a rest anywhere in the open row** — a working copy's path
// (`path`), which is the supplement to what is already open rather than a line of its own
// (デザイン規約 §左メニューの所作). The rest over these lines is one road to it and the rest on the row's own line
// (`rowPointed`) is the other; the box is the same box, so the hand crosses between them without it going down.
//
// **The name is not here.** It stays in the row's own place and is shown whole there (`NameCell.whole`): what the
// reader was looking at does not move when a hand rests on it.
//
// **It is the row growing, not a card over it.** The row is this much taller while it is open and the rows below it
// move down, so the list opens rather than something landing on top of it. The icons stand in the row's own mark
// column and the words begin where its name does, which is what makes the marks look like they moved down here — and
// the row takes its marks off its own line while this is out.
//
// **The press is taken here and split in two**: a drag takes the words away, and a press that never moved is a click.
// A `TextEdit` under a `MouseArea` declared after it never sees a press (measured), so one hand answers for the whole
// of this and drives the sweep itself (`SweepPad`). **The click goes where it landed**: in the band of a line whose
// name stands on a commit of its own, the graph goes to that commit (`NavFactLine.goes`, デザイン規約
// §左メニューの所作); anywhere else — a line naming nothing, one standing where the row does, the foot — it is the
// row's own.
Item {
    id: facts

    /// The row this belongs to — where a press that took no words goes (`NavItemDelegate.rowPressed`), and where a
    /// press in a line's band is handed (`followFact`).
    property Item row: null
    /// The lines to draw, in reading order — **one table answers for every section** (`NavFacts.lines`), so
    /// nothing here knows which of them the row is in. Each is `{mark, markTint, text, tone, ahead, behind, note,
    /// to, withAbove}` — `to` is where a press on it takes the graph, null where it goes nowhere of its own
    /// (`NavFacts.place` / `joined`), and `withAbove` says it shares the band of the line above.
    property var lines: []
    /// WORKTREES only: where this working copy stands — the path git lists it under. **Not a line**: the row is
    /// named by the folder that path ends in, and a reader who has already opened the row is asking about the copy
    /// and not about where it sits on the disk. It comes out as the supplement to what is open
    /// (デザイン規約 §左メニューの所作), which is the one place it answers a question somebody asked.
    /// Empty on every other kind of row.
    property string path: ""
    /// The hand is on the row's own line — the name this supplement is about (`NavItemDelegate.handOn`). **The same
    /// rest, read a line higher**: the reader who hovers a working copy's name is asking what that copy is, and the
    /// path is the one thing neither the row nor these lines show, so it comes out there rather than waiting for
    /// the hand to walk down here.
    property bool rowPointed: false
    /// Which line the hand is on, of those that keep a supplement, or -1 for none — **the line's own answer, held
    /// here** because one pointer opens one thing and the box that opens is this block's (デザイン規約 §hover の
    /// ツールチップ). A line going out clears it only if it is still the one holding it: the hand reaches the next
    /// line before Qt says it left the last.
    property int noteRow: -1
    function noteHand(row, on) {
        if (on)
            facts.noteRow = row
        else if (facts.noteRow === row)
            facts.noteRow = -1
    }
    /// What that line keeps: the reason it is wearing the colour it is (`NavFacts.carrierLine`). Empty wherever the
    /// hand is on a line that has nothing to add — and **the pointer is already in it**, since only a line under
    /// the hand names itself here.
    readonly property string noted:
        facts.noteRow >= 0 && facts.noteRow < facts.lines.length
        ? (facts.lines[facts.noteRow].note || "") : ""
    // **Not cleared when the lines are handed over again.** A row re-gathers its answers while the hand is still on
    // it (`NavItemDelegate.gatherFacts` builds a fresh array every time), and a reset there drops a hand nothing
    // will report again — the supplement then never comes out from under a pointer that never moved (measured: the
    // run waited out its ceiling). What ends the rest is the line saying so, and what ends the row is the row going.
    /// **Beside these lines, never over them** (`SharedToolTip.tipBeside`): what the supplement adds to is a name
    /// the reader is looking at, and every seat inside this panel covers somebody's — the row's own, or a row above
    /// or below it.
    readonly property bool tipBeside: true
    /// Whether the tip standing is **this one's**. **Read off the instance, not off the attached property**: the
    /// shared box is handed from target to target without this attachee hearing a word (`SharedToolTip.handOver`),
    /// so a row that asked its own attached `visible` was told the tip was still its own long after another row had
    /// it — and held itself open behind a hand that had walked off to another section (observed).
    ///
    /// **The row holds itself open on this**: the supplement stands over the panel and takes the pointer with it, so
    /// without it the walk into the tip closes the row it came out of, the lines go, and the tip goes with them
    /// (デザイン規約 §hover のツールチップ「出したものは持ち帰れる」).
    readonly property var tipInstance: facts.ToolTip.toolTip
    readonly property bool tipShown: !!facts.tipInstance && facts.tipInstance.visible
                                     && facts.tipInstance.parent === facts
    /// The hand is on these lines — the whole block's answer, which the row reads as its own hover
    /// (`NavItemDelegate.factsPointed` says why the row cannot read it off its own handler).
    readonly property bool pointed: factsHover.hovered
    /// Whether the press that is ending took words with it. **That is what tells a drag from a click** — a sweep that
    /// came away empty never moved — and a drag must not reach the row, whose second click opens a name box.
    readonly property bool swept: pad.sweptText() !== ""
    /// What a run reads back off that sweep: whether the caret landed in a field, and what came away with it
    /// (`SweepPad`, verify-ui §壊れない動詞の実装).
    readonly property bool caretLanded: pad.caretLanded
    function sweptText() {
        return pad.sweptText()
    }
    /// PGG_AUTO_ACT=nav-open-tag `:tip`: the rest taken on one line of these, where the pointer cannot be put
    /// (verify-ui スキル). It goes in at that line's own stand-in, so what answers is the line's wiring and the
    /// block's — not a second copy of either. False where the lines have not been built.
    function pointLineTip(row, on) {
        const line = lineSeats.itemAt(row)
        if (line === null)
            return false
        line.tipPointed = on
        return true
    }
    /// Nothing under these lines holds a selection any more — what a press on the row's own line above clears
    /// before it decides what that press is (`NavItemDelegate`). One selection in the window.
    function dropSweep() {
        pad.dropValues()
    }

    /// The band a line that goes somewhere is pressed in, in this item's own coordinates — **the target and the
    /// wash are one rect** (デザイン規約 §当たり判定「端に接しないものは判定と塗りを一致させる」). The whole line,
    /// mark to measure, across the whole of this item — which is a gap wider than the lines either side
    /// (`bandReach`), so the mark at the head of the line is not flat against the edge of what it is lit in — and
    /// half the step between lines above and below, so the bands of neighbouring lines meet and a hand moving down
    /// them is always on one. **Never above this item's top**: over it is the row's own line, and a press there is
    /// the row's.
    ///
    /// **Lines going to one commit share one band** (`NavFacts.joined`): it runs from the first of them to the last.
    function bandOf(row) {
        const first = facts.bandStart(row)
        let last = row
        while (last + 1 < facts.lines.length && facts.lines[last + 1].withAbove)
            last++
        const head = first >= 0 && first < lineSeats.count ? lineSeats.itemAt(first) : null
        const foot = last >= 0 && last < lineSeats.count ? lineSeats.itemAt(last) : null
        if (head === null || foot === null)
            return Qt.rect(0, 0, 0, 0)
        const top = Math.max(0, lines.y + head.y - Theme.spaceXs / 2)
        return Qt.rect(0, top, facts.width, lines.y + foot.y + foot.height + Theme.spaceXs / 2 - top)
    }
    /// The first line of the band a line is in — itself, unless it goes where the line above it goes.
    function bandStart(row) {
        let first = row
        while (first > 0 && first < facts.lines.length && facts.lines[first].withAbove)
            first--
        return first
    }
    /// How far this item reaches past its lines on either side — the room the band keeps around the mark and the
    /// measure. **The seat hands this item that much more room than the lines take** (`NavItemDelegate` /
    /// `HeadPinRow`), so the lines stay in the row's own columns and every part of a band is inside this item:
    /// its hover, its press and its paint. A band that reached out past the item by a handler's margin took the
    /// hover off the row's own line under it, and the row closed under a hand still on it (measured,
    /// `tst_openrowholdshand`).
    readonly property real bandReach: Theme.spaceXs
    /// Which line going somewhere has its band under a point, or -1.
    function goingAt(x, y) {
        for (let i = 0; i < facts.lines.length; i++) {
            if (!facts.lines[i].to)
                continue
            const band = facts.bandOf(i)
            if (x >= band.x && x < band.x + band.width && y >= band.y && y < band.y + band.height)
                return i
        }
        return -1
    }
    /// Stands in for the pointer on one line's band where headless cannot put one (PGG_AUTO_ACT=nav-follow-lit), -1
    /// for none.
    property int pointedRow: -1
    /// The line the hand is on, of those that go somewhere, or -1 — the one wearing the band and the underline, and
    /// what turns the pointer to the hand (`factsMouse`). **Read off the place the hand is**, by the reader on this
    /// item (`factsHover`), which is the ancestor of every line and so hears the pointer over all of them.
    readonly property int aimRow: {
        if (facts.pointedRow >= 0)
            return facts.pointedRow
        if (!factsHover.hovered || lineSeats.count === 0)
            return -1
        const p = factsHover.point.position
        return facts.goingAt(p.x, p.y)
    }
    /// Where the press that is ending went down, in this item's own coordinates — what decides where its click goes.
    property point pressFrom: Qt.point(-1, -1)
    /// Where a press at that point goes: the place the line whose band it is in names (`NavFacts.place`), or null
    /// where it is in no such band.
    function goesAt(p) {
        const row = facts.goingAt(p.x, p.y)
        return row < 0 ? null : facts.lines[row].to
    }
    /// The first line whose words go somewhere, or -1 — the one a run presses (PGG_AUTO_ACT=nav-follow), and where it
    /// goes.
    function firstGoing() {
        for (let i = 0; i < facts.lines.length; i++)
            if (facts.lines[i].to)
                return i
        return -1
    }
    function lineGoesTo(row) {
        return row >= 0 && row < facts.lines.length && facts.lines[row].to ? facts.lines[row].to : null
    }
    /// The middle of that line's words, in this item's own coordinates — where a run puts its press.
    function lineWordsMiddle(row) {
        const line = lineSeats.itemAt(row)
        return line === null ? Qt.point(-1, -1) : line.wordsMiddle(facts)
    }
    /// PGG_AUTO_ACT=nav-follow-lit: the pointer resting on that line's band, where it cannot be put. False where the
    /// line has not been built.
    function pointLineWords(row, on) {
        if (lineSeats.itemAt(row) === null)
            return false
        facts.pointedRow = on ? row : -1
        return true
    }
    /// Whether that line is wearing the band — **read off the band that is drawn**, and where it was drawn — and the
    /// pointer the block has turned for it.
    function lineAimed(row) {
        const band = facts.bandOf(row)
        return aimBand.visible && aimBand.y === band.y && aimBand.height === band.height
    }
    readonly property bool handShown: factsMouse.cursorShape === Qt.PointingHandCursor

    /// The hand, in this item's own coordinates: the press anchors a sweep at the nearest field whether it landed on
    /// the words or in the air beside them, the move drags it, and the click goes where it landed — unless the drag
    /// took words with it, in which case the reader was copying and nothing hears it.
    function handPressed(button, x, y) {
        if (button !== Qt.LeftButton)
            return
        facts.pressFrom = Qt.point(x, y)
        pad.pressAt(x, y)
    }
    function handMoved(x, y) {
        pad.moveAt(x, y)
    }
    function handReleased() {
        pad.releaseNow()
    }
    function handClicked(button, modifiers) {
        if (facts.swept || facts.row === null)
            return
        // In the band of a line going somewhere of its own: the graph goes there.
        const to = button === Qt.LeftButton ? facts.goesAt(facts.pressFrom) : null
        if (to !== null) {
            facts.row.followFact(to)
            return
        }
        // **A menu asked for from here is about the row these lines belong to**, so it does not take them down: the
        // reader who right-clicked them is reading about that row, and lines that went out from under the menu would
        // take what it is about off the screen with them (デザイン規約 §左メニューの所作).
        if (button === Qt.RightButton)
            facts.row.factsMenuAsked()
        facts.row.rowPressed(button, modifiers)
    }
    /// **A double-click on a line's way somewhere is two presses of it**, and the first already went: it is not a
    /// double-click on the row, which would move the working tree to a name the hand was not on.
    function handDoubled(button) {
        if (facts.swept || facts.row === null || facts.goesAt(facts.pressFrom) !== null)
            return
        facts.row.rowDoubled(button)
    }

    /// **Nothing at all where no line is drawn**: a row whose whole answer is its own name opens no height, so the
    /// rows under it do not move for it (the name is shown in the row's own place — `NameCell.whole`).
    implicitHeight: lines.implicitHeight > 0 ? lines.implicitHeight + Theme.spaceXs : 0

    // The supplement itself: what the open row keeps for a hand that rests on it, on the same wait every other one
    // of them opens after. **Two kinds and one box** — the whole block's (a working copy's path, which the row's
    // own line asks for as well) and one line's (why that line is wearing a warning). A row never has both: the
    // path rides on WORKTREES rows and the notes on TAGS ones, and one pointer opens one thing either way
    // (デザイン規約 §hover のツールチップ). **The note wins where a row somehow had both** — it is about the line the
    // hand is actually on, and the path is about the row it is anywhere in.
    readonly property string says:
        facts.noted !== "" ? facts.noted
        : (factsHover.hovered || facts.rowPointed) && facts.path !== "" ? facts.path : ""
    /// The words the box is wearing — **the last thing asked for, not what is being asked for now**. The ask falls
    /// the instant the hand leaves for the box, which is the first step of the walk into it, and the attached
    /// property writes `text` straight through to the shared instance while this is its target: bound to the ask,
    /// that write empties the box, and what the walk arrives at is an empty frame (observed — the box put back for
    /// the walk carries whatever the instance holds, `SharedToolTip.reopen`). Every other site in this window is
    /// this shape already; here the two happened to answer to one string.
    ///
    /// **Written before the ask falls**, since a handler runs ahead of the bindings the same property feeds
    /// (rules-refs/app-ui.md) — so the box never opens on the words of the rest before it.
    property string said: ""
    onSaysChanged: if (facts.says !== "") facts.said = facts.says
    ToolTip.text: facts.said
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.visible: facts.says !== ""

    // The hand's ground, under everything drawn here: the pad answers where the fields are, and the hand below drives
    // it (`SweepPad`).
    SweepPad {
        id: pad
        anchors.fill: parent
        content: lines
    }
    // The band the line under the hand wears (`bandOf`), under the words it lights. **Its own look, not the row's
    // wash**: the whole row is already lit while it is open, and a line lit the same way reads as nothing having
    // changed under the hand.
    Rectangle {
        id: aimBand
        readonly property rect at: facts.bandOf(facts.aimRow)
        visible: facts.aimRow >= 0
        x: aimBand.at.x
        y: aimBand.at.y
        width: aimBand.at.width
        height: aimBand.at.height
        radius: Theme.radiusSm
        color: NavFacts.aimFill
        border.width: NavFacts.aimRim.a > 0 ? Theme.borderWidth : 0
        border.color: NavFacts.aimRim
    }
    ColumnLayout {
        id: lines
        // In by the band's reach either side, which the seat added to this item (`bandReach`): the lines stand in
        // the row's own columns.
        anchors.left: parent.left
        anchors.leftMargin: facts.bandReach
        anchors.right: parent.right
        anchors.rightMargin: facts.bandReach
        anchors.top: parent.top
        spacing: Theme.spaceXs

        // One line to a fact, as the table handed them over (`NavFacts.lines`): the mark that stands for it in the
        // row's own mark column, the words beginning where the row's name does, and — where the line names a
        // branch — the measure that branch's own row draws.
        Repeater {
            id: lineSeats
            model: facts.lines
            NavFactLine {
                required property var modelData
                required property int index
                Layout.fillWidth: true
                mark: modelData.mark
                markTint: modelData.markTint
                text: modelData.text
                tone: modelData.tone
                ahead: modelData.ahead
                behind: modelData.behind
                // **Truthiness, not a comparison**: this part takes plain values and is handed them by name
                // (`tests/qml/tst_factstipsteal.qml`), so a line drawn without the field at all has no supplement
                // either — and a handler armed on one would read the pointer for nothing.
                noted: !!modelData.note
                // The same truthiness for where the line goes: a line handed without the field goes nowhere.
                goes: !!modelData.to
                // Every line of the band the hand is on — a band may hold more than one (`bandOf`).
                aimed: facts.aimRow >= 0 && facts.bandStart(facts.aimRow) === facts.bandStart(index)
                // **Each handler is one line into the item's own**, so a run enters this wiring rather than a copy
                // of it (verify-ui スキル §注入はハンドラ本体そのものへ入れる).
                onHandRested: on => facts.noteHand(index, on)
            }
        }
    }
    // The rest over these lines — one of the two roads to the supplement, the row's own line being the other
    // (`rowPointed`). **A handler, not the area below** — handlers are passive, so the hand that sweeps the words
    // goes on being answered by the same area while this reads the same pointer (rules-refs/app-ui.md 「行の hover
    // は `HoverHandler`」). A run has no pointer to rest here (verify-ui スキル): it comes in at the row's own
    // stand-in, which writes the same ask.
    HoverHandler {
        id: factsHover
    }

    // **One hand for the whole of it, and it answers two gestures** — see the head of this file.
    MouseArea {
        id: factsMouse
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        preventStealing: true
        // The words are to be taken everywhere but on a way somewhere, where the press goes before it takes anything.
        cursorShape: facts.aimRow >= 0 ? Qt.PointingHandCursor : Qt.IBeamCursor
        // **Each handler is one line into the item's own** — a run enters those, so what it drives is this wiring
        // rather than a copy of it (verify-ui スキル §注入はハンドラ本体そのものへ入れる).
        onPressed: mouse => facts.handPressed(mouse.button, mouse.x, mouse.y)
        onPositionChanged: mouse => facts.handMoved(mouse.x, mouse.y)
        onReleased: facts.handReleased()
        onCanceled: facts.handReleased()
        onClicked: mouse => facts.handClicked(mouse.button, mouse.modifiers)
        onDoubleClicked: mouse => facts.handDoubled(mouse.button)
    }
}
