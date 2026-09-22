pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The shape every standing question takes (デザイン規約 §可否・警告の出し場所): a bar that comes down from the top of the list the
// question is about and pushes its rows down — what is being judged has to stay in sight —
// with the row it concerns marked, so the question is written exactly once.
//
// Two lists raise one: the graph, and the working tree's changed files. Only one question stands at a time (the page
// holds the run it guards), so the Escape below is never ambiguous.
Rectangle {
    id: bar

    /// The question. What the bar says — `open` alone raises and lowers it, so the words are
    /// still here while it goes back up.
    property string label: ""
    /// **A ref name inside the question's words**, and the sentence with the name's own seat left in it ("%1 where?").
    /// The name is drawn in the colour it wears everywhere else it is met, since the one name
    /// a question takes is the branch the working tree is on and that name is `textLink` wherever it appears — the
    /// left pane's row, the chip, the menu row (デザイン規約 §ref の種別「名前が出る場所すべてで textLink」). Both empty for
    /// a question that names nothing, and `label` carries the whole of it.
    property string labelSentence: ""
    property string labelRef: ""
    /// The colour that name takes instead. **Its own only while the bar has nothing of its own to say**: a question
    /// wearing a state colour says what it costs across the whole heading (§状態), and a name lit blue in the middle of
    /// it would read as the one part still answering — the rule the menu rows carry (`AppMenuItem.refColor`).
    readonly property color labelTint: bar.neutral ? Theme.textLink : bar.tone
    /// The heading as the markup rich text reads: the sentence in the bar's colour, the name in its own.
    readonly property string labelWords: bar.labelSentence === ""
                                         ? "" : Words.nameInSentence(bar.labelSentence, bar.labelRef, bar.labelTint)
    /// What answering costs, in the one line §用語 allows for it.
    property string detail: ""
    /// The words on the pill that answers. Left empty where the act has a command of its own and `code` says it
    /// instead.
    property string accept: ""
    /// The command this question is about, where what it guards is a git command
    /// (デザイン規約 §git 用語のコード表記). The question opens with it and the pill answers with it — one word said twice, from a
    /// single place, so the press that raises the bar and the press that answers it cannot come to name two different
    /// things. Empty leaves both in the ordinary voice, which is what every question whose act git has no one word for
    /// takes (`Move` / `Rename`).
    property string code: ""
    /// Throwing away work in hand — danger; reaching past this machine — warning (§状態).
    property bool danger: false
    /// The question asks for something: where to send a branch, what to call it. Nothing is at
    /// stake until the answer says otherwise, so it wears the accent — and turns the warning colours back on by
    /// itself when what is typed would reach past this machine after all (a name the remote already has).
    property bool neutral: false
    /// What the pill says on hover: that it is held, and what the far side will make of what it does. The bar's own
    /// line has room for one thing only, and this is where §長押し puts the rest.
    property string tip: ""
    /// The far side could not be read — the remote never answered, or what it named is not in this repository. The
    /// frame goes `warning` and a `!` follows the word, which is the same news the toolbar's own pair carries in the
    /// same two marks (デザイン規約 §リモートへ送る): **the word and the gesture stay as they were**, because what could not be read
    /// changes neither what would run nor how it is pressed.
    property bool alert: false
    /// Whether answering takes a hold (デザイン規約 §進行中・長押しの定数): the frame fills from the left while the
    /// press lasts, and letting go part way leaves nothing behind. A hold pill answers only on a fill that reaches
    /// the end: both releases stop there.
    property bool hold: false
    /// How far into the hold the press has got, 0 to 1.
    readonly property alias holdProgress: holdDrive.progress
    /// **The gesture the press under way was given** (`HoldDriver.armedMs`), which is the live one while no press is
    /// under way. The mark and the word's colour read this: the answer the question is waiting on
    /// can land while a hand is on the pill, and a pill that changed how it is answered under that hand would take a
    /// click for an answer to a question that now wants a hold (デザイン規約 §長押し).
    readonly property alias armedMs: holdDrive.armedMs
    /// The hand the question's words are dragged over from the air around them, named so a run can enter it (the way
    /// a card is reached at `<card>.background.pad`).
    property alias pad: askHand
    /// What the question needs in order to have an answer at all — a chooser, a name box. Declared by whoever raises
    /// the question, since only they know what is being asked; the bar just gives it a place under the words and above
    /// nothing (デザイン規約 §可否・警告の出し場所: the answer is given where the question is). Null for
    /// the questions the pill alone can answer.
    ///
    /// **Put down before the next one goes in** (`GraphPane.startAsking`). A Loader handed the
    /// component it already has keeps the item, so a second question of the same kind would come up holding what was
    /// typed into the first — and clearing it at the press instead would take the form out from under a bar that is
    /// still on screen, dropping the pill and the ✕ (centred against the whole row) by half its height.
    property Component form: null
    /// The loaded form, so the owner can read what was put into it.
    readonly property alias formItem: formLoader.item
    /// A form that says its answer is finished answers the question, exactly as the pill does.
    ///
    /// **Because the keyboard is in the form, not on the pill.** A question answered by the pill alone hands it the
    /// focus as the bar opens and is answered with Space or Enter there (デザイン規約 §立っている質問は 1 か所で聞く);
    /// a question with a form leaves the focus in the box, so the same Enter has to be the box's — otherwise the one
    /// question that *is* typed into is the one nobody can finish without reaching for the pointer. Which keystroke
    /// means "finished" is the form's to decide (`AppCombo.submitted`), and what it costs is the bar's.
    function answerFromForm() {
        // Nothing to send yet — the pill is dark, and a key cannot do what the press it stands for cannot. A held
        // question is the pill's alone for the same reason it is held at all (§進行中・長押しの定数): the gesture is
        // the intent, and a keystroke is not one.
        if (!bar.answerable || bar.hold)
            return
        bar.confirmed()
    }
    /// `ignoreUnknownSignals`, since a form is free to have no such keystroke: a form that is all choosers has
    /// nothing a key could finish, and answers with the pill alone.
    Connections {
        target: formLoader.item
        ignoreUnknownSignals: true
        function onAnswered() { bar.answerFromForm() }
    }
    /// Whether the pill can be pressed yet. A form that has nothing in it is a question with no answer to give, and the
    /// pill says so by going quiet — the shape of the bar holds while it is being
    /// filled in.
    property bool answerable: true
    /// Automation: run the hold to its end without a press behind it.
    function completeHold() {
        holdDrive.begin()
    }
    /// Called off without the hand letting go — the question itself has gone, so the fill blanks
    /// (HoldDriver.blank).
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
    /// The way out of a standing question, and **the body both gestures that take it enter** — the ✕ and Escape.
    /// Named so the headless run presses what a hand presses (verify-ui: a run that calls a second function doing the
    /// same thing passes for a mark, or a key, wired to nothing).
    function dismiss() {
        bar.cancelled()
    }

    /// Whether the question stands. **The words are `label` and the rest, and they stay put when it comes down**: the
    /// bar is on screen for the whole 200ms it spends going back up, and what can be seen holds still while it can be
    /// seen. A bar emptied at the press spends that time as a blank band in whatever colour its owner's bindings fell
    /// back to, which is the flash that was reported. The next question is what replaces the words
    /// (`GraphPane.startAsking` dresses the bar and then raises it; `stopAsking` only lowers it).
    property bool open: false
    readonly property color tone: bar.danger ? Theme.danger : bar.neutral ? Theme.accent : Theme.warning
    /// Whether the heading is written in the plain ink instead of the bar's colour.
    ///
    /// **For the one question whose colour follows an answer being picked in it** (`RenameCarryFlow`): the frame and
    /// the pill already turn as the chooser is worked through, and a heading turning with them is the same thing said
    /// a third time — read as though the words themselves were changing. Every other
    /// question's colour is settled when it opens and the heading is where it says what it costs (デザイン規約 §状態),
    /// so this is off unless a flow asks for it — and put down again as each question is dressed
    /// (`GraphPane.startAsking`).
    property bool plainWords: false
    // A question walked away from mid-press takes the press with it: a fill left standing would carry on into whatever
    // is asked next.
    //
    // Opening hands the pill the focus so the keyboard's way in needs no hunting for it. The bar only ever opens
    // because the person just asked for it from a list, so there is no typing here to interrupt.
    //
    // A tick later: the gesture that raised the question is still being delivered — a double-click on a graph
    // row has a release and a second click behind it — and the list it lands on takes the focus back if the pill claims
    // it first.
    //
    // A question with a form leaves the focus to it: the first thing to do there is type, and the form's
    // own box asks for it.
    onOpenChanged: {
        // First, ahead of the bindings that read the same property: this is the travel the 200ms is for.
        bar.travelling = true
        if (bar.open) {
            if (bar.form === null) {
                acceptPill.tookTheOpening = true
                Qt.callLater(acceptPill.forceActiveFocus)
            }
        } else {
            bar.blankHold()
        }
    }

    // Sized by its own words, opened and closed with the standard 200ms.
    clip: true
    color: Theme.bgElevated
    /// The height the words ask for, before the 200ms takes it there. Kept apart from the animated one so a reader
    /// (automation) can tell a bar that is all the way down from one still on its way — `label` is set a whole opening
    /// ahead of any of it being on screen, so a run that photographed on the label caught a red line and no words.
    readonly property real openHeight: bar.open ? askRow.implicitHeight + 2 * Theme.spaceMd : 0
    readonly property bool settled: bar.openHeight > 0 && bar.implicitHeight === bar.openHeight
    /// And the same edge at the other end: answered, and all the way back up. A run that photographed on the answer
    /// caught the bar half-retracted with its words cut off by the clip.
    readonly property bool shut: !bar.open && bar.implicitHeight === 0
    implicitHeight: bar.openHeight
    /// Whether the height on the move is the bar itself coming down or going back up. **The 200ms is those two and
    /// nothing else**: a bar already standing grows when its words arrive late — the remote's answer lands in the
    /// line under the heading — and those words take their room in the pass that lays them out, while an animated
    /// height would still be on its way there. What is left outside is cut by the clip, which was the boxes at the
    /// foot of the column with their bottoms sheared off and the hairline drawn through them (observed, `publish`
    /// in a narrow middle).
    ///
    /// **Raised from the handler, not read off `open`.** A condition standing on `open` answers with the value it
    /// holds when the height arrives, and a bar that was open a moment ago says "not travelling" about its own way
    /// out — which takes it off the screen in one frame, with the words still being read (observed:
    /// `publish-dismiss`). The handler runs ahead of the bindings that read the same property (app-ui.md), so the
    /// animation is armed before the height it is to carry.
    ///
    /// **The travel is over when the bar has arrived**, which the two edges a run photographs on already name: all
    /// the way down, or all the way back up.
    property bool travelling: false
    onSettledChanged: if (bar.settled) bar.travelling = false
    onShutChanged: if (bar.shut) bar.travelling = false
    Behavior on implicitHeight {
        enabled: bar.travelling
        NumberAnimation { duration: 200 }
    }
    BandRule {
        color: bar.tone
    }
    // The hand the question's own words are dragged over from the air around them (規約 §右のペインの字は掴める) — what
    // it is about is a branch, a remote or a folder this window is the only place to read, and a reader who has to
    // retype one of those out of a warning is a reader the bar failed. Under the row, so the pill
    // that answers, the ✕ and whatever the form put up all keep every press they had.
    SweepPad {
        id: askHand
        anchors.fill: parent
        content: askWords
    }
    RowLayout {
        id: askRow
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: Theme.spaceMd
        spacing: Theme.spaceMd
        ColumnLayout {
            id: askWords
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            // The question, opened by the command where the act it guards is one
            // (デザイン規約 §git 用語のコード表記) — `push main where?` reads as one sentence,
            // so the chip takes the heading's weight.
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                // The chip opens the sentence, so it stands on the line the sentence **starts** on: both are hung
                // from the top and the shorter of the two drops half the difference, which is where a centred row
                // put them while the heading was one line. Centred against the whole block instead, a chip beside a
                // heading that wrapped floats between its lines, reading as a word of its own.
                CodeChip {
                    id: askCode
                    Layout.alignment: Qt.AlignTop
                    Layout.topMargin: Math.max(0, (askWord.lineHeight - askCode.implicitHeight) / 2)
                    visible: bar.code !== ""
                    word: bar.code
                    tint: bar.tone
                    weight: Font.DemiBold
                }
                // Fields, because what the question names is this window's own — a branch, a remote, the
                // folder another working copy is holding — and none of it is written anywhere a reader could take it
                // from while the bar is standing over the list (規約 §右のペインの字は掴める).
                //
                // **Wrapped, never cut** (デザイン規約 §答えの要らない報せ): the name is what is being decided about, and a
                // question whose subject ends in a `…` is a question nobody can answer. So the wrapping half of the
                // pair stands here, and the bar grows by the line.
                CardText {
                    id: askWord
                    text: bar.label
                    markup: bar.labelWords
                    color: bar.plainWords ? Theme.textPrimary : bar.tone
                    pixelSize: Theme.fontMd
                    weight: Font.DemiBold
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignTop
                    // The other half of the pair above, and **only where a chip stands**: an invisible chip is out of
                    // the layout but still has a height of its own, and read unguarded it would drop every question
                    // git has no one word for.
                    Layout.topMargin: bar.code === ""
                                      ? 0 : Math.max(0, (askCode.implicitHeight - askWord.lineHeight) / 2)
                }
            }
            // **Gone where there is nothing to say**, not merely empty: an empty `CardText` still stands a line high,
            // and a question whose answers say the whole of it themselves would carry a blank band under its heading
            // (`RenameCarryFlow` is the one, デザイン規約 §手元の改名をリモートへ運ぶ). A layout skips what is not
            // visible, spacing and all.
            CardText {
                text: bar.detail
                visible: bar.detail !== ""
                color: Theme.textSecondary
                pixelSize: Theme.fontSm
                Layout.fillWidth: true
            }
            // Put down as the next question is dressed, so the pill and the ✕ — centred against the whole row — hold
            // still when the ✕ is pressed. What was typed still cannot come back with the next question:
            // `GraphPane.startAsking` clears the form before it hands over the new one, and a Loader given the same
            // component twice would otherwise keep the item it has.
            Loader {
                id: formLoader
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
            // — 警告の色は枠と印が持つ).
            //
            // Colour comes back where the gesture changes. A coloured word belongs to the presses that are held —
            // `Remove`, the stopped fetch's `Resume`, the toolbar's own `push -f` — so on a pill it reads as a held
            // press. That is also why the remote that could not be read keeps the plain
            // word: it is still one click, and the frame and the `!` carry the whole of that news.
            readonly property color wordInk:
                !bar.answerable ? Theme.textMuted
                : bar.holdProgress > 0 ? Theme.textOnAccent : bar.armedMs > 0 ? bar.tone : Theme.textPrimary
            // Reachable without a pointer, and given the focus as the bar opens: the pill is the only thing here that
            // acts, so there is nothing else for a tab to land on first (デザイン規約 §長押し).
            //
            // Closed, it leaves the tab order by going disabled — Qt refuses to clear `activeFocusOnTab` on the item
            // that currently holds the focus, and warns. Disabling takes the focus away first, and the pill draws its
            // own colours, so the collapse looks no different.
            activeFocusOnTab: true
            enabled: bar.open && bar.answerable
            /// Whether the focus this pill holds arrived under a finger. Cleared when the focus leaves, so the next way
            /// in is read on its own terms — and cleared when the bar closes with it, since the focus goes then too.
            property bool tookAPress: false
            /// And whether the focus it holds is the one the bar handed it as it opened. **Nobody reached for it**,
            /// so the ring has nothing to report — a question that comes down wearing a blue frame over its own
            /// warning one is saying "the keyboard is here" to a reader who has not touched the keyboard. Cleared the
            /// moment the focus leaves, so a tab back onto the pill rings.
            property bool tookTheOpening: false
            onActiveFocusChanged: {
                if (acceptPill.activeFocus)
                    return
                acceptPill.tookAPress = false
                acceptPill.tookTheOpening = false
                // A key let go after the focus has moved is answered somewhere else (`HoldDriver.focusLost`).
                holdDrive.focusLost()
            }
            // The pill draws itself, so it has to name itself. The gesture is said here
            // because the words on it no longer carry it.
            Accessible.role: Accessible.Button
            Accessible.name: bar.code !== "" ? bar.code : bar.accept
            Accessible.description: bar.hold ? Words.holdToActivate : ""
            // That the fill reaches the end is the whole progress report.
            HoldFill {
                progress: bar.holdProgress
                tone: bar.tone
                inset: Theme.borderWidth
            }
            // Outside the frame: the frame's colour says what answering costs, and it keeps saying it under the
            // ring. **Outside is still over it** — a blue ring around a warning or a danger frame reads as one more
            // colour on the same control — so the ring only comes up for focus somebody actually moved here.
            //
            // Focus that came from the keyboard (`ActionButton` carries the same reading and the
            // reason). **`visualFocus` belongs to `Control`** — this pill is drawn as a
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
                         && !acceptPill.tookTheOpening
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
                    visible: bar.armedMs > 0
                    progress: bar.holdProgress
                    tint: acceptPill.wordInk
                }
                // The command, where the question is about one: the same word the button that raised the bar wears, in
                // the same dress (デザイン規約 §git 用語のコード表記). Which of these two speaks is `code` alone, so the pill has no
                // wording of its own to drift from the head of the question. The word, and the mark that stands at the
                // end of it. Only the word is in the row: the mark hangs off the end of the word's advance and into the
                // pill's own padding, which is how the toolbar sets the same pair — spaced by the air the chip keeps at
                // its edge (デザイン規約 §git 用語のコード表記).
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
                onPressed: {
                    acceptPill.tookAPress = true
                    // Unconditionally: the latch is what a plain press is answered from too, and the question's own
                    // answer can come back while a hand is on the pill (`HoldDriver.armedMs`).
                    holdDrive.begin()
                }
                // `released` inside the pill: Qt stops emitting `clicked` once its own press-and-hold
                // timer has gone off (800ms), so a click pill held down the way the hold pills ask for would answer
                // nothing at all and say nothing about it. Releasing away from the pill still calls it off —
                // `containsMouse` is what a click checked.
                onReleased: {
                    // Worked out before the latch opens, and nothing at all where what the question asks for has
                    // changed since the press (`HoldDriver.stale`): a click that became a hold under the hand is not
                    // an answer to the question now standing (デザイン規約 §長押し).
                    const plain = holdDrive.armedMs <= 0 && !holdDrive.stale && containsMouse
                    holdDrive.letUp()
                    if (plain)
                        bar.confirmed()
                }
                // A press dragged off the pill has to call the hold off, the way letting go does — `pressed` stays
                // true out there, since the grab is kept, so sliding away would otherwise be no escape at all. Read
                // off the moves — `containsPress` also falls on the release above and would open
                // the latch before that handler had read it.
                onPositionChanged: if (!containsMouse) holdDrive.letUp()
                onCanceled: holdDrive.letUp()
            }
            // The same answer without a pointer: Space or Enter, held where the question asks for a hold
            // (`HoldDriver.pressKey`, which is disarmed here when it does not) and simply pressed where it does not —
            // the key is the pill's either way, and a plain press lands its answer as it comes back up.
            Keys.onPressed: event => {
                if (holdDrive.pressKey(event) || !holdDrive.ownsKey(event))
                    return
                event.accepted = true
            }
            Keys.onReleased: event => {
                if (holdDrive.releaseKey(event) || !holdDrive.ownsKey(event))
                    return
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
                onClicked: bar.dismiss()
            }
        }
    }
    // Escape is heard as a shortcut: while a question stands the focus is on
    // the pill or on the form's own box, and the graph list — which takes it on a row click — holds no Escape of its
    // own to swallow it first.
    //
    // **This bar reads only its own standing.** A question takes Escape ahead of the report bar, and that
    // order is written in the one place the two meet (`RepoPage`, デザイン規約 §立っている質問は 1 か所で聞く): this bar is
    // always the one that wins it, so the condition is its own standing and nothing else.
    Shortcut {
        id: escapeKey
        // `sequences`: Cancel is more than one key on some platforms, and binding the single
        // form takes only the first of them (Qt warns about exactly this).
        sequences: [StandardKey.Cancel]
        enabled: bar.open
        // The ✕'s own body: Escape and the ✕ walk away from the same question, and a run pressing a copy would pass
        // for a key wired to nothing (verify-ui).
        onActivated: bar.dismiss()
    }
    /// Automation: whether Escape is this bar's to take at this moment. **Read off the shortcut itself**, so a build
    /// where the key is dead cannot say otherwise — the reading `QuitWaitDialog.escapes` takes, for the same reason.
    readonly property alias escapes: escapeKey.enabled
}
