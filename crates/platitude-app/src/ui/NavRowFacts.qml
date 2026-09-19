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
// **What these lines have of their own is said on a rest over them** — a working copy's path (`path`), which is
// the supplement to what is already open rather than a line of its own (デザイン規約 §左メニューの所作).
//
// **The name is not here.** It stays in the row's own place and is shown whole there (`NameCell.whole`): what the
// reader was looking at does not move when a hand rests on it.
//
// **It is the row growing, not a card over it.** The row is this much taller while it is open and the rows below it
// move down, so the list opens rather than something landing on top of it. The icons stand in the row's own mark
// column and the words begin where its name does, which is what makes the marks look like they moved down here — and
// the row takes its marks off its own line while this is out.
//
// **The press is taken here and split in two**: a drag takes the words away, and a press that never moved is the
// row's own click. A `TextEdit` under a `MouseArea` declared after it never sees a press (measured), so one hand
// answers for the whole of this and drives the sweep itself (`SweepPad`).
Item {
    id: facts

    /// The row this belongs to — where a press that took no words goes (`NavItemDelegate.rowPressed`).
    property Item row: null
    /// The lines to draw, in reading order — **one table answers for every section** (`NavFacts.lines`), so
    /// nothing here knows which of them the row is in. Each is `{mark, markTint, text, tone, ahead, behind}`.
    property var lines: []
    /// WORKTREES only: where this working copy stands — the path git lists it under. **Not a line**: the row is
    /// named by the folder that path ends in, and a reader who has already opened the row is asking about the copy
    /// and not about where it sits on the disk. It comes out as the supplement to what is open, on a rest over
    /// these lines (デザイン規約 §左メニューの所作), which is the one place it answers a question somebody asked.
    /// Empty on every other kind of row.
    property string path: ""
    /// Stands in for the pointer on these lines where headless cannot put one (PGG_AUTO_ACT=nav-open-tip).
    property bool tipPointed: false
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
    /// When the button went down, so the row's gesture takes the time it was held off the wait it has left — Qt
    /// measures a double-click press to press, and the click arrives at the release (`NavItemDelegate`).
    property real heldFrom: 0
    /// Whether the press that is ending took words with it. **That is what tells a drag from a click** — a sweep that
    /// came away empty never moved — and a drag must not reach the row, whose second click opens a name box.
    readonly property bool swept: pad.sweptText() !== ""
    /// What a run reads back off that sweep: whether the caret landed in a field, and what came away with it
    /// (`SweepPad`, verify-ui §壊れない動詞の実装).
    readonly property bool caretLanded: pad.caretLanded
    function sweptText() {
        return pad.sweptText()
    }
    /// Nothing under these lines holds a selection any more — what a press on the row's own line above clears
    /// before it decides what that press is (`NavItemDelegate`). One selection in the window.
    function dropSweep() {
        pad.dropValues()
    }

    /// The hand, in this item's own coordinates: the press anchors a sweep at the nearest field whether it landed on
    /// the words or in the air beside them, the move drags it, and the click is the row's — unless the drag took
    /// words with it, in which case the reader was copying and the row hears nothing.
    function handPressed(button, x, y) {
        if (button !== Qt.LeftButton)
            return
        facts.heldFrom = Date.now()
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
        // **A menu asked for from here is about the row these lines belong to**, so it does not take them down: the
        // reader who right-clicked them is reading about that row, and lines that went out from under the menu would
        // take what it is about off the screen with them (デザイン規約 §左メニューの所作).
        if (button === Qt.RightButton)
            facts.row.factsMenuAsked()
        facts.row.rowPressed(button, modifiers, Date.now() - facts.heldFrom)
    }
    function handDoubled(button) {
        if (!facts.swept && facts.row !== null)
            facts.row.rowDoubled(button)
    }

    /// **Nothing at all where no line is drawn**: a row whose whole answer is its own name opens no height, so the
    /// rows under it do not move for it (the name is shown in the row's own place — `NameCell.whole`).
    implicitHeight: lines.implicitHeight > 0 ? lines.implicitHeight + Theme.spaceXs : 0

    // The supplement itself: what these lines keep for a hand that rests on them (`path`), on the same wait every
    // other one of them opens after.
    ToolTip.visible: (factsHover.hovered || facts.tipPointed) && facts.path !== ""
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: facts.path

    // The hand's ground, under everything drawn here: the pad answers where the fields are, and the hand below drives
    // it (`SweepPad`).
    SweepPad {
        id: pad
        anchors.fill: parent
        content: lines
    }
    ColumnLayout {
        id: lines
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        spacing: Theme.spaceXs

        // One line to a fact, as the table handed them over (`NavFacts.lines`): the mark that stands for it in the
        // row's own mark column, the words beginning where the row's name does, and — where the line names a
        // branch — the measure that branch's own row draws.
        Repeater {
            model: facts.lines
            NavFactLine {
                required property var modelData
                Layout.fillWidth: true
                mark: modelData.mark
                markTint: modelData.markTint
                text: modelData.text
                tone: modelData.tone
                ahead: modelData.ahead
                behind: modelData.behind
            }
        }
    }
    // The rest over what is open, and the one thing these lines keep for it: where that working copy stands.
    // **A handler, not the area below** — handlers are passive, so the hand that sweeps the words goes on being
    // answered by the same area while this reads the same pointer (rules-refs/app-ui.md 「行の hover は
    // `HoverHandler`」). The stand-in beside it is for the runs, which have no pointer to rest (verify-ui スキル).
    HoverHandler {
        id: factsHover
    }

    // **One hand for the whole of it, and it answers two gestures** — see the head of this file.
    MouseArea {
        id: factsMouse
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        preventStealing: true
        cursorShape: Qt.IBeamCursor
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
