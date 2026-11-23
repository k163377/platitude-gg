pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui
import platitude.ui.bars
import platitude.ui.face
import platitude.ui.words

// The shape every standing question takes (デザイン規約 §可否・警告の出し場所): a bar that comes down from the top of
// the list the question is about and pushes its rows down, so what is being judged stays in sight. Only one question
// stands at a time (the page holds the run it guards), and it waits its turn behind a report standing over the page
// (`held`), so the Escape below is never ambiguous.
Rectangle {
    id: bar

    /// The question's words; they stay set while the bar goes back up (`open`, `held`).
    property string label: ""
    /// A ref name inside the question's words, and the sentence with the name's seat left in it ("%1 where?"). The
    /// name keeps the colour it wears wherever it appears, `labelRefTint` (デザイン規約 §ref の種別). Both empty for a
    /// question that names nothing; `label` then carries the whole of it.
    property string labelSentence: ""
    property string labelRef: ""
    /// That name's own colour: `textLink` for the branch the worktree is on, `textPrimary` for one it is not
    /// (§ref の種別「名前の色 = 在処」).
    property color labelRefTint: Theme.textLink
    /// The heading's words' colour.
    readonly property color headingInk: bar.plainWords ? Theme.textPrimary : bar.tone
    /// The name's colour against those words (`Words.nameInk`, the rule `AppMenuItem.refColor` carries): its own
    /// whatever the heading wears, a plain one plain. The field draws the name in it as code (`headingMarks`; the rule
    /// taken again on its answer gives the same colour).
    readonly property color labelTint: Words.nameInk(bar.headingInk, bar.labelRefTint)
    /// The mark that name wears in front of it (デザイン規約 §ref の種別「名前の印」); empty for none.
    property string labelRefMark: ""
    /// The names of a heading in plain words (no `labelSentence`), as the field takes them (`CardText.marks`,
    /// `Words.marksOf`).
    property var labelMarks: []
    /// The heading's names as the field takes them: the one `labelSentence` seats — behind its mark if it has one, in
    /// its own colour and cut at the name ceiling either way (`CardText.holdsNames`) — or `labelMarks`.
    readonly property var headingMarks: bar.labelSentence !== ""
                                        ? [[bar.labelSentence.indexOf("%1"), bar.labelRefMark, bar.labelTint,
                                            bar.labelRef.length]]
                                        : bar.labelMarks
    /// What answering costs, in the one line §用語 allows for it.
    property string detail: ""
    /// The names it says, each behind its mark — as `labelMarks`.
    property var detailMarks: []
    /// The names the pill's tip says, each behind its mark (`TipAsk.marksNow`).
    property var tipMarks: []
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
    /// room for (§回数押し).
    property string tip: ""
    /// Stands in for a hand on the pill, which headless cannot inject (`HoverToolButton.pointedAt`). Read only where a
    /// real hover is — the pill's wash and its tip — so what a run lights is what a hand lights.
    property bool pointedAt: false
    /// The tip standing: `tipDelayMs` waited out and the words on screen (`publish-tip`).
    readonly property bool tipStanding: askBody.item !== null && askBody.item.tipShown
    /// The far side could not be read (the remote never answered, or what it named is not in this repository): the
    /// frame goes `warning` and a `!` follows the word, as on the toolbar's pair (デザイン規約 §リモートへ送る). The
    /// word and the gesture stay as they were — what could not be read changes neither what would run nor how it is
    /// pressed.
    property bool alert: false
    /// Whether answering takes the warning's count of presses (デザイン規約 §回数押し). Such a pill answers only at its
    /// last press, and presses given up part way leave nothing behind.
    property bool hold: false
    /// …and whether that hold is the plain delete's instead: two presses, the word in the plain ink while the mark and
    /// the fill carry the warning (§回数押しの色) — on a bar of any tone, whose frame keeps it.
    property bool plainHold: false
    /// What the hold wears: the plain ink for the plain delete (`HoldIcon` / `HoldFill` lift it to the warning
    /// themselves), the bar's tone otherwise.
    readonly property color holdTone: bar.plainHold ? Theme.textPrimary : bar.tone
    readonly property int holdPresses: !bar.hold ? 0 : bar.plainHold ? Metrics.holdPressesPlain
                                                                     : Metrics.holdPressesWarned
    /// How far into its presses the pill has got, 0 to 1.
    readonly property alias holdProgress: holdDrive.progress
    /// The gesture the press under way was given (`HoldDriver.armedPresses`; the live one between presses). The mark
    /// and the word's colour read this: the answer the question waits on can land under a hand, and a pill that changed
    /// its gesture there would take a click for an answer to a question that now wants a hold (デザイン規約 §回数押し).
    readonly property alias armedPresses: holdDrive.armedPresses
    /// The sweep hand over the question's words, exposed so a run can enter it (as `<card>.background.pad` is).
    /// Each of these is the words' (`askBody`): null until the first question stands.
    readonly property var pad: askBody.item !== null ? askBody.item.pad : null
    /// Automation: where the words and the two controls stand, so a test can read the two columns
    /// (`tst_askseats.qml`), as `NoticeBar.pill` is read.
    readonly property var words: askBody.item !== null ? askBody.item.words : null
    readonly property var heading: askBody.item !== null ? askBody.item.heading : null
    readonly property var headingWords: askBody.item !== null ? askBody.item.headingWords : null
    readonly property var detailLine: askBody.item !== null ? askBody.item.detailLine : null
    readonly property var pill: askBody.item !== null ? askBody.item.pill : null
    readonly property var pillWord: askBody.item !== null ? askBody.item.pillWord : null
    readonly property var closeButton: askBody.item !== null ? askBody.item.closeButton : null
    /// …and whether the heading and the line under it are partly a scroll away (`room`).
    readonly property bool wordsScroll: askBody.item !== null && askBody.item.wordsScroll
    /// What the question needs to have an answer at all (a chooser, a name box), declared by whoever raises it; the
    /// bar seats it under the words. Null where the pill alone answers.
    ///
    /// **Put down before the next one goes in** (`GraphPane.startAsking`), not at the press: a Loader handed the
    /// component it already has keeps the item, so the next question of the same kind would come up holding what was
    /// typed into the first — and clearing it at the press takes the form out from under a bar still on screen, and
    /// the pill standing on its line (`askColumns`) with it.
    property Component form: null
    /// The form is built as its question first comes on screen, and kept until the next question is dressed: every
    /// form's box takes the keyboard as it completes (`PublishForm`), and one built behind a report (`held`) would take
    /// it from that report's `OK`.
    property bool formReached: false
    onFormChanged: bar.formReached = bar.shown
    /// The loaded form, so the owner can read what was put into it.
    readonly property var formItem: askBody.item !== null ? askBody.item.formItem : null
    /// A form that says its answer is finished answers the question, as the pill does (デザイン規約
    /// §立っている質問は 1 か所で聞く; user decision — ask before changing). Which keystroke means "finished" is the
    /// form's to decide (`AppCombo.submitted`); whether it may answer is the bar's. Needed because a question with a
    /// form gives the focus to the form's box, not the pill (typing is the first thing to do there), so the pill's own
    /// Enter never reaches it; what runs is the pill's.
    function answerFromForm() {
        // A key cannot do what the dark pill cannot, and a held question is the pill's alone: the gesture is the
        // intent, and a keystroke is not one (§進行中・回数押しの定数).
        if (!bar.answerable || bar.hold)
            return
        bar.confirmed()
    }
    /// `ignoreUnknownSignals`: a form of choosers alone has no keystroke to finish it, and answers with the pill.
    Connections {
        target: bar.formItem
        ignoreUnknownSignals: true
        function onAnswered() { bar.answerFromForm() }
    }
    /// Whether the pill can be pressed yet — false while a form has no answer to give.
    property bool answerable: true
    /// The pill cannot be pressed: its word greys, and the press mark keeps the bar's tone a step down
    /// (`AppMenuItem.holdOff`).
    readonly property bool holdOff: !bar.answerable && bar.holdProgress <= 0
    /// Automation: every press at once without a hand behind them.
    function completeHold() {
        holdDrive.runOut()
    }
    /// Called off without the hand letting go — the question itself has gone, so the fill blanks (HoldDriver.blank).
    function blankHold() {
        holdDrive.blank()
    }
    readonly property HoldDriver holdDrive: HoldDriver {
        id: holdDrive
        holdPresses: bar.holdPresses
        // At the last press, while the question is still the one pressed for. The owner lowers the bar as it answers;
        // the bar stays down for the floor regardless (`onFloor`).
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
    /// `animStandardMs` it spends going back up, and a bar emptied at the press spends it as a blank band in whatever
    /// colour its owner's bindings fell back to. The next question replaces the words (`GraphPane.startAsking` dresses the bar,
    /// then raises it; `stopAsking` only lowers it).
    property bool open: false
    /// The question waits its turn: a report stands over the middle of the page, and one bar comes down there at a
    /// time, the upper first (`RepoPage`). It still stands — its words, its form and what was typed into it, its run —
    /// but nothing of it is on screen, marks its row or takes a key until the report is gone; then it takes the
    /// report's place (`enterFrom`).
    property bool held: false
    /// Something waits behind the question to take the middle once it is answered — the discard band, or a report
    /// waiting for the answer under way (`RepoPage.centreColumn`, `answering`): it goes in one frame, as it does for a
    /// report coming over it (`held`), and what comes takes its place at its height
    /// (`BarTravel`, `handingOver`). Only a question with nothing before or after it comes down from the top or goes
    /// back up.
    property bool givesWay: false
    /// The question leaves the middle in this frame for what comes after it (`held` / `givesWay`), standing at `height`
    /// until now: the page stands that there and keeps the graph's rows where they are (`RepoPage.askHandsOver`).
    /// Said wherever it was — standing, on the floor of a pill pressed through, or on its way back up.
    signal handingOver(real height)
    /// The height of the bar this question takes the place of, gone in the same frame (`givesWay`). Read once and put
    /// down; 0 for a question coming down from nothing.
    property real enterFrom: 0
    /// Whether the question is on screen: standing, and its turn. What the bar draws, keys and measures by; `open` is
    /// the owner's.
    readonly property bool shown: bar.open && !bar.held
    readonly property color tone: bar.danger ? Theme.danger : bar.neutral ? Theme.accent : Theme.warning
    /// The heading in the plain ink instead of the bar's colour, for the question whose colour follows the answer
    /// being picked in it (`RenameCarryFlow`): the frame and the pill already turn with the chooser, and a heading
    /// turning too reads as the words themselves changing. And for the plain delete's question (`UpstreamFlow`'s
    /// unset), whose words are as plain as its pill's (`plainHold`). Every other question says its cost in the heading
    /// (デザイン規約 §状態), so this is off unless a flow asks — and put down as each question is dressed
    /// (`GraphPane.startAsking`).
    property bool plainWords: false
    // Another question's words start at their first line, wherever the last ones were scrolled to: the heading's line
    // is the one a short column always keeps (デザイン規約 §立っている質問は 1 か所で聞く).
    onLabelChanged: if (askBody.item !== null) askBody.item.toTop()
    onDetailChanged: if (askBody.item !== null) askBody.item.toTop()
    // The next question, raised on the last one's floor: that answer has gone, and its fill is not this pill's. Put
    // down as it is raised, ahead of `shown`: a report waiting for that answer comes now (`answering`,
    // `RepoPage.raiseNotice`), and this question waits behind it.
    onOpenChanged: if (bar.open && holdDrive.done) holdDrive.settle()
    // Going off screen blanks the hold: a fill left standing would carry on into whatever is asked next.
    //
    // Coming on screen hands the pill the focus a tick later: the gesture that raised it is still being delivered (a
    // double-click has a release and a second click behind it), and the list takes the focus back if the pill claims it
    // first. A question with a form leaves the focus to the form's own box — which a form built before a report came
    // over it is handed back, the report's `OK` having taken it (`held`).
    onShownChanged: {
        if (bar.shown) {
            // First, ahead of the height: the animation must be armed before the height moves (`travelling`). A bar
            // already standing — raised on the last one's floor — never went up, so nothing comes down either: words
            // arriving later grow it unanimated.
            bar.travelling = !bar.standing
            const formBack = bar.formItem !== null
            // The form and, for the first question, the words (`askBody`), ahead of the height too.
            bar.formReached = true
            askBody.active = true
            bar.standing = true
            if (bar.travelling && bar.enterFrom > 0)
                BarTravel.standThenTravel(bar, bar.enterFrom, () => bar.openHeight)
            bar.enterFrom = 0
            // A bar built open hears this while it is still completing, its words not built yet: it takes the
            // opening as it completes, once its form is bound (`Component.onCompleted`).
            if ((bar.form === null || formBack) && askBody.item !== null)
                askBody.item.takeOpening()
        } else {
            bar.blankHold()
            // A pill pressed through keeps the bar down for its floor: it lets go as the floor passes (`onFloor`).
            if (!bar.onFloor)
                bar.leave()
        }
    }
    /// The bar stands on screen at its words' height: from the moment it is shown until it lets go (`leave`) — through
    /// the floor of a pill pressed through, after it is answered (デザイン規約 §回数押し).
    property bool standing: false
    /// The full fill is still on screen after the last press (`HoldDriver.settling`).
    readonly property bool onFloor: holdDrive.settling
    // Read off `open` and `held`, not `shown`: a floor put down by the next question being raised ends while `shown` is
    // still the last one's (`onOpenChanged`).
    onOnFloorChanged: if (!bar.onFloor && !(bar.open && !bar.held)) bar.leave()
    /// An answer is under way on a pill that takes presses: being pressed through, or its full fill standing after the
    /// last press (`HoldDriver.gesturing`) — which only a standing question's pill takes. It keeps the middle — a
    /// report waits until it is over, then takes the question's place (`RepoPage.showNotice`; user decision — ask
    /// before changing). A plain pill answers at once. Read off the gesture alone: what the page does as it ends moves
    /// `standing`, and the count is the one the gesture began with (`armedPresses`) — a pill turned into a count under
    /// a hand already down is still a plain press, and the next question's count, dressed on this one's floor, is not
    /// this answer's.
    readonly property bool answering: bar.armedPresses > 0 && holdDrive.gesturing
    /// [`leave`] is under way.
    property bool leaving: false
    /// Lets go of the middle: back up, or — something coming after it (`held` / `givesWay`) — gone in this frame,
    /// saying where it stood (`handingOver`). The animation is armed ahead of the height (`travelling`). Once: what the
    /// page brings in as it hears where the bar stood (a report, `held`) finds it already going (`leaving`).
    function leave() {
        if (!bar.standing || bar.leaving)
            return
        bar.leaving = true
        const atOnce = bar.held || bar.givesWay
        if (atOnce && bar.implicitHeight > 0)
            bar.handingOver(bar.implicitHeight)
        bar.travelling = !atOnce
        bar.standing = false
        bar.leaving = false
    }
    // A report coming over whatever of the question is on screen takes the middle in this frame, at its height
    // (`RepoPage.raiseNotice`): a question standing goes at once, and one answered and on its way back up goes from
    // where it is. One on its floor stands until the floor passes (`answering`), then goes (`onOnFloorChanged`).
    onHeldChanged: {
        if (!bar.held || bar.shut || bar.onFloor)
            return
        if (bar.standing) {
            bar.leave()
        } else {
            bar.handingOver(bar.implicitHeight)
            BarTravel.cut(bar, () => bar.openHeight)
        }
    }

    clip: true
    color: Theme.bgElevated
    /// The most the bar may stand at: what its column leaves it with the graph's rows kept, the one bar standing there
    /// (`RepoPage.centreColumn`, デザイン規約 §窓の床). Past it the heading and the line under it scroll, a line of them at the
    /// least; the form, the pill and the ✕ stay where they are.
    property real room: Infinity
    /// The heading's one line with the code chip on it, the two on one baseline (`headingRow`), whatever the question
    /// says: a chip's height is its face's, and the line's is read off a line that sets a name (`lineRuler`).
    readonly property real chipLine: Math.max(
        lineRuler.setLineHeight + Math.max(0, Math.round(chipRuler.baselineOffset - lineRuler.baselineOffset)),
        chipRuler.implicitHeight + Math.max(0, Math.round(lineRuler.baselineOffset - chipRuler.baselineOffset)))
    /// The least the window's floor keeps for a question, whichever stands: a line of its words with a chip on it, a
    /// form's row of boxes (`controlHeight`, as every form's are) and the pill on a line of its own under them, as the
    /// graph at its least stands it (`PageLayout.panesMinHeight`).
    readonly property real floorHeight: Math.ceil(2 * Theme.spaceMd + bar.chipLine + 2 * Theme.spaceXs
                                                  + Theme.controlHeight + Theme.spaceSm + Theme.controlHeight)
    /// The height the words ask for, kept apart from the animated one so automation can tell a bar all the way down
    /// from one on its way — `label` is set a whole opening ahead of any of it being on screen (a shot taken on
    /// `label` shows a red line and no words).
    ///
    /// A pill just pressed through keeps the bar down for its floor, the question answered or not: the full fill is
    /// what the answer looks like (デザイン規約 §回数押し).
    ///
    /// The words wrap and the bar grows by the line (`NoticeBar` alike), the page's centre giving up as much: Layouts
    /// have no height-for-width, but a `Text` / `CardText` answers a new `implicitHeight` once its width is set and the
    /// column polishes again (as `CommitHoverCard`'s `CardText`). The width decides it, so a run shoots the wrap in a
    /// window whose graph is squeezed to its least (`--config-dir` holding a large `[layout] details_width`).
    readonly property real openHeight: bar.standing && askBody.item !== null
                                       ? askBody.item.columnsHeight + 2 * Theme.spaceMd : 0
    readonly property bool settled: bar.openHeight > 0 && bar.implicitHeight === bar.openHeight
    /// The other edge: answered, or waiting its turn (`held`), and all the way back up or gone (a shot taken on the
    /// answer shows the bar half-retracted).
    readonly property bool shut: !bar.shown && bar.implicitHeight === 0
    implicitHeight: bar.openHeight
    /// Whether the height on the move is the bar itself coming on or going off — down from nothing, from the height of
    /// the bar it takes the place of (`enterFrom`), or back up. **Only those animate**: a standing bar grows when late
    /// words arrive (the remote's answer in the line under the heading), and an animated height there lags the laid-out
    /// words, so the clip shears the bottom of the column off.
    ///
    /// **Raised ahead of the height, not read off `shown`**: a condition on `shown` would hold the value it has when
    /// the height arrives, so a bar going out would say "not travelling" and leave the screen in one frame. The height
    /// follows `standing`, which `onShownChanged` and `leave` write after setting this. Lowered on arrival (`settled` /
    /// `shut`).
    property bool travelling: false
    onSettledChanged: if (bar.settled) bar.travelling = false
    onShutChanged: if (bar.shut) bar.travelling = false
    Behavior on implicitHeight {
        enabled: bar.travelling
        NumberAnimation { duration: Metrics.animStandardMs }
    }
    BandRule {
        color: bar.tone
    }
    // A line of the heading with a name on it, never shown: a name is set in git's face beside the words' and stands
    // the line taller than words alone, so a field's own line moves with what it says (`CardText.lineHeight`). What a
    // line stands at whatever the question says — the least the column and the window's floor keep — is read here
    // (`CardText.setLineHeight`).
    CardText {
        id: lineRuler
        visible: false
        text: "main"
        marks: [[0, "branch", Theme.textLink, 4]]
        holdsNames: true
        pixelSize: Theme.fontMd
        weight: Theme.fontWeightStrong
    }
    // The heading's code chip as the floor reads it (`chipLine`), never shown: a chip's line is its word's face's,
    // whatever word it holds, so the floor stands before any question has (`askBody`).
    CodeChip {
        id: chipRuler
        visible: false
        weight: Theme.fontWeightStrong
    }
    // The words, the form, the pill and the ✕, built by the first question and kept (rules-refs/app-ui.md — the
    // `WindowDialogSeat` line): most pages never ask one, and a question going back up keeps its words while the bar
    // travels (`shown`). What the column and the window's floor read stays out here (`floorHeight` off the rulers).
    Loader {
        id: askBody
        anchors.fill: parent
        active: false
        // Shut — gone up, or waiting behind the report — it takes no key: a form built before a report came over it
        // keeps its box, which Tab from the report's `OK` would otherwise walk into.
        enabled: !bar.shut
        sourceComponent: Item {
            /// What the bar reads off its words (`pad` … `formItem`, `tipStanding`, `openHeight`).
            readonly property real columnsHeight: askColumns.implicitHeight
            readonly property alias pad: askHand
            readonly property alias words: askWords
            readonly property alias heading: headingRow
            readonly property alias headingWords: askWord
            readonly property alias detailLine: detailWord
            readonly property alias pill: acceptPill
            readonly property alias pillWord: wordSeat
            readonly property alias closeButton: dismissButton
            readonly property bool wordsScroll: askLines.scrolls
            readonly property var formItem: formLoader.item
            readonly property bool tipShown: pillTip.shown
            function toTop() {
                askLines.toTop()
            }
            /// The pill takes the focus a tick later: the gesture that raised the bar is still being delivered. A form
            /// already built takes it instead, back in the box it was in (the loader is a focus scope and keeps it).
            function takeOpening() {
                if (formLoader.item !== null) {
                    Qt.callLater(formLoader.forceActiveFocus)
                    return
                }
                acceptPill.tookTheOpening = true
                Qt.callLater(acceptPill.forceActiveFocus)
            }
            // The question's words can be swept from the air around them (規約 §右のペインの字は掴める). Under the columns, so the
            // pill, the ✕ and the form keep every press they had. Its content is the words' column (`askWords`), so the
            // form's boxes, the pill and the ✕ are never walked. Headless: `ask-sweep`.
            SweepPad {
                id: askHand
                anchors.fill: parent
                content: askWords
            }
            // Two columns (デザイン規約 §立っている質問は 1 か所で聞く; user decision — ask before changing). On the left the words — the
            // heading, the line under it and the form, whichever stand, packed from the top — and on the right the two
            // controls, their right edges on one line: the ✕ at the top, its mark's top on the heading's, and the pill
            // at the bottom. Every row of the words keeps the right column's room. The pill's foot is the words' foot,
            // so a form's boxes, the pill's height, share its line. Words that end higher than the ✕ with the pill
            // `spaceXs` under its seat — a heading and its line, or a heading alone that wraps — leave the right column
            // to set the height, and stay at the top.
            //
            // Words that cannot keep a pair of boxes' floors beside the pill (the graph at its least) put the pill on a
            // line of its own under them, its right edge still the ✕'s (user decision — ask before changing): the
            // heading and the ✕, then the line under it and the form, then the pill. The right column is then the ✕
            // alone: the heading keeps the ✕'s room and every other row takes the whole width — a line keeping room for
            // a pill that is not beside it breaks early for nothing.
            //
            // A heading alone — no line under it, no form — that would stand on one line in the two columns puts the
            // pill and the ✕ beside it on one row (user decision — ask before changing): in two columns the pill would
            // stand a row under the only words there are. The ✕ keeps its corner and the pill stands left of it, a step
            // from its ink; the heading's first line stands on the pill's word and the ✕'s mark on the pill's middle,
            // so the few widths where the ✕'s extra room wraps the heading keep the pill and the ✕ on its first line. A
            // heading that wraps in the two columns anyway keeps them.
            Item {
                id: askColumns
                /// What the words keep beside the pill at least, every question alike: a pair of boxes on their floor —
                /// what a form names (`PublishForm.leastWidth`), else two `askFieldMinW` (a chooser alone names none).
                /// Narrower, a heading behind a code chip breaks inside its words, and a form's boxes give way below
                /// their floor.
                readonly property real wordsLeast: formLoader.item !== null && formLoader.item.leastWidth !== undefined
                                                   ? formLoader.item.leastWidth : 2 * Metrics.askFieldMinW
                readonly property bool pillUnder:
                    askColumns.width - acceptPill.width - Theme.spaceMd < askColumns.wordsLeast
                /// The right column's least height: the ✕'s seat, `spaceXs`, and the pill.
                readonly property real controlsHeight: dismissButton.y + dismissButton.height + Theme.spaceXs
                                                       + acceptPill.height
                /// The air the ✕'s mark holds inside its box, asked of the mark (デザイン規約 §余白): the pill beside it on one
                /// row counts it into its step. `NavIcon.inkRight` is arithmetic, so it answers with no scene around
                /// it.
                readonly property NavIcon closeInk: NavIcon {
                    kind: "close"
                    width: dismissButton.markSize
                    height: dismissButton.markSize
                }
                readonly property real closeAir: askColumns.closeInk.width - askColumns.closeInk.inkRight
                /// Where the pill stands on one row: `spaceSm` short of the ✕'s ink, whose box ends at the corner.
                readonly property real rowPillX: askColumns.width - dismissButton.markSize + askColumns.closeAir
                                                 - Theme.spaceSm - acceptPill.width
                /// Decided on the heading's unwrapped width (`CardText.implicitWidth`) against the two columns' words,
                /// which neither layout moves.
                readonly property bool oneRow:
                    !askColumns.pillUnder && bar.detail === "" && bar.form === null
                    && headingRow.implicitWidth <= askColumns.width - acceptPill.width - Theme.spaceMd
                /// The heading's first baseline and the pill's word's, each from its own top. The heading's is its
                /// words' where `headingRow` stands them (a chip standing deeper drops them to its baseline); the
                /// pill's is its word's in the middle of the pill.
                readonly property real headingBase: askWord.baselineOffset
                    + (bar.code === "" ? 0 : Math.max(0, Math.round(askCode.baselineOffset - askWord.baselineOffset)))
                readonly property real pillBase: acceptRow.y + wordSeat.y + wordSeat.base
                /// On one row the two stand on one baseline (rules-refs/app-ui.md「`CardText` を語と同じ行に置く所は、 ベースラインを揃える」):
                /// both hang from the top and the one whose word stands higher drops by the difference, in whole
                /// pixels. One value each, which the ✕ reads too, so nothing on the row reads another item's place.
                readonly property real rowHeadingY:
                    Math.max(0, Math.round(askColumns.pillBase - askColumns.headingBase))
                readonly property real rowPillY: Math.max(0, Math.round(askColumns.headingBase - askColumns.pillBase))
                /// That row's height: down to the heading's foot or the pill's, whichever is lower.
                readonly property real rowHeight: Math.max(askColumns.rowHeadingY + headingRow.implicitHeight,
                                                           askColumns.rowPillY + acceptPill.height)
                /// What the heading and the line under it may stand at: `room` less the form and a pill under the
                /// words, in whole lines of the heading, down — a line cut through its waist under the first reads as a
                /// stray mark, and a part of one past the room comes off the graph. A line at the least. No line
                /// measured yet (the ruler is still being made), no cap: an unbounded room over a 0 is no number.
                readonly property real headingLine: bar.code === "" ? lineRuler.setLineHeight : bar.chipLine
                readonly property real linesCap: {
                    const line = askColumns.headingLine
                    const left = Math.floor(bar.room - 2 * Theme.spaceMd
                                            - (askColumns.pillUnder ? Theme.spaceSm + acceptPill.height : 0)
                                            - (bar.form !== null ? 2 * Theme.spaceXs + formLoader.implicitHeight : 0))
                    return line > 0 ? Math.max(1, Math.floor(left / line)) * line : Infinity
                }
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: Theme.spaceMd
                // Under the words, the pill stands a form's gap below them (the line over a form is the same `spaceXs`
                // twice).
                implicitHeight: askColumns.pillUnder ? askWords.implicitHeight + Theme.spaceSm + acceptPill.height
                                : askColumns.oneRow ? askColumns.rowHeight
                                : Math.max(askWords.implicitHeight, askColumns.controlsHeight)
                ColumnLayout {
                    id: askWords
                    anchors.left: parent.left
                    anchors.right: parent.right
                    // The right column (the pill is wider than the ✕'s mark) and the gap in front of it — none where
                    // the pill stands under the words, and the heading keeps the ✕'s room alone. On one row, up to the
                    // pill.
                    anchors.rightMargin: askColumns.pillUnder ? 0
                                         : askColumns.oneRow ? askColumns.width - askColumns.rowPillX + Theme.spaceMd
                                         : acceptPill.width + Theme.spaceMd
                    anchors.top: parent.top
                    anchors.topMargin: askColumns.oneRow ? askColumns.rowHeadingY : 0
                    spacing: Theme.spaceXs
                    // The heading and the line under it, which a column short of room scrolls rather than cut (`room`).
                    // The bar stands in the gap before the right column, or in the bar's own margin where the words
                    // take the whole width.
                    CappedLines {
                        id: askLines
                        Layout.fillWidth: true
                        cap: askColumns.linesCap
                        gap: Theme.spaceMd
                        spacing: Theme.spaceXs
                        // `push main where?` reads as one sentence, so the chip takes the heading's weight.
                        RowLayout {
                            id: headingRow
                            Layout.fillWidth: true
                            Layout.rightMargin: askColumns.pillUnder ? Theme.iconSm + Theme.spaceMd : 0
                            spacing: Theme.spaceXs
                            // The chip stands on the heading's first line, where the sentence starts: floated beside
                            // the middle of a wrapped heading it reads as a word of its own, not part of the sentence.
                            // Both hang from the top and the one whose word stands higher drops to the other's
                            // baseline, in whole pixels. Centred on the line, the chip's word stands under the
                            // heading's by half the leading a field files under its line (2〜3px on macOS); on the row's
                            // `Qt.AlignBaseline` the two faces' depths leave the heading on a half pixel
                            // (rules-refs/app-ui.md「その組は自分だけの `RowLayout` に入れる」).
                            CodeChip {
                                id: askCode
                                Layout.alignment: Qt.AlignTop
                                Layout.topMargin:
                                    Math.max(0, Math.round(askWord.baselineOffset - askCode.baselineOffset))
                                visible: bar.code !== ""
                                word: bar.code
                                tint: bar.tone
                                weight: Theme.fontWeightStrong
                            }
                            // A field (the pad above), wrapped, never cut but for a name past the name ceiling, given
                            // back whole under the hand: the bar grows by the line and the list goes down with it
                            // (デザイン規約 §立っている質問は 1 か所で聞く). The name is cut in its middle (user decision — ask before
                            // changing), a remote's at its end (`CardText.holdsNames`, デザイン規約 §寸法): a question names a
                            // branch, a remote or a worktree's folder — names only this window shows — and with the
                            // tail gone the reader cannot tell what the question is about.
                            CardText {
                                id: askWord
                                text: bar.label
                                marks: bar.headingMarks
                                holdsNames: true
                                color: bar.headingInk
                                pixelSize: Theme.fontMd
                                weight: Theme.fontWeightStrong
                                Layout.fillWidth: true
                                Layout.alignment: Qt.AlignTop
                                // The other half, **only where a chip stands**: an invisible chip still has a baseline,
                                // and read unguarded it would drop every question git has no one word for.
                                Layout.topMargin: bar.code === ""
                                    ? 0 : Math.max(0, Math.round(askCode.baselineOffset - askWord.baselineOffset))
                            }
                        }
                        // **Hidden when empty**, not merely empty: an empty `CardText` still stands a line high, a
                        // blank band under the heading (`RenameCarryFlow`'s question); a layout skips what is not
                        // visible, spacing and all. Its names cut at the name ceiling as the heading's are, the whole
                        // line under the hand (user decision — ask before changing): a name of a hundred letters
                        // otherwise breaks over three lines at its dashes.
                        CardText {
                            id: detailWord
                            text: bar.detail
                            marks: bar.detailMarks
                            holdsNames: true
                            visible: bar.detail !== ""
                            color: Theme.textSecondary
                            pixelSize: Theme.fontSm
                            Layout.fillWidth: true
                        }
                    }
                    // Unloaded only as the next question is dressed (`form`). Its boxes stand `controlHeight` tall, the
                    // pill's height (`PublishForm`, `UpstreamForm`, `RenameCarryForm`). Hidden with no form, as the
                    // line above is: an empty one still takes the column's spacing, and the pill's foot would stand
                    // that far under the words.
                    Loader {
                        id: formLoader
                        visible: bar.form !== null
                        active: bar.formReached
                        sourceComponent: bar.form
                        // The column's width from the start, not from the column's first layout: until then a Loader of
                        // no width leaves the form at its own implicit width, which the form's boxes work out from that
                        // width (`PublishForm.askRow`) — a pass that grows the form by a pixel, over and over.
                        width: askWords.width
                        Layout.fillWidth: true
                        Layout.topMargin: Theme.spaceXs
                    }
                }
                // The answer — a click, or a hold where the question asks for one. The only thing on the bar that acts.
                Rectangle {
                    id: acceptPill
                    /// A hand on the pill, or its stand-in (`AskBar.pointedAt`).
                    readonly property bool lit: acceptMouse.containsMouse || bar.pointedAt
                    /// The names its tip says, behind their marks (`TipAsk.marksNow`).
                    readonly property var tipMarks: bar.tipMarks
                    // The right column's foot (`askColumns`), or left of the ✕ on one row.
                    x: askColumns.oneRow ? askColumns.rowPillX : askColumns.width - acceptPill.width
                    y: askColumns.oneRow ? askColumns.rowPillY : askColumns.height - acceptPill.height
                    implicitWidth: acceptRow.implicitWidth + 2 * Theme.spaceMd
                    implicitHeight: Theme.controlHeight
                    radius: Theme.radiusSm
                    color: acceptPill.lit && bar.holdProgress === 0 ? Theme.bgHover : "transparent"
                    border.color: bar.alert ? Theme.warning : bar.tone
                    border.width: Theme.borderWidth
                    // One ink for the mark, the word and the chip, so the three cannot come apart. Lifted while the
                    // fill runs under it; with nothing to send only the ink drops and the frame stays, so the bar keeps
                    // its shape (デザイン規約 §無効).
                    //
                    // Only a hold pill's word takes the tone; a click pill's stays plain whatever the tone or `alert`
                    // (デザイン規約 §立っている質問は 1 か所で聞く).
                    readonly property color wordInk:
                        !bar.answerable ? Theme.textMuted
                        : bar.holdProgress > 0 ? Theme.textOnAccent
                        : bar.armedPresses > 0 ? bar.holdTone : Theme.textPrimary
                    // Reachable without a pointer (デザイン規約 §回数押し). Closed, it leaves the tab order by going disabled: Qt
                    // refuses to clear `activeFocusOnTab` on the item holding the focus, and warns. Disabling takes the
                    // focus away first, and the pill draws its own colours, so it looks no different.
                    activeFocusOnTab: true
                    enabled: bar.shown && bar.answerable
                    /// The focus this pill holds arrived with a press. Cleared when the focus leaves (closing takes it
                    /// too).
                    property bool tookAPress: false
                    /// The focus it holds is the one the bar handed it on opening. **Nobody reached for it**, so no
                    /// ring: a blue ring over a warning frame says "the keyboard is here" to a reader who has not
                    /// touched it. Cleared when the focus leaves, so a tab back onto the pill rings.
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
                    Accessible.description: bar.hold ? Words.pressToActivate(bar.holdPresses) : ""
                    HoldFill {
                        progress: bar.holdProgress
                        tone: bar.holdTone
                        dimmed: holdDrive.dimmed
                        inset: Theme.borderWidth
                    }
                    // Outside the frame, and only for focus moved here from the keyboard (デザイン規約 §回数押し). **Not
                    // `visualFocus`**: that belongs to `Control`, and on this plain rectangle it binds `undefined` and
                    // the ring never shows. With a press the only other way in, remembering the press says the same.
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
                        // Ahead of the word: how the pill is answered before what answering does (デザイン規約 §回数押し). A
                        // letter of the pill's words (デザイン規約 §アイコンの席 の名指し): as tall as their capitals and on their
                        // middle, in the middle of the seat of their letters, the pill's side given back — the padding
                        // already parts the mark from the frame (`MarkSeat.leadWidth` / `leadX`).
                        Item {
                            id: acceptHoldSeat
                            visible: bar.armedPresses > 0
                            width: MarkSeat.leadWidth(acceptHold.size, acceptRow.spacing,
                                                      bar.code !== "" ? Theme.spaceXs / 2 : 0)
                            height: wordSeat.implicitHeight
                            HoldIcon {
                                id: acceptHold
                                x: MarkSeat.leadX(acceptHold.size, acceptHold.width)
                                y: Math.round(wordSeat.y + wordSeat.base - WordFace.drawnCapitalMiddle(acceptHold.size)
                                              - acceptHold.height / 2)
                                size: Theme.fontMd
                                progress: bar.holdProgress
                                presses: bar.armedPresses
                                off: bar.holdOff
                                tint: bar.holdOff ? bar.holdTone : acceptPill.wordInk
                            }
                        }
                        // The chip or the word (`code` decides). Only the word takes room in the row: the `!` hangs off
                        // the end of its advance into the pill's padding, as the toolbar sets the same pair (デザイン規約
                        // §git 用語のコード表記).
                        Item {
                            id: wordSeat
                            /// The word's baseline, in this seat's y: what the hold mark heads.
                            readonly property real base: bar.code !== "" ? acceptCode.baselineOffset
                                                                         : acceptWord.baselineOffset
                            anchors.verticalCenter: parent.verticalCenter
                            implicitWidth: bar.code !== "" ? acceptCode.implicitWidth : acceptWord.implicitWidth
                            implicitHeight: bar.code !== "" ? acceptCode.implicitHeight : acceptWord.implicitHeight
                            CodeChip {
                                id: acceptCode
                                visible: bar.code !== ""
                                word: bar.code
                                tint: acceptPill.wordInk
                                buttonWord: true
                            }
                            PlainTextLabel {
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
                    readonly property TipAsk pillTip: TipAsk {
                        id: pillTip
                        target: acceptPill
                        // Flush under the pill: over it, or beside it on one row, stand the heading and the ✕. Its foot
                        // is at or under the words' foot (`askColumns`), so nothing of the bar's words is under it
                        // (`TipAsk.under`) — but on one row whose heading wraps, where the pill stands on its first
                        // line: no question with a tip is a heading alone (each says its cost on a line or carries a
                        // form).
                        under: true
                        asked: bar.tip !== "" && acceptPill.lit
                        words: bar.tip
                    }
                    MouseArea {
                        id: acceptMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        // Focus a hand brought wants no ring.
                        onPressed: {
                            acceptPill.tookAPress = true
                            // Unconditionally: a plain press is answered from the latch too, since the question's
                            // answer can land while a hand is on the pill (`HoldDriver.armedPresses`).
                            holdDrive.begin()
                        }
                        // `released`, not `clicked`: Qt stops emitting `clicked` once its own press-and-hold timer
                        // (800ms) has gone off, so a click pill kept down a while would answer nothing. `containsMouse`
                        // keeps a release away from the pill a cancel.
                        onReleased: {
                            // Read before the latch opens; nothing where the question changed since the press
                            // (`HoldDriver.stale`) — a click that became a count under the hand is no answer (デザイン規約
                            // §回数押し).
                            const plain = holdDrive.armedPresses <= 0 && !holdDrive.stale && containsMouse
                            holdDrive.letUp()
                            if (plain)
                                bar.confirmed()
                        }
                        // Dragging off calls the presses off (the grab keeps `pressed` true out there). Read off the
                        // moves, not `containsPress`: that also falls on the release and would open the latch before
                        // `onReleased` read it.
                        onPositionChanged: if (!containsMouse) holdDrive.leave()
                        // The hand left the pill part way through its presses: back to zero (`HoldDriver.leave`).
                        onExited: holdDrive.leave()
                        onCanceled: holdDrive.letUp()
                    }
                    // Space or Enter: counted where the question asks for presses (`HoldDriver.pressKey`), otherwise
                    // answered as the key comes back up.
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
                // The way out that can be seen (Escape and a click elsewhere walk away too), a step under what it
                // closes, in the bars' one seat (デザイン規約 §寸法). In the top-right corner (user decision — ask before
                // changing): the mark's right edge on the pill's and its top on the heading's — up in the corner, it
                // leaves the pill under it its air instead of sitting on its shoulder. On one row the pill stands left
                // of it instead (`rowPillX`), and its mark is centred on the pill's middle.
                CloseToolButton {
                    id: dismissButton
                    x: askColumns.width - dismissButton.width + (dismissButton.width - dismissButton.markSize) / 2
                    y: askColumns.oneRow
                       ? Math.round(askColumns.rowPillY + (acceptPill.height - dismissButton.height) / 2)
                       : headingRow.y - (dismissButton.height - dismissButton.markSize) / 2
                    onClicked: bar.dismiss()
                }
            }
        }
    }
    // Escape as a shortcut: the focus may be on the pill, the form's box or the graph list (which takes it on a row
    // click and has no Escape of its own). **The condition is this bar being on screen**: the report bar's Escape lives
    // only while the report stands, and this question waits behind it (`held`), so the two never live at once
    // (rules-refs/app-ui.md「Esc は窓に 1 本しか生きられない」).
    Shortcut {
        id: escapeKey
        // `sequences`: Cancel is more than one key on some platforms, and `sequence` takes only the first (Qt warns).
        sequences: [StandardKey.Cancel]
        enabled: bar.shown
        // The ✕'s own body (`dismiss`).
        onActivated: bar.dismiss()
    }
    /// A bar built open heard `shown` change before its words could be built: they stand once it completes, and it
    /// takes the opening then, with its form bound (`tests/qml/tst_askopening.qml`).
    Component.onCompleted: {
        if (!bar.shown)
            return
        bar.formReached = true
        askBody.active = true
        if (bar.form === null)
            askBody.item.takeOpening()
    }
    /// Automation: whether Escape is this bar's to take now. **Read off the shortcut itself**, so a build where the
    /// key is dead cannot say otherwise (as `QuitWaitDialog.escapes`).
    readonly property alias escapes: escapeKey.enabled
}
