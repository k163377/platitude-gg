pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The shape every standing question takes (デザイン規約 §可否・警告の出し場所): a bar that comes down from the top of the list the
// question is about and pushes its rows down rather than covering them — what is being judged has to stay in sight —
// with the row it concerns marked instead of named, so the question is written exactly once.
//
// Two lists raise one: the graph, and the working tree's changed files. Only one question stands at a time (the page
// holds the run it guards), so the Escape below is never ambiguous.
Rectangle {
    id: bar

    /// The question. Empty is closed; nothing else opens or shuts it.
    property string label: ""
    /// What answering costs, in the one line §用語 allows for it.
    property string detail: ""
    /// The words on the pill that answers. Left empty where the act has a command of its own and `code` says it
    /// instead.
    property string accept: ""
    /// The command this question is about, where what it guards is a git command rather than a description of one
    /// (デザイン規約 §git 用語のコード表記). The question opens with it and the pill answers with it — one word said twice, from a
    /// single place, so the press that raises the bar and the press that answers it cannot come to name two different
    /// things. Empty leaves both in the ordinary voice, which is what every question whose act git has no one word for
    /// takes (`Move` / `Rename`).
    property string code: ""
    /// Throwing away work in hand (danger) rather than reaching past this machine (warning) — §状態.
    property bool danger: false
    /// The question asks for something rather than for consent: where to send a branch, what to call it. Nothing is at
    /// stake until the answer says otherwise, so it wears neither warning colour — and it turns them back on by itself
    /// when what is typed would reach past this machine after all (a name the remote already has).
    property bool neutral: false
    /// What the pill says on hover: that it is held, and what the far side will make of what it does. The bar's own
    /// line has room for one thing only, and this is where §長押し puts the rest.
    property string tip: ""
    /// The far side could not be read — the remote never answered, or what it named is not in this repository. The
    /// frame goes `warning` and a `!` follows the word, which is the same news the toolbar's own pair carries in the
    /// same two marks (デザイン規約 §リモートへ送る): **the word and the gesture stay as they were**, because what could not be read
    /// changes neither what would run nor how it is pressed.
    property bool alert: false
    /// Whether answering takes a hold rather than a click (デザイン規約 §進行中・長押しの定数): the frame fills from the left while the
    /// press lasts, and letting go part way leaves nothing behind. A hold pill reports no click at all — neither the
    /// release that completes the hold nor the one that gives up on it may fall through to the answer.
    property bool hold: false
    /// How far into the hold the press has got, 0 to 1.
    readonly property alias holdProgress: holdDrive.progress
    /// What the question needs in order to have an answer at all — a chooser, a name box. Declared by whoever raises
    /// the question, since only they know what is being asked; the bar just gives it a place under the words and above
    /// nothing (デザイン規約 §可否・警告の出し場所: the answer is given where the question is, not in a window of its own). Null for
    /// the questions the pill alone can answer.
    property Component form: null
    /// The loaded form, so the owner can read what was put into it.
    readonly property alias formItem: formLoader.item
    /// Whether the pill can be pressed yet. A form that has nothing in it is a question with no answer to give, and the
    /// pill says so by going quiet rather than by disappearing — the shape of the bar must not jump while it is being
    /// filled in.
    property bool answerable: true
    /// Automation: run the hold to its end without a press behind it.
    function completeHold() {
        holdDrive.begin()
    }
    /// Called off without the hand letting go — the question itself has gone, so the fill blanks instead of sliding
    /// back (HoldDriver.blank).
    function blankHold() {
        holdDrive.blank()
    }
    HoldDriver {
        id: holdDrive
        holdMs: bar.hold ? Metrics.holdMs : 0
        onFinished: bar.confirmed()
    }

    /// The pill was clicked: the owner runs what the question guarded.
    signal confirmed()
    /// Walked away from — Escape, the ✕, or a click elsewhere.
    signal cancelled()

    readonly property bool open: bar.label !== ""
    readonly property color tone: bar.danger ? Theme.danger : bar.neutral ? Theme.accent : Theme.warning
    // A question walked away from mid-press takes the press with it: a fill left standing would carry on into whatever
    // is asked next.
    //
    // Opening hands the pill the focus so the keyboard's way in needs no hunting for it. The bar only ever opens
    // because the person just asked for it from a list, so there is no typing here to interrupt.
    //
    // A tick later, not now: the gesture that raised the question is still being delivered — a double-click on a graph
    // row has a release and a second click behind it — and the list it lands on takes the focus back if the pill claims
    // it first.
    //
    // A question with a form does not take the focus for its pill: the first thing to do there is type, and the form's
    // own box asks for it.
    onOpenChanged: {
        if (bar.open) {
            if (bar.form === null)
                Qt.callLater(acceptPill.forceActiveFocus)
        } else {
            bar.blankHold()
        }
    }

    // Sized by its own words, opened and closed with the standard 200ms.
    clip: true
    color: Theme.bgElevated
    implicitHeight: bar.open ? askRow.implicitHeight + 2 * Theme.spaceMd : 0
    Behavior on implicitHeight {
        NumberAnimation { duration: 200 }
    }
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: Theme.borderWidth
        color: bar.tone
    }
    RowLayout {
        id: askRow
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: Theme.spaceMd
        spacing: Theme.spaceMd
        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            // The question, opened by the command where the act it guards is one (デザイン規約 §git 用語のコード表記) — `push main
            // where?` reads as one sentence, so the chip takes the heading's weight rather than sitting in it as a
            // lighter word.
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                CodeChip {
                    Layout.alignment: Qt.AlignVCenter
                    visible: bar.code !== ""
                    word: bar.code
                    tint: bar.tone
                    weight: Font.DemiBold
                }
                Label {
                    text: bar.label
                    color: bar.tone
                    font.pixelSize: Theme.fontMd
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
            }
            Label {
                text: bar.detail
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
            // Built only while the question stands, so what was typed into the last one cannot come back with the next.
            Loader {
                id: formLoader
                active: bar.open
                sourceComponent: bar.form
                Layout.fillWidth: true
                Layout.topMargin: bar.form === null ? 0 : Theme.spaceXs
            }
        }
        // This is the answer — by a click, or by a press held all the way down where the question asks for one. It is
        // the only thing on the bar that acts, so nothing else here can be hit by accident.
        Rectangle {
            id: acceptPill
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: acceptRow.implicitWidth + 2 * Theme.spaceMd
            implicitHeight: Theme.controlHeight
            radius: Theme.radiusSm
            color: acceptMouse.containsMouse && bar.holdProgress === 0 ? Theme.bgHover : "transparent"
            border.color: bar.alert ? Theme.warning : bar.tone
            border.width: Theme.borderWidth
            // The ink for everything the pill says — the mark, the word, the chip a command wears — decided once so the
            // three cannot come apart. Lifted while the fill runs under it: what is written crosses both the filled
            // side and the bare one. With nothing to send yet, only the ink drops — the frame stays, so the bar keeps
            // its shape while it is filled in (デザイン規約 §無効).
            //
            // A pill answered by a click keeps the plain word, whatever the bar is toned: this is the same act as the
            // button that raised it, and that button says its word in `textPrimary` in every state it can be clicked in
            // — a first push is not a different press for being asked about first. What the answer costs is the frame's
            // to say, and the bar has said it twice over by then, in the heading and in the line under it (デザイン規約 §長押し
            // — 警告の色は語ではなく枠と印が持つ).
            //
            // Colour comes back where the gesture changes. A coloured word belongs to the presses that are held —
            // `Remove`, the stopped fetch's `Resume`, the toolbar's own `push -f` — so on a pill it reads as "this one
            // is not clicked" rather than as trim. That is also why the remote that could not be read keeps the plain
            // word: it is still one click, and the frame and the `!` carry the whole of that news.
            readonly property color wordInk:
                !bar.answerable ? Theme.textMuted
                : bar.holdProgress > 0 ? Theme.textOnAccent : bar.hold ? bar.tone : Theme.textPrimary
            // Reachable without a pointer, and given the focus as the bar opens: the pill is the only thing here that
            // acts, so there is nothing else for a tab to land on first (デザイン規約 §長押し).
            //
            // Closed, it leaves the tab order by going disabled rather than by dropping `activeFocusOnTab` — Qt refuses
            // to clear that on the item that currently holds the focus, and warns. Disabling takes the focus away
            // first, and the pill draws its own colours rather than the palette's, so the collapse looks no different.
            activeFocusOnTab: true
            enabled: bar.open && bar.answerable
            /// Whether the focus this pill holds arrived under a finger. Cleared when the focus leaves, so the next way
            /// in is read on its own terms — and cleared when the bar closes with it, since the focus goes then too.
            property bool tookAPress: false
            onActiveFocusChanged: if (!acceptPill.activeFocus) acceptPill.tookAPress = false
            // The pill draws itself rather than being a control, so it has to name itself. The gesture is said here
            // because the words on it no longer carry it.
            Accessible.role: Accessible.Button
            Accessible.name: bar.code !== "" ? bar.code : bar.accept
            Accessible.description: bar.hold ? qsTr("Hold to activate") : ""
            // The hold filling the frame from the left, inset by the border so the frame stays a frame while it fills:
            // that the fill reaches the end is the whole progress report.
            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.margins: Theme.borderWidth
                // Never thinner than `holdFillMin` while it runs — see ActionButton for why the proportional start is
                // no good.
                width: bar.holdProgress > 0
                       ? Math.max(Metrics.holdFillMin,
                                  (parent.width - 2 * Theme.borderWidth) * bar.holdProgress)
                       : 0
                color: bar.tone
                visible: bar.holdProgress > 0
            }
            // Outside the frame: the frame's colour says what answering costs, and focus must not be able to take that
            // over.
            //
            // Focus that came from the keyboard, not from a press (`ActionButton` carries the same reading and the
            // reason). **`visualFocus` is not ours to read** — that is a `Control` property and this pill is drawn as a
            // plain rectangle, so binding to it assigns `undefined` and the ring never comes up at all (QML says so
            // twice per bar and nowhere else). Where a press is the only other way in, remembering that it happened
            // says the same thing.
            Rectangle {
                anchors.fill: parent
                anchors.margins: -Theme.spaceXs / 2
                color: "transparent"
                border.color: Theme.borderFocus
                border.width: Theme.borderWidth
                radius: Theme.radiusMd
                visible: acceptPill.activeFocus && !acceptPill.tookAPress
            }
            Row {
                id: acceptRow
                anchors.centerIn: parent
                spacing: Theme.spaceXs
                // Ahead of the word, where the eye starts: the pill says how it is answered before it says what
                // answering does (デザイン規約 §長押し). A pill answered by a click wears no mark and spends no width on one.
                HoldIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.verticalCenterOffset: Metrics.opticalDrop
                    visible: bar.hold
                    progress: bar.holdProgress
                    tint: acceptPill.wordInk
                }
                // The command, where the question is about one: the same word the button that raised the bar wears, in
                // the same dress (デザイン規約 §git 用語のコード表記). Which of these two speaks is `code` alone, so the pill has no
                // wording of its own to drift from the head of the question. The word, and the mark that stands at the
                // end of it. Only the word is in the row: the mark hangs off the end of the word's advance and into the
                // pill's own padding, which is how the toolbar sets the same pair — spaced by the air the chip keeps at
                // its edge rather than by a gap of its own (デザイン規約 §git 用語のコード表記).
                Item {
                    id: wordSeat
                    anchors.verticalCenter: parent.verticalCenter
                    implicitWidth: bar.code !== "" ? acceptCode.implicitWidth : acceptWord.implicitWidth
                    implicitHeight: bar.code !== "" ? acceptCode.implicitHeight : acceptWord.implicitHeight
                    CodeChip {
                        id: acceptCode
                        visible: bar.code !== ""
                        word: bar.code
                        tint: acceptPill.wordInk
                    }
                    Label {
                        id: acceptWord
                        visible: bar.code === ""
                        text: bar.accept
                        color: acceptPill.wordInk
                        font.pixelSize: Theme.fontMd
                    }
                    NavIcon {
                        visible: bar.alert
                        kind: "bang"
                        tint: Theme.warning
                        width: Theme.iconSm
                        height: Theme.iconSm
                        x: wordSeat.implicitWidth
                        y: -Theme.spaceXs
                    }
                }
            }
            ToolTip.visible: bar.tip !== "" && acceptMouse.containsMouse
            ToolTip.delay: Metrics.tipDelayMs
            ToolTip.text: bar.tip
            MouseArea {
                id: acceptMouse
                anchors.fill: parent
                hoverEnabled: true
                // A hand on the pill takes the focus with it, and a ring drawn for that is a ring nobody asked for
                // (§長押し: the ring is the keyboard's way of seeing where it is).
                onPressed: acceptPill.tookAPress = true
                // `released` inside the pill, not `clicked`: Qt stops emitting `clicked` once its own press-and-hold
                // timer has gone off (800ms), so a click pill held down the way the hold pills ask for would answer
                // nothing at all and say nothing about it. Releasing away from the pill still calls it off —
                // `containsMouse` is what a click checked.
                onReleased: if (!bar.hold && containsMouse) bar.confirmed()
                // `containsPress`, not `pressed`: a press dragged off the pill has to call the hold off, the way
                // letting go does. `pressed` stays true out there — it keeps the grab — and would leave sliding away as
                // no escape at all.
                onContainsPressChanged: {
                    if (!bar.hold)
                        return
                    if (containsPress)
                        holdDrive.begin()
                    else
                        holdDrive.letUp()
                }
            }
            // The same answer without a pointer: Space or Enter, held where the question asks for a hold and simply
            // pressed where it does not. Auto-repeat is dropped on both edges — see ActionButton.
            Keys.onPressed: event => {
                if (event.isAutoRepeat || !holdDrive.holdKey(event.key))
                    return
                if (bar.hold)
                    holdDrive.begin()
                event.accepted = true
            }
            Keys.onReleased: event => {
                if (event.isAutoRepeat || !holdDrive.holdKey(event.key))
                    return
                if (bar.hold)
                    holdDrive.letUp()
                else
                    bar.confirmed()
                event.accepted = true
            }
        }
        // Escape and a click anywhere else walk away too; this is the way out that can be seen, for a bar that stands
        // until it is answered — and a step under what it closes, like every other one of these (デザイン規約 §寸法).
        NavIcon {
            Layout.alignment: Qt.AlignVCenter
            width: Theme.iconSm
            height: Theme.iconSm
            kind: "close"
            tint: dismissMouse.containsMouse ? Theme.textPrimary : Theme.textSecondary
            MouseArea {
                id: dismissMouse
                anchors.fill: parent
                anchors.margins: -Theme.spaceXs
                hoverEnabled: true
                onClicked: bar.cancelled()
            }
        }
    }
    // Escape is heard as a shortcut rather than as a key handler on the bar: while a question stands the focus is on
    // the pill or on the form's own box, and the graph list — which takes it on a row click — holds no Escape of its
    // own to swallow it first.
    Shortcut {
        // `sequences` rather than `sequence`: Cancel is more than one key on some platforms, and binding the single
        // form takes only the first of them (Qt warns about exactly this).
        sequences: [StandardKey.Cancel]
        enabled: bar.open
        onActivated: bar.cancelled()
    }
}
