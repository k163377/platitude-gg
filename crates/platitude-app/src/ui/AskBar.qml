pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The shape every standing question takes (デザイン規約 §可否・警告の出し場所): a bar that comes down from the top of
// the list the question is about and pushes its rows down, so what is being judged stays in sight. Only one question
// stands at a time (the page holds the run it guards), so the Escape below is never ambiguous.
Rectangle {
    id: bar

    /// The question's words; they stay set while the bar goes back up (`open`).
    property string label: ""
    /// A ref name inside the question's words, and the sentence with the name's seat left in it ("%1 where?"). The
    /// name keeps the `textLink` it wears wherever it appears (デザイン規約 §ref の種別「名前が出る場所すべてで textLink」).
    /// Both empty for a question that names nothing; `label` then carries the whole of it.
    property string labelSentence: ""
    property string labelRef: ""
    /// The name's own blue only on a neutral bar: a state colour takes the whole heading (the rule
    /// `AppMenuItem.refColor` carries).
    readonly property color labelTint: bar.neutral ? Theme.textLink : bar.tone
    readonly property string labelWords: bar.labelSentence === ""
                                         ? "" : Words.nameInSentence(bar.labelSentence, bar.labelRef, bar.labelTint)
    /// What answering costs, in the one line §用語 allows for it.
    property string detail: ""
    /// The pill's word. Empty where the act is a git command and `code` says it instead.
    property string accept: ""
    /// The git command the question guards (デザイン規約 §git 用語のコード表記). The heading opens with it and the pill
    /// answers with it — one property, so the two cannot come to name different things. Empty for acts git has no one
    /// word for (`Move` / `Replace`): both then take the ordinary voice.
    property string code: ""
    /// Throwing away work in hand — danger; reaching past this machine — warning (§状態).
    property bool danger: false
    /// The question asks for something (where to send a branch, what to call it): nothing is at stake yet, so it
    /// wears the accent — until what is typed would reach past this machine after all (a name the remote already has).
    property bool neutral: false
    /// The pill's hover tip: that it is held, and what the far side will make of it — what the bar's one line has no
    /// room for (§長押し).
    property string tip: ""
    /// Stands in for a hand on the pill, which headless cannot inject (`HoverToolButton.pointedAt`). Read only where a
    /// real hover is — the pill's wash and its tip — so what a run lights is what a hand lights.
    property bool pointedAt: false
    /// The tip standing: `tipDelayMs` waited out and the words on screen (`publish-tip`).
    readonly property bool tipStanding: acceptPill.ToolTip.visible
    /// The far side could not be read (the remote never answered, or what it named is not in this repository): the
    /// frame goes `warning` and a `!` follows the word, as on the toolbar's pair (デザイン規約 §リモートへ送る). The
    /// word and the gesture stay as they were — what could not be read changes neither what would run nor how it is
    /// pressed.
    property bool alert: false
    /// Whether answering takes a hold (デザイン規約 §進行中・長押しの定数). A hold pill answers only on a fill that
    /// reaches the end — both releases stop there — and letting go part way leaves nothing behind.
    property bool hold: false
    /// How far into the hold the press has got, 0 to 1.
    readonly property alias holdProgress: holdDrive.progress
    /// The gesture the press under way was given (`HoldDriver.armedMs`; the live one between presses). The mark and
    /// the word's colour read this: the answer the question waits on can land under a hand, and a pill that changed
    /// its gesture there would take a click for an answer to a question that now wants a hold (デザイン規約 §長押し).
    readonly property alias armedMs: holdDrive.armedMs
    /// The sweep hand over the question's words, exposed so a run can enter it (as `<card>.background.pad` is).
    property alias pad: askHand
    /// What the question needs to have an answer at all (a chooser, a name box), declared by whoever raises it; the
    /// bar seats it under the words. Null where the pill alone answers.
    ///
    /// **Put down before the next one goes in** (`GraphPane.startAsking`), not at the press: a Loader handed the
    /// component it already has keeps the item, so the next question of the same kind would come up holding what was
    /// typed into the first — and clearing it at the press takes the form out from under a bar still on screen,
    /// dropping the pill and the ✕ (centred against the row) by half its height.
    property Component form: null
    /// The loaded form, so the owner can read what was put into it.
    readonly property alias formItem: formLoader.item
    /// A form that says its answer is finished answers the question, as the pill does (デザイン規約
    /// §立っている質問は 1 か所で聞く). Which keystroke means "finished" is the form's to decide (`AppCombo.submitted`);
    /// whether it may answer is the bar's.
    function answerFromForm() {
        // A key cannot do what the dark pill cannot, and a held question is the pill's alone: the gesture is the
        // intent, and a keystroke is not one (§進行中・長押しの定数).
        if (!bar.answerable || bar.hold)
            return
        bar.confirmed()
    }
    /// `ignoreUnknownSignals`: a form of choosers alone has no keystroke to finish it, and answers with the pill.
    Connections {
        target: formLoader.item
        ignoreUnknownSignals: true
        function onAnswered() { bar.answerFromForm() }
    }
    /// Whether the pill can be pressed yet — false while a form has no answer to give.
    property bool answerable: true
    /// Automation: run the hold to its end without a press behind it.
    function completeHold() {
        holdDrive.begin()
    }
    /// Called off without the hand letting go — the question itself has gone, so the fill blanks (HoldDriver.blank).
    function blankHold() {
        holdDrive.blank()
    }
    HoldDriver {
        id: holdDrive
        holdMs: bar.hold ? Metrics.holdMs : 0
        onFinished: bar.confirmed()
    }

    /// Answered — by the pill, its keys, or the form: the owner runs what the question guarded.
    signal confirmed()
    /// Walked away from — Escape, the ✕, or a click elsewhere.
    signal cancelled()
    /// The way out, and the one body the ✕ and Escape both enter, so a headless run presses what a hand presses — a
    /// run calling a second function that does the same passes for a mark or a key wired to nothing (verify-ui).
    function dismiss() {
        bar.cancelled()
    }

    /// Whether the question stands. **The words stay put when it comes down**: the bar is on screen for the whole
    /// 200ms it spends going back up, and a bar emptied at the press spends it as a blank band in whatever colour its
    /// owner's bindings fell back to. The next question replaces the words (`GraphPane.startAsking` dresses the bar,
    /// then raises it; `stopAsking` only lowers it).
    property bool open: false
    readonly property color tone: bar.danger ? Theme.danger : bar.neutral ? Theme.accent : Theme.warning
    /// The heading in the plain ink instead of the bar's colour, for the one question whose colour follows the answer
    /// being picked in it (`RenameCarryFlow`): the frame and the pill already turn with the chooser, and a heading
    /// turning too reads as the words themselves changing. Every other question says its cost in the heading
    /// (デザイン規約 §状態), so this is off unless a flow asks — and put down as each question is dressed
    /// (`GraphPane.startAsking`).
    property bool plainWords: false
    // Closing blanks the hold: a fill left standing would carry on into whatever is asked next.
    //
    // Opening hands the pill the focus a tick later: the gesture that raised it is still being delivered (a
    // double-click has a release and a second click behind it), and the list takes the focus back if the pill claims
    // it first. A question with a form leaves the focus to the form's own box.
    onOpenChanged: {
        // First, ahead of the height binding: the animation must be armed before the height moves (`travelling`).
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

    clip: true
    color: Theme.bgElevated
    /// The height the words ask for, kept apart from the animated one so automation can tell a bar all the way down
    /// from one on its way — `label` is set a whole opening ahead of any of it being on screen (a shot taken on
    /// `label` shows a red line and no words).
    readonly property real openHeight: bar.open ? askRow.implicitHeight + 2 * Theme.spaceMd : 0
    readonly property bool settled: bar.openHeight > 0 && bar.implicitHeight === bar.openHeight
    /// The other edge: answered and all the way back up (a shot taken on the answer shows the bar half-retracted).
    readonly property bool shut: !bar.open && bar.implicitHeight === 0
    implicitHeight: bar.openHeight
    /// Whether the height on the move is the bar itself coming down or going back up. **Only those two animate**: a
    /// standing bar grows when late words arrive (the remote's answer in the line under the heading), and an animated
    /// height there lags the laid-out words, so the clip shears the bottom of the column off.
    ///
    /// **Raised from the handler, not read off `open`**: a condition on `open` reads the value it holds when the height
    /// arrives, so a bar going out says "not travelling" and leaves the screen in one frame. The handler runs ahead of
    /// the bindings that read the same property (rules-refs/app-ui.md の `onXChanged` の行), so the animation is armed
    /// before the height it carries. Lowered on arrival (`settled` / `shut`).
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
    // The question's words can be swept from the air around them (規約 §右のペインの字は掴める). Under the row, so
    // the pill, the ✕ and the form keep every press they had.
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
            // `push main where?` reads as one sentence, so the chip takes the heading's weight.
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                // The chip stands on the heading's first line (デザイン規約 §立っている質問は 1 か所で聞く): both hang from
                // the top and the shorter drops half the difference.
                CodeChip {
                    id: askCode
                    Layout.alignment: Qt.AlignTop
                    Layout.topMargin: Math.max(0, (askWord.lineHeight - askCode.implicitHeight) / 2)
                    visible: bar.code !== ""
                    word: bar.code
                    tint: bar.tone
                    weight: Font.DemiBold
                }
                // A field (the pad above), wrapped, never cut: the bar grows by the line (デザイン規約
                // §立っている質問は 1 か所で聞く).
                CardText {
                    id: askWord
                    text: bar.label
                    markup: bar.labelWords
                    color: bar.plainWords ? Theme.textPrimary : bar.tone
                    pixelSize: Theme.fontMd
                    weight: Font.DemiBold
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignTop
                    // The other half, **only where a chip stands**: an invisible chip still has a height, and read
                    // unguarded it would drop every question git has no one word for.
                    Layout.topMargin: bar.code === ""
                                      ? 0 : Math.max(0, (askCode.implicitHeight - askWord.lineHeight) / 2)
                }
            }
            // **Hidden when empty**, not merely empty: an empty `CardText` still stands a line high, a blank band under
            // the heading (`RenameCarryFlow`'s question); a layout skips what is not visible, spacing and all.
            CardText {
                text: bar.detail
                visible: bar.detail !== ""
                color: Theme.textSecondary
                pixelSize: Theme.fontSm
                Layout.fillWidth: true
            }
            // Unloaded only as the next question is dressed (`form`).
            Loader {
                id: formLoader
                sourceComponent: bar.form
                Layout.fillWidth: true
                Layout.topMargin: bar.form === null ? 0 : Theme.spaceXs
            }
        }
        // The answer — a click, or a hold where the question asks for one. The only thing on the bar that acts.
        Rectangle {
            id: acceptPill
            /// A hand on the pill, or its stand-in (`AskBar.pointedAt`).
            readonly property bool lit: acceptMouse.containsMouse || bar.pointedAt
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: acceptRow.implicitWidth + 2 * Theme.spaceMd
            implicitHeight: Theme.controlHeight
            radius: Theme.radiusSm
            color: acceptPill.lit && bar.holdProgress === 0 ? Theme.bgHover : "transparent"
            border.color: bar.alert ? Theme.warning : bar.tone
            border.width: Theme.borderWidth
            // One ink for the mark, the word and the chip, so the three cannot come apart. Lifted while the fill runs
            // under it; with nothing to send only the ink drops and the frame stays, so the bar keeps its shape
            // (デザイン規約 §無効).
            //
            // Only a hold pill's word takes the tone; a click pill's stays plain whatever the tone or `alert`
            // (デザイン規約 §立っている質問は 1 か所で聞く).
            readonly property color wordInk:
                !bar.answerable ? Theme.textMuted
                : bar.holdProgress > 0 ? Theme.textOnAccent : bar.armedMs > 0 ? bar.tone : Theme.textPrimary
            // Reachable without a pointer (デザイン規約 §長押し). Closed, it leaves the tab order by going disabled: Qt
            // refuses to clear `activeFocusOnTab` on the item holding the focus, and warns. Disabling takes the focus
            // away first, and the pill draws its own colours, so it looks no different.
            activeFocusOnTab: true
            enabled: bar.open && bar.answerable
            /// The focus this pill holds arrived with a press. Cleared when the focus leaves (closing takes it too).
            property bool tookAPress: false
            /// The focus it holds is the one the bar handed it on opening. **Nobody reached for it**, so no ring: a
            /// blue ring over a warning frame says "the keyboard is here" to a reader who has not touched it. Cleared
            /// when the focus leaves, so a tab back onto the pill rings.
            property bool tookTheOpening: false
            onActiveFocusChanged: {
                if (acceptPill.activeFocus)
                    return
                acceptPill.tookAPress = false
                acceptPill.tookTheOpening = false
                // A key let go after the focus has moved is answered somewhere else (`HoldDriver.focusLost`).
                holdDrive.focusLost()
            }
            // A drawn pill has to name itself.
            Accessible.role: Accessible.Button
            Accessible.name: bar.code !== "" ? bar.code : bar.accept
            Accessible.description: bar.hold ? Words.holdToActivate : ""
            HoldFill {
                progress: bar.holdProgress
                tone: bar.tone
                inset: Theme.borderWidth
            }
            // Outside the frame, and only for focus moved here from the keyboard (デザイン規約 §長押し). **Not
            // `visualFocus`**: that belongs to `Control`, and on this plain rectangle it binds `undefined` and the ring
            // never shows. With a press the only other way in, remembering the press says the same.
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
                // Ahead of the word: how the pill is answered before what answering does (デザイン規約 §長押し).
                HoldIcon {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.verticalCenterOffset: Metrics.opticalDrop
                    visible: bar.armedMs > 0
                    progress: bar.holdProgress
                    tint: acceptPill.wordInk
                }
                // The chip or the word (`code` decides). Only the word takes room in the row: the `!` hangs off the end
                // of its advance into the pill's padding, as the toolbar sets the same pair (デザイン規約 §git 用語のコード表記).
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
            ToolTip.visible: bar.tip !== "" && acceptPill.lit
            ToolTip.delay: Metrics.tipDelayMs
            ToolTip.text: bar.tip
            MouseArea {
                id: acceptMouse
                anchors.fill: parent
                hoverEnabled: true
                // Focus a hand brought wants no ring.
                onPressed: {
                    acceptPill.tookAPress = true
                    // Unconditionally: a plain press is answered from the latch too, since the question's answer can
                    // land while a hand is on the pill (`HoldDriver.armedMs`).
                    holdDrive.begin()
                }
                // `released`, not `clicked`: Qt stops emitting `clicked` once its own press-and-hold timer (800ms) has
                // gone off, so a click pill held down like a hold pill would answer nothing. `containsMouse` keeps a
                // release away from the pill a cancel.
                onReleased: {
                    // Read before the latch opens; nothing where the question changed since the press
                    // (`HoldDriver.stale`) — a click that became a hold under the hand is no answer (デザイン規約 §長押し).
                    const plain = holdDrive.armedMs <= 0 && !holdDrive.stale && containsMouse
                    holdDrive.letUp()
                    if (plain)
                        bar.confirmed()
                }
                // Dragging off calls the hold off (the grab keeps `pressed` true out there). Read off the moves, not
                // `containsPress`: that also falls on the release and would open the latch before `onReleased` read it.
                onPositionChanged: if (!containsMouse) holdDrive.letUp()
                onCanceled: holdDrive.letUp()
            }
            // Space or Enter: held where the question asks for a hold (`HoldDriver.pressKey`), otherwise answered as
            // the key comes back up.
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
        // The way out that can be seen (Escape and a click elsewhere walk away too), a step under what it closes
        // (デザイン規約 §寸法).
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
    // Escape as a shortcut: the focus may be on the pill, the form's box or the graph list (which takes it on a row
    // click and has no Escape of its own). **The condition is this bar's own standing only**: a question takes Escape
    // ahead of the report bar, and that order lives where the two meet
    // (`RepoPage`, デザイン規約 §立っている質問は 1 か所で聞く).
    Shortcut {
        id: escapeKey
        // `sequences`: Cancel is more than one key on some platforms, and `sequence` takes only the first (Qt warns).
        sequences: [StandardKey.Cancel]
        enabled: bar.open
        // The ✕'s own body (`dismiss`).
        onActivated: bar.dismiss()
    }
    /// Automation: whether Escape is this bar's to take now. **Read off the shortcut itself**, so a build where the
    /// key is dead cannot say otherwise (as `QuitWaitDialog.escapes`).
    readonly property alias escapes: escapeKey.enabled
}
