pragma ComponentBehavior: Bound

import QtQuick
// For the attached ToolTip alone (rules-refs/app-ui.md carries what an unimported one answers).
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What a row of the left panel opens under itself: one fact to a line, behind its mark (デザイン規約 §左メニューの所作).
// This part draws what it is handed — which lines a section opens is one table for every section (`NavFacts`).
// It is the row growing, not a card over it: the rows below move down, and the name stays whole in the row's own
// place (`NameCell.whole`).
//
// **The press is taken here and split in two**: a drag takes the words away, and a press that never moved is a click.
// A `TextEdit` under a `MouseArea` declared after it never sees a press, so one hand answers for the whole of this and
// drives the sweep itself (`SweepPad`). A click in the band of a line naming a commit of its own goes there
// (`NavFactLine.goes`); anywhere else it is the row's own.
Item {
    id: facts

    /// The row this belongs to — where a press that took no words goes (`NavItemDelegate.rowPressed`), and where a
    /// press in a line's band is handed (`followFact`).
    property Item row: null
    /// The lines to draw, in reading order (`NavFacts.lines`). Each is `{mark, markTint, text, tone, ahead, behind,
    /// note, to, withAbove}` — `to` is where a press on it takes the graph, null where it goes nowhere of its own
    /// (`NavFacts.place`), and `withAbove` says it shares the band of the line above (`NavFacts.joined`).
    property var lines: []
    /// WORKTREES only: the working copy's path — said as the supplement, not as a line (デザイン規約 §左メニューの所作).
    /// Empty on every other kind of row.
    property string path: ""
    /// The hand is on the row's own line (`NavItemDelegate.handOn`) — the other road to the same supplement: hovering
    /// the copy's name asks for its path too. One box for both, so the hand crosses between them without it going down.
    property bool rowPointed: false
    /// Which line the hand is on, of those that keep a supplement, or -1 — held here because the box is this
    /// block's. A line going out clears it only while it still holds it: the hand reaches the next line before Qt
    /// says it left the last.
    property int noteRow: -1
    function noteHand(row, on) {
        if (on)
            facts.noteRow = row
        else if (facts.noteRow === row)
            facts.noteRow = -1
    }
    /// What that line keeps: why it wears the colour it does (`NavFacts.carrierLine`); empty where it has nothing
    /// to add.
    readonly property string noted:
        facts.noteRow >= 0 && facts.noteRow < facts.lines.length
        ? (facts.lines[facts.noteRow].note || "") : ""
    // **`noteRow` is not cleared when the lines are handed over again**: a row re-gathers them while the hand is still
    // on it (`NavItemDelegate.gatherFacts`), and nothing reports that hand again, so the supplement would never come
    // out. Only the line going out, or the row going, ends it.
    /// **Beside these lines, never over them** (`SharedToolTip.tipBeside`): every seat inside the panel covers some
    /// row's name.
    readonly property bool tipBeside: true
    /// Whether the tip standing is **this one's**. **Read off the instance, not off the attached `visible`**: the
    /// shared box is handed between targets without this attachee hearing of it (`SharedToolTip.handOver`), and the
    /// attached read went on calling the tip this row's after another row had it. The row holds itself open on this
    /// (`NavItemDelegate.factsTipOut`).
    readonly property var tipInstance: facts.ToolTip.toolTip
    readonly property bool tipShown: !!facts.tipInstance && facts.tipInstance.visible
                                     && facts.tipInstance.parent === facts
    /// The hand is on these lines — read by the row as its own hover (`NavItemDelegate.factsPointed`).
    readonly property bool pointed: factsHover.hovered
    /// Whether the press that is ending took words with it — what tells a drag from a click. A drag must not reach
    /// the row, whose second click opens a name box.
    readonly property bool swept: pad.sweptText() !== ""
    /// What a run reads back off that sweep: whether the caret landed in a field, and what came away with it
    /// (`SweepPad`, verify-ui implement.md §壊れない動詞の実装と反復).
    readonly property bool caretLanded: pad.caretLanded
    function sweptText() {
        return pad.sweptText()
    }
    /// PGG_AUTO_ACT=nav-open-tag `:tip`: a rest on one line, entered at that line's own stand-in so what answers is
    /// the real wiring. False where the lines have not been built.
    function pointLineTip(row, on) {
        const line = lineSeats.itemAt(row)
        if (line === null)
            return false
        line.tipPointed = on
        return true
    }
    /// Clears any selection in these lines — a press on the row's own line does this first (`NavItemDelegate`):
    /// one selection in the window.
    function dropSweep() {
        pad.dropValues()
    }

    /// The band a line that goes somewhere is pressed in, in this item's coordinates — **target and wash are one
    /// rect** (デザイン規約 §当たり判定「端に接しないものは判定と塗りを一致させる」). Across the whole item
    /// (`bandReach`), and half the line step above and below so neighbouring bands meet. **Never above this item's
    /// top**: that is the row's own line. Lines going to one commit share one band (`NavFacts.joined`).
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
    /// How far this item reaches past its lines either side — the band's room around the mark and the measure. The
    /// seat adds this much (`NavItemDelegate` / `HeadPinRow`) so a band's hover, press and paint are all inside this
    /// item: a band reaching past it by a handler's margin took the hover off the row's own line, and the row closed
    /// under the hand (`tst_openrowholdshand`).
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
    /// what turns the pointer to the hand (`factsMouse`). Read off `factsHover`, which hears every line's band.
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
    /// The first line that goes somewhere, or -1 — the one a run presses (PGG_AUTO_ACT=nav-follow).
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
    /// Whether that line is wearing the band — read off the band that is drawn, and where — and (`handShown`) the
    /// pointer the block has turned for it.
    function lineAimed(row) {
        const band = facts.bandOf(row)
        return aimBand.visible && aimBand.y === band.y && aimBand.height === band.height
    }
    readonly property bool handShown: factsMouse.cursorShape === Qt.PointingHandCursor

    /// The hand, in this item's coordinates: the press anchors a sweep at the nearest field, the move drags it, and
    /// the click goes where it landed — unless the drag took words, in which case the reader was copying and nothing
    /// hears it.
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
        const to = button === Qt.LeftButton ? facts.goesAt(facts.pressFrom) : null
        if (to !== null) {
            facts.row.followFact(to)
            return
        }
        // A menu asked for from here is the row's, and keeps these lines up: they are what it is about
        // (デザイン規約 §左メニューの所作).
        if (button === Qt.RightButton)
            facts.row.factsMenuAsked()
        facts.row.rowPressed(button, modifiers)
    }
    /// **A double-click on a line that goes somewhere is two presses of it**, not a double-click on the row — which
    /// would move the working tree to a name the hand was not on.
    function handDoubled(button) {
        if (facts.swept || facts.row === null || facts.goesAt(facts.pressFrom) !== null)
            return
        facts.row.rowDoubled(button)
    }

    /// No height where no line is drawn, so the rows under it do not move.
    implicitHeight: lines.implicitHeight > 0 ? lines.implicitHeight + Theme.spaceXs : 0

    // The supplement: a line's note (why it wears a warning) or the block's path (WORKTREES — also asked for from the
    // row's own line). A row never has both; where one did, the note wins, being about the line the hand is on.
    readonly property string says:
        facts.noted !== "" ? facts.noted
        : (factsHover.hovered || facts.rowPointed) && facts.path !== "" ? facts.path : ""
    /// The words the box is wearing — **the last thing asked for, not what is asked now**: bound to the ask, the
    /// attached write-through empties the box the instant the hand leaves for it (rules-refs/app-ui.md の
    /// `NavRowFacts.said` の行). Written from the handler, which runs ahead of the `visible` binding on the same
    /// property, so the box never opens on the previous rest's words.
    property string said: ""
    onSaysChanged: if (facts.says !== "") facts.said = facts.says
    ToolTip.text: facts.said
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.visible: facts.says !== ""

    // Under everything drawn here; the hand below drives it.
    SweepPad {
        id: pad
        anchors.fill: parent
        content: lines
    }
    // The band the line under the hand wears (`bandOf`). **Its own look, not the row's wash**: the open row is
    // already lit, so the same wash would read as no change.
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
        // In by `bandReach` either side, so the lines stand in the row's own columns.
        anchors.left: parent.left
        anchors.leftMargin: facts.bandReach
        anchors.right: parent.right
        anchors.rightMargin: facts.bandReach
        anchors.top: parent.top
        spacing: Theme.spaceXs

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
                // **Truthiness, not a comparison**: lines are handed as plain values
                // (`tests/qml/tst_factstipsteal.qml`), and one without the field has no supplement.
                noted: !!modelData.note
                // The same truthiness: without the field the line goes nowhere.
                goes: !!modelData.to
                // Every line of the band the hand is on — a band may hold more than one (`bandOf`).
                aimed: facts.aimRow >= 0 && facts.bandStart(facts.aimRow) === facts.bandStart(index)
                onHandRested: on => facts.noteHand(index, on)
            }
        }
    }
    // The rest over these lines. **A handler, not the area below**: passive, so the sweeping area keeps answering the
    // same pointer (rules-refs/app-ui.md「行の hover は `HoverHandler`」). A run comes in at the row's own stand-in.
    HoverHandler {
        id: factsHover
    }

    // One hand for the whole of it — see the head of this file.
    MouseArea {
        id: factsMouse
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        preventStealing: true
        // A line that goes somewhere is pressed, not swept.
        cursorShape: facts.aimRow >= 0 ? Qt.PointingHandCursor : Qt.IBeamCursor
        // Each handler is one line into the item's own (verify-ui implement.md「注入はハンドラ本体そのものへ入れる」).
        onPressed: mouse => facts.handPressed(mouse.button, mouse.x, mouse.y)
        onPositionChanged: mouse => facts.handMoved(mouse.x, mouse.y)
        onReleased: facts.handReleased()
        onCanceled: facts.handReleased()
        onClicked: mouse => facts.handClicked(mouse.button, mouse.modifiers)
        onDoubleClicked: mouse => facts.handDoubled(mouse.button)
    }
}
