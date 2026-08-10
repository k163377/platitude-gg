import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Toolbar action named twice over: an icon to find it by shape, the
// word to be sure of it. Both halves dim together when disabled.
HoverToolButton {
    id: actionBtn
    property string kind: ""
    /// Colour of both halves while the button is live.
    property color tone: Theme.textPrimary
    /// The state's colour taken a step down for the wait — the ring and
    /// the frame wear it while git is on the network (デザイン規約
    /// §暗く落とした段: the hue stays, so what the button is does not
    /// change; only what state it is in).
    ///
    /// The state's, not `tone`'s: where the word stays plain and only the
    /// frame and the mark carry a warning, this follows the frame. While
    /// the wait lasts there is no word on the button to be a step down
    /// from — the ring is standing in for what the frame is saying.
    ///
    /// A plain button has no darker step of its own and takes `textMuted`,
    /// which is where its words go when they are not to be read either.
    property color toneDim: Theme.textMuted
    /// git is on the network for this button: the words step aside for a
    /// turning ring in the middle of the button, and the whole of it goes
    /// as dim and as deaf as a disabled one (デザイン規約 §長押し).
    ///
    /// Dimmed rather than actually disabled: `enabled` would take the
    /// focus away, and the press that started the network call is the
    /// very one that may have come from the keyboard.
    property bool busy: false
    /// Hold the turn still (automation — a spinning icon photographs
    /// differently every time).
    property bool still: false
    /// How long this button has to be held to fire `held()`; zero for an
    /// ordinary button, where a click is the whole gesture (デザイン規約
    /// §進行中・長押しの定数).
    property int holdMs: 0
    /// How far into the hold the press has got, 0 to 1.
    property real holdProgress: 0
    /// Frame drawn around the button. Transparent leaves the button bare.
    property color frameColor: "transparent"
    /// The last go at what this button does did not work. Drawn as a mark
    /// standing clear of the word's last letter, in the word's own colour
    /// — the frame and the colour already say something is wrong, and this
    /// is what says it is about this button rather than about the state
    /// the toolbar is in.
    property bool alert: false
    /// The mark's colour, where it is not the word's. A button whose word
    /// stays plain while only the mark is coloured says "read this" without
    /// saying "this one is dangerous" — the colours a word wears here are
    /// the ones `Remove` and a stopped fetch wear, and they mean something
    /// stronger than a notice.
    property color alertTone: actionBtn.fg
    /// Pull the mark back to a letter's distance from the word.
    ///
    /// It is placed at the end of the label's **advance** width, which is
    /// right for a box the toolbar measures but leaves whatever right side
    /// bearing the last glyph carries as clear air. A word ending in `)`
    /// carries a lot of it: measured on `Commit changes (5 staged)`, the
    /// mark stood 6px clear of the bracket's ink where the letters inside
    /// the word sit 2px apart, so it read as a separate thing. One gap
    /// back puts it where the letters are.
    property bool alertTight: false
    /// The colour the hold fills the button with — the frame's, since a
    /// framed button fills the frame it drew. A bare one names its own
    /// (the hunk heading's `Discard hunk`, which fills edge to edge the
    /// way a held menu row does — デザイン規約 §長押し).
    property color holdTone: actionBtn.frameColor
    readonly property bool framed: actionBtn.frameColor.a > 0
    /// The label is a git command said in git's own spelling, and wears
    /// the chip that says so — lowercase, mono, on a faint ground
    /// (デザイン規約 §git 用語のコード表記). Where a menu row carries the
    /// chip ahead of a sentence, here the command is the whole label, so
    /// `text` itself is what the chip is drawn around — and that text is
    /// never translated: it is the command, not a phrase about it.
    property bool code: false
    /// The icon is a mark standing next to the word, rather than an icon
    /// at the head of a band.
    ///
    /// The step goes with the role (デザイン規約 §寸法「段は役割で選ぶ」
    /// — `iconSm` is the mark beside a word, `iconMd` the icon a row or a
    /// band starts with), and the seat comes in to the icon itself: the
    /// wider seat is there to hold the icon and the hold mark side by
    /// side, and a button that is only ever clicked pays for a pairing it
    /// cannot have. Measured on `✓ Save` before this: the word stood 10px
    /// off the icon's ink and 5px off the frame, so the pair read as two
    /// things in a box rather than as one phrase (デザイン規約 §余白
    /// 「印が自分で持っている余白は、隣の詰めに数える」).
    ///
    /// The toolbar keeps the wide seat: its buttons can pair, and their
    /// width is measured into a box two of them share.
    property bool besideWord: false
    /// Text the label's box is measured for, and whether that wording is
    /// a command (the two families measure differently, so the box has to
    /// be told which one it is holding). A button whose wording changes
    /// with its state would otherwise move everything beside it in the
    /// toolbar every time the state changed.
    property string widestText: ""
    property bool widestCode: false
    /// Icon and word centred as a pair rather than packed from the left.
    /// A toolbar button is measured to its content and never sees the
    /// difference; one told to fill a pane's width does — the icon would
    /// sit against the far edge with the word adrift from it.
    property bool centred: false
    /// What the shared box hands this state past its own wording. The box
    /// is measured for the longest thing either of the pair ever says, so
    /// every shorter wording is given air it has no use for — measured
    /// against `Resume`, that is 25px behind `push` and 17px behind
    /// `fetch` (29 and 22 on Ubuntu, where the mono family is narrower).
    ///
    /// Split between the button's two ends rather than left where it
    /// falls. Packed from the left it all lands behind the word, and what
    /// the button covers stops agreeing with what it says: the wash under
    /// the pointer, the frame a warning draws and the fill a hold sweeps
    /// across all reach a third of a button further right than the last
    /// letter (measured: 10px before the icon against 29px after the word,
    /// and 33px on Ubuntu, where the band has no window buttons after this
    /// pair to stand in the leftover).
    ///
    /// The phrase moves whole, so the step from the icon to the word is
    /// the same in every state (デザイン規約 §余白 — the mark and the word
    /// are one thing said). Centring the word inside the box instead would
    /// leave the icon where it is and open that step to 18px, which is
    /// further than the icon stands from the button beside it — the two
    /// halves would stop reading as one phrase.
    readonly property real slack:
        Math.max(0, btnLabel.box - btnLabel.implicitWidth,
                 actionBtn.floorSlack)
    /// What a framed button is short of `Theme.buttonMinWidth`, which it
    /// takes as slack like any other (so the phrase still moves whole and
    /// the ink still comes out even at the two ends).
    ///
    /// A frame is a box put round a word, and a short word draws a box the
    /// eye reads as a chip rather than as something to press. Measured
    /// across the family: `fetch` / `push` 93, `Choose image…` 118,
    /// `Choose again…` 126, `Open repository…` 132 — against `OK` at 31,
    /// the gate's `Close` at 47, `Add` at 50 and `Save` at 54. The floor
    /// is the width Fusion held the same buttons at before the face came
    /// off, so nothing on screen grows past the weight it already had —
    /// it only stops carrying depth (2026-08-11 報告).
    ///
    /// Only where a frame is drawn. A bare button is a word among words —
    /// the hunk heading's two, a dialog's `Cancel` — and a floor there
    /// would stretch the hover wash and the hold's fill well past what the
    /// button names.
    readonly property real floorSlack:
        actionBtn.framed
        ? Theme.buttonMinWidth - btnRow.implicitWidth - 2 * actionBtn.padding
        : 0
    /// The air each end is already holding before the slack is shared out
    /// (デザイン規約 §余白「印が自分で持っている余白は、隣の詰めに数える」).
    ///
    /// The seat is cut to hold an icon and the hold mark side by side, so
    /// a button wearing one of them alone keeps it in the middle of that
    /// seat with air either side of the ink; and a command's chip reaches
    /// half a gap past its last letter, which is where the eye measures
    /// the word's end. Taken off before the halves are cut, so what comes
    /// out even is the **ink** at the two ends rather than the row between
    /// them — centring the row alone leaves 22px of band before the icon
    /// against 17px after the chip (measured on `push`).
    readonly property real headInk:
        seat.visible ? (seat.implicitWidth - seat.step) / 2 : 0
    readonly property real tailInk: actionBtn.code ? Theme.spaceXs / 2 : 0
    /// Held all the way down.
    signal held()
    /// Pressed and let go, meaning the button's ordinary action.
    ///
    /// A hold button never emits this: neither the release that completes
    /// a hold nor the one that gives up on it part way may fall through
    /// to what this button does when it is not a hold button.
    signal activated()
    readonly property color fg: actionBtn.busy ? actionBtn.toneDim
                                : holdProgress > 0 ? Theme.textOnAccent
                                : enabled ? tone : Theme.textMuted
    /// Nothing here answers a press while git is on the network for it.
    readonly property bool live: actionBtn.enabled && !actionBtn.busy

    /// Automation: run the hold to its end without a press behind it.
    function completeHold() {
        if (actionBtn.holdMs > 0) {
            backAnim.stop()
            holdAnim.restart()
        }
    }

    // Tab reaches the buttons that need a second way in. The rest of the
    // toolbar stays out of the tab order: a hold is the only gesture here
    // that a pointer alone can fail to make (デザイン規約 §長押し).
    activeFocusOnTab: actionBtn.holdMs > 0
    // The mark says it to whoever can see it, and this says it to whoever
    // cannot: the words themselves no longer carry the gesture.
    Accessible.description: actionBtn.holdMs > 0 ? qsTr("Hold to activate") : ""

    onClicked: if (actionBtn.holdMs <= 0 && actionBtn.live) actionBtn.activated()
    onDownChanged: {
        if (actionBtn.holdMs <= 0 || !actionBtn.live)
            return
        if (actionBtn.down) {
            backAnim.stop()
            holdAnim.restart()
        } else {
            holdAnim.stop()
        }
    }
    // The hold's other hand: focus it, then hold Space or Enter. Accepting
    // the key keeps AbstractButton from also taking Space as a press, which
    // would drive the same fill from `down` a second time.
    //
    // Auto-repeat is dropped on both edges. A held key repeats its press on
    // every platform and its release on some, and either edge would restart
    // the fill from zero for as long as the key was held — the hold could
    // then never complete.
    Keys.onPressed: event => {
        if (actionBtn.holdMs <= 0 || !actionBtn.live
                || event.isAutoRepeat || !holdKey(event.key))
            return
        backAnim.stop()
        holdAnim.restart()
        event.accepted = true
    }
    Keys.onReleased: event => {
        if (actionBtn.holdMs <= 0 || !actionBtn.live
                || event.isAutoRepeat || !holdKey(event.key))
            return
        holdAnim.stop()
        event.accepted = true
    }
    function holdKey(key) {
        return key === Qt.Key_Space || key === Qt.Key_Return
                || key === Qt.Key_Enter
    }
    NumberAnimation {
        id: holdAnim
        target: actionBtn
        property: "holdProgress"
        from: 0
        to: 1
        duration: Math.max(actionBtn.holdMs, 1)
        // A press that stopped short slides back out instead of blanking:
        // a hold button reports no click at all, so without this the only
        // answer to a plain click is nothing happening (デザイン規約
        // §長押し). Pressed all the way through, it has already fired and
        // there is nothing left to say — that one blanks.
        onStopped: {
            if (actionBtn.holdProgress >= 1)
                actionBtn.holdProgress = 0
            else if (actionBtn.holdProgress > 0)
                backAnim.restart()
        }
        onFinished: actionBtn.held()
    }
    NumberAnimation {
        id: backAnim
        target: actionBtn
        property: "holdProgress"
        to: 0
        duration: Metrics.holdBackMs
        easing.type: Easing.OutCubic
    }
    background: Rectangle {
        // The same wash every other tool button answers with
        // (`HoverToolButton.washColor`), read rather than left out: a
        // background handed in replaces the one that carries it, and the
        // frame, the hold's fill and the focus ring all have to be drawn
        // here — so this button owes the pointer the paint as well.
        //
        // Only while it is answering, though. A button with git out on the
        // network is as deaf as a disabled one (`live`), and it was
        // lighting under a pointer that had nothing to press: the hand
        // that starts a fetch is still resting on the button while the
        // fetch runs, so the wash stayed up for the whole of it and came
        // back on every fetch the timer made (2026-08-10 報告). The ring
        // is all this state has to say.
        color: actionBtn.live ? actionBtn.washColor : "transparent"
        // The frame goes a step down with the rest of the button while git
        // is out on the network: there is no press to be had until this
        // comes back, but the button is still the one that overwrites a
        // remote, and a frame that dropped to grey would take that back
        // for as long as the wait lasted.
        //
        // A button with no frame of its own grows none. It used to borrow
        // the plain one, on the reasoning that with the words gone there
        // was nothing left to say the wait had started — but the ring is
        // turning in the middle of it, which says exactly that, and a
        // frame drawn around a button that has never worn one reads as a
        // box laid over the band rather than as part of it
        // (2026-08-10 報告「ちょっと浮いて見えた」).
        border.color: actionBtn.busy && actionBtn.framed
                      ? actionBtn.toneDim : actionBtn.frameColor
        border.width: Theme.borderWidth
        radius: Theme.radiusSm
        // The hold, filling from the left. Inset by the border where
        // there is one, so the frame stays a frame while it fills; a bare
        // button fills edge to edge, the way a held menu row does.
        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.margins: actionBtn.framed ? Theme.borderWidth : 0
            radius: Theme.radiusSm
            // Never thinner than `holdFillMin` while it runs: proportional
            // from zero, the first tenth of the hold is a sub-pixel sliver,
            // so the press reads as not having taken and the whole gesture
            // feels longer than it is (デザイン規約 §進行中・長押しの定数).
            width: actionBtn.holdProgress > 0
                   ? Math.max(Metrics.holdFillMin,
                              (parent.width - 2 * anchors.margins)
                              * actionBtn.holdProgress)
                   : 0
            color: actionBtn.holdTone
            visible: actionBtn.holdProgress > 0
        }
        // Drawn outside the frame rather than in it: the frame's colour is
        // already saying this button is the dangerous one, and focus must
        // not be able to take that over.
        //
        // `visualFocus`, which is focus that arrived from the keyboard —
        // not `activeFocus`, which a press gives it as well. A button
        // takes focus when it is clicked (`focusPolicy` is StrongFocus and
        // nothing on this band takes it back), so the ring came up on the
        // press and then stayed, on a button whose reader had long since
        // moved on (2026-08-10 報告「押したら色が解除されなくなった」).
        // The ring is for whoever cannot see the pointer; it has nothing
        // to tell the hand that is holding one. Same reading as the wash
        // (`HoverToolButton.washColor`).
        Rectangle {
            anchors.fill: parent
            anchors.margins: -Theme.spaceXs / 2
            color: "transparent"
            border.color: Theme.borderFocus
            border.width: Theme.borderWidth
            radius: Theme.radiusMd
            visible: actionBtn.visualFocus
        }
    }
    // The toolbar's size unless an instance says otherwise — the hunk
    // heading's words take fontSm there, a step under the file's own
    // word (デザイン規約 §diff の中のステージ).
    font.pixelSize: Theme.fontMd

    // The slack goes into the button's own air, so what one end takes the
    // other gives: the width stays the one the pair was measured for and
    // the toolbar's right-hand end does not move (デザイン規約
    // §リモートへ送る — 全状態が同じ幅).
    //
    // The trailing side is what is left rather than the other half
    // rounded: a wording measures in fractions of a pixel, and `floor`
    // beside `ceil` would round the pair of them up to the next whole one
    // — a button a pixel wider than the box it was measured into, for no
    // reason anybody reading the two expressions would see. It is also the
    // side that ends up with the odd pixel, which is the side with a mark
    // to stand clear of (`alert`).
    // Only a button with slack to share reads the ink off its two ends —
    // one measured into a shared box (`widestText`) or one held open by
    // the frame's floor (`floorSlack`). A button sized to its own content
    // is already as tight as the two ends can be, and taking the ink off
    // its padding would move every button in the app for the sake of the
    // few that have room to give.
    readonly property real headAir:
        (btnLabel.box > 0 || actionBtn.floorSlack > 0)
        ? Math.floor((actionBtn.slack - actionBtn.headInk
                      - actionBtn.tailInk) / 2)
        : 0
    leftPadding: actionBtn.padding + actionBtn.headAir
    rightPadding: actionBtn.padding + (actionBtn.slack - actionBtn.headAir)

    // The row keeps its size while the network call runs — the toolbar
    // must not shuffle under a pointer that is still resting on the
    // button — so the words step aside by going transparent rather than
    // by leaving the layout, and the ring turns over the middle of what
    // they left.
    contentItem: Item {
        implicitWidth: btnRow.implicitWidth
        implicitHeight: btnRow.implicitHeight

        RowLayout {
            id: btnRow
            anchors.fill: parent
            spacing: Theme.spaceXs
            opacity: actionBtn.busy ? 0 : 1
            // An invisible child is left out of the layout entirely, so
            // these two cost nothing where they are not asked for.
            Item {
                Layout.fillWidth: true
                visible: actionBtn.centred
            }
            // A button with no icon to name it spends no width on one —
            // the word is the whole of it (the hunk header's buttons).
            // A button that swaps between a named icon and the mark keeps
            // the wider of the two seats whichever it is wearing, so its
            // width does not change with its state and the toolbar does
            // not slide under a pointer already resting on it. One that
            // only ever wears the mark fits it, and its words sit as
            // close to it as a menu row's do.
            Item {
                id: seat
                /// Both marks to wear at once: what the button does, and
                /// that it is held rather than clicked.
                readonly property bool paired: actionBtn.kind !== ""
                                               && actionBtn.holdMs > 0
                /// How far the two are set apart across the slash.
                readonly property int spread: Theme.iconMd - Theme.spaceXs
                /// The step this button's icon is drawn at (see
                /// `besideWord`). A button that can pair keeps the band's
                /// step whatever it is asked for: the fraction is built
                /// out of two `iconSm` halves and has no room to give.
                readonly property int step: actionBtn.besideWord
                                            && actionBtn.holdMs <= 0
                                            ? Theme.iconSm : Theme.iconMd
                /// The air the mark keeps inside its own box, which the
                /// seat gives back so that the padding on one side and
                /// the row's spacing on the other land on the **ink**
                /// (デザイン規約 §余白「印が自分で持っている余白は、隣の
                /// 詰めに数える」). The icon overflows the seat by this
                /// much either side, which is what the tokens were going
                /// to leave empty anyway. Measured before it: the frame
                /// stood 7–8px off the icon's ink where the word ended
                /// 5–6px off the other side, so a button that is one
                /// phrase read as pushed to the right.
                ///
                /// One number for the family, from the widest of these
                /// marks: 8–10px of ink inside the 12px box.
                readonly property int markAir: seat.step === Theme.iconSm
                                               ? Theme.spaceXs / 2 : 0

                visible: actionBtn.kind !== "" || actionBtn.holdMs > 0
                implicitWidth: !visible ? 0
                               : actionBtn.kind !== ""
                                 ? (seat.step === Theme.iconSm
                                    ? Theme.iconSm - 2 * seat.markAir
                                    : Math.max(Theme.iconMd,
                                               Theme.iconSm + seat.spread))
                                 : Theme.iconSm
                implicitHeight: seat.step + Theme.spaceXs
                Layout.alignment: Qt.AlignVCenter
                NavIcon {
                    width: seat.step
                    height: seat.step
                    anchors.centerIn: parent
                    kind: actionBtn.kind
                    tint: actionBtn.fg
                    // Whole at `iconMd`, and the grid ratio under it.
                    stroke: Metrics.iconStroke * seat.step / 16
                    visible: actionBtn.holdMs <= 0
                }
                // A held button with nothing to name it says only how it
                // is worked, where the eye starts the row (デザイン規約
                // §長押し).
                HoldIcon {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.verticalCenterOffset: Metrics.opticalDrop
                    progress: actionBtn.holdProgress
                    tint: actionBtn.fg
                    visible: actionBtn.holdMs > 0 && !seat.paired
                }
                // A held button that also has a name wears both, set as a
                // fraction: each a size down, the slash between them, each
                // pushed off the middle line. Letting the hold mark take
                // the seat on its own would cost the button the one thing
                // that says what it does — `push -f` would stop being a
                // push at a glance (デザイン規約 §長押し).
                Item {
                    anchors.fill: parent
                    visible: seat.paired
                    NavIcon {
                        kind: actionBtn.kind
                        tint: actionBtn.fg
                        width: Theme.iconSm
                        height: Theme.iconSm
                        x: 0
                        y: 0
                    }
                    Label {
                        text: "/"
                        color: actionBtn.fg
                        font.pixelSize: Theme.fontMd
                        anchors.centerIn: parent
                    }
                    HoldIcon {
                        progress: actionBtn.holdProgress
                        tint: actionBtn.fg
                        width: Theme.iconSm
                        height: Theme.iconSm
                        x: seat.spread
                        y: parent.height - Theme.iconSm
                    }
                }
            }
            // Measured, never drawn: a hidden item is left out of the
            // layout, and a Label measures the way the visible one does —
            // TextMetrics reports a few pixels tighter, which is enough of
            // a difference to shift the toolbar it is here to hold still.
            Label {
                id: widest
                visible: false
                text: actionBtn.widestText
                font.family: actionBtn.widestCode ? Theme.monoFamily
                                                  : Theme.uiFamily
                font.wordSpacing: actionBtn.widestCode ? -Theme.spaceXs : 0
                font.pixelSize: actionBtn.font.pixelSize
            }
            Item {
                id: btnLabel
                // The box is the widest wording's ink and nothing else.
                // What stands between that ink and the frame is the
                // button's own air, shared out by one rule in every state
                // (`slack`) — the widest included, whose slack is nothing
                // and whose air is therefore the padding itself.
                //
                // The gap this used to add for a wording that is not a
                // command has gone with it. It was there to keep the last
                // letter off the border, which the padding now does on both
                // sides at once; and charging it to one family and not the
                // other made the box jump five pixels whenever the two came
                // within one of each other. That is what happened: the UI
                // family changed under it and `Resume` (53) passed
                // `push -f` (52), which took the pair from 92px to 97px
                // with nothing on screen, and nothing in either table, to
                // say what had grown.
                //
                // Measured from the font even where the flag is drawn (see
                // below) — the box is what holds the toolbar still, and it
                // must not move when a shorter rule is chosen for the flag.
                readonly property real box: widest.implicitWidth
                /// A command's flag, set apart from the command itself so
                /// its dashes can be drawn rather than typed. Every dash
                /// the mono family carries is the same 7px rule in an 8px
                /// cell (measured over U+002D / 2010 / 2011 / 2212), and
                /// on the wording the shared box was measured for that is
                /// what leaves the mark no room past the word. Drawn, the
                /// rule's length and the air either side are ours to pick
                /// (デザイン規約 §git 用語のコード表記).
                readonly property int flagAt: actionBtn.code
                                              ? actionBtn.text.indexOf(" -") : -1
                readonly property bool splitFlag: btnLabel.flagAt > 0
                readonly property string head: btnLabel.splitFlag
                    ? actionBtn.text.substring(0, btnLabel.flagAt) : actionBtn.text
                /// The flag with its leading dashes taken off, and how
                /// many of them there were.
                readonly property string flagRest: btnLabel.splitFlag
                    ? actionBtn.text.substring(btnLabel.flagAt + 1).replace(/^-+/, "") : ""
                readonly property int dashCount: btnLabel.splitFlag
                    ? actionBtn.text.substring(btnLabel.flagAt + 1).length
                      - btnLabel.flagRest.length : 0

                implicitWidth: headText.width
                               + (btnLabel.splitFlag ? flagRow.width : 0)
                implicitHeight: headText.implicitHeight
                Layout.maximumWidth: 240
                // Its own width. What the shared box asks for past this
                // wording is held by the button's padding (`slack`), so
                // the cell no longer carries the whole of it behind the
                // last letter — and the chip and the mark, both measured
                // off this cell, keep sitting on the word.
                Layout.preferredWidth: btnLabel.implicitWidth
                Layout.alignment: Qt.AlignVCenter

                // The command, or the whole wording where there is no flag
                // to take off it.
                Label {
                    id: headText
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    // Bounded by the cell's own ceiling rather than by its
                    // width: the width comes from this, so reading it back
                    // would close a loop.
                    width: Math.min(implicitWidth, btnLabel.Layout.maximumWidth)
                    text: btnLabel.head
                    color: actionBtn.fg
                    font.family: actionBtn.code ? Theme.monoFamily
                                                : Theme.uiFamily
                    // A command and its flag are one thing said, and a mono
                    // space is far wider than the air the chip keeps at its
                    // own ends — left alone, `-f` drifts away from the
                    // `push` it belongs to and the chip reads as two words
                    // on one ground (デザイン規約 §git 用語のコード表記).
                    font.wordSpacing: actionBtn.code ? -Theme.spaceXs : 0
                    font.pixelSize: actionBtn.font.pixelSize
                    elide: Text.ElideRight
                }
                // The flag: the air the mono space held, a drawn rule for
                // each dash, and the letters after them in the font. The
                // rule is `borderWidth` thick because that is what the
                // font's own dash measures at this size, and a hair of air
                // follows it so the letter does not touch.
                Row {
                    id: flagRow
                    visible: btnLabel.splitFlag
                    anchors.left: headText.right
                    anchors.verticalCenter: headText.verticalCenter
                    spacing: 0
                    Item {
                        width: Theme.spaceXs
                        height: headText.height
                    }
                    Repeater {
                        model: btnLabel.dashCount
                        Item {
                            width: Theme.spaceXs + Theme.borderWidth
                            height: headText.height
                            Rectangle {
                                anchors.left: parent.left
                                anchors.verticalCenter: parent.verticalCenter
                                width: Theme.spaceXs
                                height: Theme.borderWidth
                                color: actionBtn.fg
                            }
                        }
                    }
                    Label {
                        text: btnLabel.flagRest
                        color: actionBtn.fg
                        font.family: Theme.monoFamily
                        font.pixelSize: actionBtn.font.pixelSize
                    }
                }
                // The chip a command wears, behind the glyphs and only as
                // wide as they are — the box around them is measured for
                // the longest wording of the pair, and a ground stretched
                // to that would draw a chip the word does not fill. Half a
                // gap of tint hangs off either end, the same as a menu
                // row's (デザイン規約 §git 用語のコード表記).
                Rectangle {
                    z: -1
                    visible: actionBtn.code
                    x: -Theme.spaceXs / 2
                    width: btnLabel.implicitWidth + Theme.spaceXs
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    radius: Theme.radiusSm
                    color: Theme.bgHover
                }
                // Past the word's end rather than over its shoulder: the
                // last letter has to stay readable, and the box is
                // measured for the longest wording so there is room after
                // the shorter ones. Past the chip's edge where there is
                // one — a mark crossing that edge reads as stuck to the
                // chip rather than said after the word.
                //
                // Set close to it, though: the air a chip's ground keeps
                // at its own end is already enough to tell the two apart,
                // and further out the mark starts to read as the toolbar's
                // rather than this button's. Closer still after a flag —
                // that is the longest thing the button says and the one
                // wording whose right-hand side is short of room
                // (デザイン規約 §リモートへ送る).
                NavIcon {
                    visible: actionBtn.alert
                    kind: "bang"
                    tint: actionBtn.alertTone
                    width: Theme.iconSm
                    height: Theme.iconSm
                    x: btnLabel.implicitWidth
                       - (btnLabel.splitFlag ? Theme.spaceXs / 2 : 0)
                       - (actionBtn.alertTight ? Theme.spaceXs : 0)
                    y: -Theme.spaceXs
                }
            }
            Item {
                Layout.fillWidth: true
                visible: actionBtn.centred
            }
        }
        // Its own item rather than a rotation on the icon above: an
        // animator leaves the angle where it stopped, and the icon that
        // returns must not come back tilted.
        NavIcon {
            anchors.centerIn: parent
            width: Theme.iconMd
            height: Theme.iconMd
            kind: "spinner"
            tint: actionBtn.toneDim
            visible: actionBtn.busy
            // On the render thread, so it keeps turning while the GUI
            // thread drains models.
            RotationAnimator on rotation {
                running: actionBtn.busy && !actionBtn.still
                loops: Animation.Infinite
                from: 0
                to: 360
                duration: Metrics.spinMs
            }
        }
    }
}
