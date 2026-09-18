pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import platitude.ui

// What a branch row of the left panel opens under itself: the name in full where the row had to cut it, the working
// copy that has the branch out, and the branch it is measured against — one fact to a line, each behind the mark the
// row wears for it (デザイン規約 §左メニューの所作).
//
// **It is the row growing, not a card over it.** The row is this much taller while it is open and the rows below it
// move down, so the list opens rather than something landing on top of it. The icons stand in the row's own mark
// column and the words begin where its name does, which is what makes the marks look like they moved down here — and
// the row takes them off its own line while this is out.
//
// **The press is taken here and split in two**: a drag takes the words away, and a press that never moved is the
// row's own click. A `TextEdit` under a `MouseArea` declared after it never sees a press (measured), so one hand
// answers for the whole of this and drives the sweep itself (`SweepPad`).
Item {
    id: facts

    /// The row this belongs to — where a press that took no words goes (`NavItemDelegate.rowPressed`).
    property Item row: null
    /// The name in full, empty wherever the row shows the whole of it already: two copies of one name, one under the
    /// other, is the row saying the same thing twice.
    property string fullName: ""
    property color nameTone: Theme.textPrimary
    property int nameWeight: Font.Normal
    /// The working copy that has this branch out, empty where this one does.
    property string heldBy: ""
    /// The remote branch the counts are measured against, empty where the branch tracks nothing.
    property string upstream: ""
    /// That branch is configured but not here — git's own `[gone]` (デザイン規約 §左メニューの所作).
    property bool gone: false
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

    implicitHeight: lines.implicitHeight + Theme.spaceXs

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

        // The name the row had to cut, whole and wrapped. It carries the row's own colour and weight: the same name
        // in the same voice, one line further down.
        CardText {
            visible: facts.fullName !== ""
            Layout.fillWidth: true
            Layout.leftMargin: Theme.iconSm + Theme.spaceXs
            text: facts.fullName
            pixelSize: Theme.fontMd
            color: facts.nameTone
            weight: facts.nameWeight
        }
        // Local first, then the remote: the reader's own copy of this branch, then where it is measured against.
        RowLayout {
            spacing: Theme.spaceXs
            visible: facts.heldBy !== ""
            NavIcon {
                kind: "tree"
                tint: Theme.textSecondary
                Layout.preferredWidth: Theme.iconSm
                Layout.preferredHeight: Theme.iconSm
            }
            CardText {
                Layout.fillWidth: true
                text: facts.heldBy
                pixelSize: Theme.fontSm
                color: Theme.textSecondary
            }
        }
        RowLayout {
            spacing: Theme.spaceXs
            visible: facts.upstream !== ""
            NavIcon {
                kind: "remote"
                tint: facts.gone ? Theme.warning : Theme.textSecondary
                Layout.preferredWidth: Theme.iconSm
                Layout.preferredHeight: Theme.iconSm
            }
            // A branch git answers `[gone]` for says so in words, and the whole line wears the state
            // (デザイン規約 §状態). **The sentence is what the row cannot say**: the row wears the mark, and the
            // mark alone cannot tell "measured against something that is not here" from "measured against
            // something". git's own word for it is `gone`, and it leads with the name so every line here begins in
            // the same column.
            CardText {
                Layout.fillWidth: true
                text: facts.gone ? qsTr("%1 is gone").arg(facts.upstream) : facts.upstream
                pixelSize: Theme.fontSm
                color: facts.gone ? Theme.warning : Theme.textSecondary
            }
        }
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
